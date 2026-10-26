//! Recover and audit every localised Car Collection description table.

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;

use crate::rt2::{self, Rt2File};

pub type TextKey = (String, String);

const LANGUAGE_FILES: &[(&str, &[&str])] = &[
    ("ar", &["M_1_PHS6I"]),
    ("cn", &["P_9_PS69N", "0_E_P4DE3"]),
    ("cz", &["D_X_6XPTC"]),
    ("de", &["9_F_6U6VC"]),
    ("es", &["5_2_6QEGX", "S_Z_6UX1V"]),
    ("fr", &["3_0_6T88H"]),
    ("gb", &["D_Y_6SI87"]),
    ("it", &["8_C_PAS6G"]),
    ("jp", &["W_D_PAQ3K"]),
    ("kr", &["U_2_6MC5B"]),
    ("nl", &["E_I_PQDHU"]),
    ("pl", &["K_4_6AM96"]),
    ("pt", &["Y_T_PYSY9", "T_P_6M4RW"]),
    ("ru", &["P_Z_PN2WD"]),
    ("th", &["6_2_PHN2O"]),
    ("tr", &["1_6_6NUU3"]),
    ("tw", &["4_Y_6FP1O"]),
    ("us", &["L_O_PWIJL"]),
    ("zh", &["0_E_P4DE3", "P_9_PS69N"]),
];

#[derive(Clone)]
pub struct LanguageTable {
    pub code: String,
    pub source: String,
    pub catch: HashMap<String, String>,
    pub long: HashMap<String, String>,
}

#[derive(Clone)]
struct Candidate {
    source: String,
    entries: HashMap<TextKey, String>,
    catch: usize,
    long: usize,
    internal_replacements: usize,
    bad_entries: usize,
    controls: usize,
    chars: usize,
    entry_replacements: HashMap<TextKey, usize>,
    entry_controls: HashMap<TextKey, usize>,
}

impl Candidate {
    fn score(&self) -> usize {
        self.bad_entries * 1_000_000
            + self.internal_replacements * 100
            + self.controls
            + usize::from(self.catch != 580 || self.long != 580) * 1_000_000_000
    }
}

fn load_json(path: &Path) -> Result<Value> {
    serde_json::from_slice(&fs::read(path).with_context(|| path.display().to_string())?)
        .with_context(|| path.display().to_string())
}

fn corpus_models(root: &Path, code: &str) -> Result<([u64; 256], [u64; 256])> {
    let path = root.join(format!("extracted/text_decoded/{code}.json"));
    let corpus = load_json(&path)?;
    let mut counts = [0u64; 256];
    let mut first = [0u64; 256];
    for section in corpus
        .as_object()
        .into_iter()
        .flat_map(|values| values.values())
    {
        for text in section
            .as_object()
            .into_iter()
            .flat_map(|values| values.values())
            .filter_map(Value::as_str)
        {
            let bytes = text.as_bytes();
            for byte in bytes {
                counts[*byte as usize] += 1;
            }
            if let Some(byte) = bytes.first() {
                first[*byte as usize] += 1;
            }
        }
    }
    Ok((counts, first))
}

fn clean_text(text: &str) -> String {
    text.chars()
        .filter(|ch| {
            *ch != '\u{FFFD}'
                && !('\u{E000}'..='\u{F8FF}').contains(ch)
                && (!ch.is_control() || matches!(ch, '\n' | '\r' | '\t'))
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn decode_candidate(root: &Path, code: &str, source: &str) -> Result<Candidate> {
    let raw_path = root.join(format!("extracted/descriptions_raw/{source}.rt2"));
    let table = Rt2File::parse(&raw_path)?;
    let payloads = table.payloads();
    let max_len = table.max_payload_len();
    let (counts, first) = corpus_models(root, code)?;
    let key = rt2::recover_keystream_with_counts(&payloads, max_len, &counts, &first);
    let decoded = rt2::decode(&table, &key);
    let mut entries = HashMap::new();
    let mut internal_replacements = 0;
    let mut bad_entries = 0;
    let mut controls = 0;
    let mut chars = 0;
    let mut entry_replacements = HashMap::new();
    let mut entry_controls = HashMap::new();
    for entry in decoded {
        if !matches!(
            entry.section.as_str(),
            "DescriptionCatch" | "DescriptionLong"
        ) {
            continue;
        }
        let values = entry.text.chars().collect::<Vec<_>>();
        let last = values
            .iter()
            .rposition(|value| *value != '\u{FFFD}')
            .unwrap_or(0);
        let replacements = values
            .iter()
            .enumerate()
            .filter(|(index, value)| **value == '\u{FFFD}' && *index < last)
            .count();
        internal_replacements += replacements;
        bad_entries += usize::from(replacements > 0);
        let entry_control_count = values
            .iter()
            .filter(|value| value.is_control() && !matches!(**value, '\n' | '\r' | '\t'))
            .count();
        controls += entry_control_count;
        if entry.section == "DescriptionLong" {
            chars += values.len();
        }
        let key = (entry.section, format!("0x{:08X}", entry.id));
        entry_replacements.insert(key.clone(), replacements);
        entry_controls.insert(key.clone(), entry_control_count);
        entries.insert(key, clean_text(&entry.text));
    }
    let catch = entries
        .keys()
        .filter(|(section, _)| section == "DescriptionCatch")
        .count();
    let long = entries
        .keys()
        .filter(|(section, _)| section == "DescriptionLong")
        .count();
    Ok(Candidate {
        source: source.to_string(),
        entries,
        catch,
        long,
        internal_replacements,
        bad_entries,
        controls,
        chars,
        entry_replacements,
        entry_controls,
    })
}

fn combine(mut candidates: Vec<Candidate>) -> Candidate {
    let mut combined = candidates.remove(0);
    let mut sources = vec![combined.source.clone()];
    for alternative in candidates {
        sources.push(alternative.source.clone());
        for (key, text) in alternative.entries {
            let current = combined
                .entry_replacements
                .get(&key)
                .copied()
                .unwrap_or(usize::MAX);
            let replacement = alternative
                .entry_replacements
                .get(&key)
                .copied()
                .unwrap_or(usize::MAX);
            if replacement < current {
                combined.entries.insert(key.clone(), text);
                combined.entry_replacements.insert(key.clone(), replacement);
                combined.entry_controls.insert(
                    key.clone(),
                    alternative.entry_controls.get(&key).copied().unwrap_or(0),
                );
            }
        }
    }
    combined.source = sources.join("+");
    combined.catch = combined
        .entries
        .keys()
        .filter(|(section, _)| section == "DescriptionCatch")
        .count();
    combined.long = combined
        .entries
        .keys()
        .filter(|(section, _)| section == "DescriptionLong")
        .count();
    combined.internal_replacements = combined.entry_replacements.values().sum();
    combined.bad_entries = combined
        .entry_replacements
        .values()
        .filter(|count| **count > 0)
        .count();
    combined.controls = combined.entry_controls.values().sum();
    combined.chars = combined
        .entries
        .iter()
        .filter(|((section, _), _)| section == "DescriptionLong")
        .map(|(_, text)| text.chars().count())
        .sum();
    combined
}

fn as_section(entries: &HashMap<TextKey, String>, section: &str) -> HashMap<String, String> {
    entries
        .iter()
        .filter(|((candidate_section, _), _)| candidate_section == section)
        .map(|((_, id), text)| (id.clone(), text.clone()))
        .collect()
}

pub fn build(root: &Path) -> Result<(BTreeMap<String, LanguageTable>, Value)> {
    let mut tables = BTreeMap::new();
    let mut audits = Vec::new();
    let mut residuals = Vec::new();
    for (code, files) in LANGUAGE_FILES {
        let mut candidates = files
            .iter()
            .map(|source| decode_candidate(root, code, source))
            .collect::<Result<Vec<_>>>()?;
        candidates.sort_by_key(Candidate::score);
        let candidate = combine(candidates);
        for ((section, text_id), count) in &candidate.entry_replacements {
            if *count == 0 {
                continue;
            }
            residuals.push(json!({
                "code": code,
                "source": candidate.source,
                "section": section,
                "textId": text_id,
                "replacements": count,
                "excerpt": candidate
                    .entries
                    .get(&(section.clone(), text_id.clone()))
                    .map(|text| text.chars().take(160).collect::<String>())
                    .unwrap_or_default(),
            }));
        }
        tables.insert(
            (*code).to_string(),
            LanguageTable {
                code: (*code).to_string(),
                source: candidate.source.clone(),
                catch: as_section(&candidate.entries, "DescriptionCatch"),
                long: as_section(&candidate.entries, "DescriptionLong"),
            },
        );
        audits.push(json!({
            "code": code,
            "source": candidate.source,
            "alternates": files.get(1..).unwrap_or(&[]),
            "catchEntries": candidate.catch,
            "longEntries": candidate.long,
            "internalReplacements": candidate.internal_replacements,
            "affectedEntries": candidate.bad_entries,
            "controlCharacters": candidate.controls,
            "longCharacters": candidate.chars,
            "status": if candidate.catch == 580 && candidate.long == 580 && candidate.internal_replacements == 0 {
                "ok"
            } else if candidate.catch == 580 && candidate.long == 580 {
                "recovered-with-residual-glyphs"
            } else {
                "incomplete"
            }
        }));
    }

    let reference = tables
        .get("us")
        .context("English description table missing")?;
    let reference_catch = reference.catch.keys().cloned().collect::<HashSet<_>>();
    let reference_long = reference.long.keys().cloned().collect::<HashSet<_>>();
    for audit in &mut audits {
        let code = audit.get("code").and_then(Value::as_str).unwrap_or("");
        let Some(table) = tables.get(code) else {
            continue;
        };
        let missing_catch = reference_catch
            .difference(&table.catch.keys().cloned().collect())
            .count();
        let missing_long = reference_long
            .difference(&table.long.keys().cloned().collect())
            .count();
        audit["missingCatchIds"] = json!(missing_catch);
        audit["missingLongIds"] = json!(missing_long);
        if missing_catch != 0 || missing_long != 0 {
            audit["status"] = json!("id-mismatch");
        }
    }

    let report = json!({
        "generatedFrom": "PS5 RT2 DescriptionCatch/DescriptionLong with language-aware keystream recovery",
        "supportedLanguages": audits.len(),
        "referenceLanguage": "us",
        "residualCount": residuals.len(),
        "residuals": residuals,
        "languages": audits,
    });
    Ok((tables, report))
}

pub fn write_residual_csv(path: &Path, report: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut writer = csv::Writer::from_path(path)?;
    writer.write_record([
        "code",
        "source",
        "section",
        "textId",
        "replacements",
        "excerpt",
    ])?;
    for row in report
        .get("residuals")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        writer.write_record([
            row.get("code").and_then(Value::as_str).unwrap_or(""),
            row.get("source").and_then(Value::as_str).unwrap_or(""),
            row.get("section").and_then(Value::as_str).unwrap_or(""),
            row.get("textId").and_then(Value::as_str).unwrap_or(""),
            &row.get("replacements")
                .map(Value::to_string)
                .unwrap_or_default(),
            row.get("excerpt").and_then(Value::as_str).unwrap_or(""),
        ])?;
    }
    writer.flush()?;
    Ok(())
}

pub fn write_language_files(
    root: &Path,
    output: &Path,
    tables: &BTreeMap<String, LanguageTable>,
) -> Result<Vec<String>> {
    fs::create_dir_all(output)?;
    let mut written = Vec::new();
    for (code, table) in tables {
        let value = json!({
            "code": table.code,
            "sourceFile": table.source,
            "DescriptionCatch": table.catch,
            "DescriptionLong": table.long,
        });
        let path = output.join(format!("{code}.json"));
        fs::write(&path, serde_json::to_vec(&value)?)?;
        let report_path = root.join(format!("reports/vehicle_collection_i18n/{code}.json"));
        if let Some(parent) = report_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(report_path, serde_json::to_vec_pretty(&value)?)?;
        written.push(code.clone());
    }
    Ok(written)
}

pub fn write_audit_csv(path: &Path, report: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut writer = csv::Writer::from_path(path)?;
    writer.write_record([
        "code",
        "source",
        "catchEntries",
        "longEntries",
        "missingCatchIds",
        "missingLongIds",
        "internalReplacements",
        "affectedEntries",
        "controlCharacters",
        "longCharacters",
        "status",
    ])?;
    for row in report
        .get("languages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        writer.write_record([
            row.get("code").and_then(Value::as_str).unwrap_or(""),
            row.get("source").and_then(Value::as_str).unwrap_or(""),
            &row.get("catchEntries")
                .map(Value::to_string)
                .unwrap_or_default(),
            &row.get("longEntries")
                .map(Value::to_string)
                .unwrap_or_default(),
            &row.get("missingCatchIds")
                .map(Value::to_string)
                .unwrap_or_default(),
            &row.get("missingLongIds")
                .map(Value::to_string)
                .unwrap_or_default(),
            &row.get("internalReplacements")
                .map(Value::to_string)
                .unwrap_or_default(),
            &row.get("affectedEntries")
                .map(Value::to_string)
                .unwrap_or_default(),
            &row.get("controlCharacters")
                .map(Value::to_string)
                .unwrap_or_default(),
            &row.get("longCharacters")
                .map(Value::to_string)
                .unwrap_or_default(),
            row.get("status").and_then(Value::as_str).unwrap_or(""),
        ])?;
    }
    writer.flush()?;
    Ok(())
}
