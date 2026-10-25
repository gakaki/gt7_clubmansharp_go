//! Reconstruct the full Simplified Chinese Car Collection copy from the
//! game's two Chinese description tables.

use anyhow::{Context, Result};
use rayon::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::rt2::{self, Rt2File};

pub type TextKey = (String, String);

fn load_json(path: &Path) -> Result<Value> {
    serde_json::from_slice(&fs::read(path).with_context(|| path.display().to_string())?)
        .with_context(|| path.display().to_string())
}

fn decode_table(path: &Path, key_path: &Path) -> Result<HashMap<TextKey, String>> {
    let table = Rt2File::parse(path)?;
    let key = fs::read(key_path).with_context(|| key_path.display().to_string())?;
    Ok(rt2::decode(&table, &key)
        .into_iter()
        .filter(|entry| !entry.text.is_empty())
        .map(|entry| ((entry.section, format!("0x{:08X}", entry.id)), entry.text))
        .collect())
}

#[cfg(windows)]
fn to_simplified(input: &str) -> String {
    use std::ffi::{c_void, OsStr, OsString};
    use std::os::windows::ffi::{OsStrExt, OsStringExt};

    const LCMAP_SIMPLIFIED_CHINESE: u32 = 0x0200_0000;
    #[link(name = "kernel32")]
    extern "system" {
        fn LCMapStringEx(
            locale: *const u16,
            flags: u32,
            source: *const u16,
            source_len: i32,
            destination: *mut u16,
            destination_len: i32,
            version: *const c_void,
            reserved: *const c_void,
            sort_handle: isize,
        ) -> i32;
    }

    let source = OsStr::new(input)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut destination = vec![0u16; source.len() + 16];
    let length = unsafe {
        LCMapStringEx(
            std::ptr::null(),
            LCMAP_SIMPLIFIED_CHINESE,
            source.as_ptr(),
            -1,
            destination.as_mut_ptr(),
            destination.len() as i32,
            std::ptr::null(),
            std::ptr::null(),
            0,
        )
    };
    if length <= 0 {
        return input.to_string();
    }
    let end = destination
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(length as usize);
    OsString::from_wide(&destination[..end])
        .to_string_lossy()
        .into_owned()
}

#[cfg(not(windows))]
fn to_simplified(input: &str) -> String {
    input.to_string()
}

fn align(left: &[char], right: &[char]) -> Vec<(Option<usize>, Option<usize>)> {
    let rows = left.len() + 1;
    let cols = right.len() + 1;
    let mut cost = vec![vec![0u32; cols]; rows];
    let mut direction = vec![vec![0u8; cols]; rows];
    for i in 1..rows {
        cost[i][0] = i as u32;
        direction[i][0] = 1;
    }
    for j in 1..cols {
        cost[0][j] = j as u32;
        direction[0][j] = 2;
    }
    for i in 1..rows {
        for j in 1..cols {
            let diagonal =
                cost[i - 1][j - 1] + u32::from(left[i - 1] != right[j - 1]);
            let delete = cost[i - 1][j] + 1;
            let insert = cost[i][j - 1] + 1;
            if diagonal <= delete && diagonal <= insert {
                cost[i][j] = diagonal;
                direction[i][j] = 0;
            } else if delete <= insert {
                cost[i][j] = delete;
                direction[i][j] = 1;
            } else {
                cost[i][j] = insert;
                direction[i][j] = 2;
            }
        }
    }

    let mut i = left.len();
    let mut j = right.len();
    let mut result = Vec::with_capacity(left.len().max(right.len()));
    while i > 0 || j > 0 {
        match direction[i][j] {
            0 => {
                result.push((Some(i - 1), Some(j - 1)));
                i -= 1;
                j -= 1;
            }
            1 => {
                result.push((Some(i - 1), None));
                i -= 1;
            }
            _ => {
                result.push((None, Some(j - 1)));
                j -= 1;
            }
        }
    }
    result.reverse();
    result
}

fn merge(
    base: &str,
    alternatives: &[String],
    frequency: &HashMap<char, u64>,
    bigrams: &HashMap<(char, char), u64>,
) -> String {
    let base_chars = base.chars().collect::<Vec<_>>();
    let mut candidates = base_chars
        .iter()
        .map(|value| vec![*value])
        .collect::<Vec<_>>();
    for alternative in alternatives {
        let chars = alternative.chars().collect::<Vec<_>>();
        for (left, right) in align(&base_chars, &chars) {
            if let (Some(left), Some(right)) = (left, right) {
                candidates[left].push(chars[right]);
            }
        }
    }

    let choices = candidates
        .iter()
        .enumerate()
        .map(|(index, values)| {
            let base = base_chars[index];
            let mut values = values.clone();
            values.sort_unstable();
            values.dedup();
            if base.is_ascii() {
                values.retain(|value| value.is_ascii() || *value == '\u{FFFD}');
            }
            if !values.contains(&base) {
                values.push(base);
            }
            values
        })
        .collect::<Vec<_>>();

    let mut beam = vec![(0f64, '\n', String::new())];
    let vocabulary = frequency.len().max(1) as f64;
    for (index, options) in choices.iter().enumerate() {
        let base = base_chars[index];
        let mut next = Vec::<(f64, char, String)>::new();
        for (score, previous, text) in &beam {
            for option in options {
                let count = bigrams
                    .get(&(*previous, *option))
                    .copied()
                    .unwrap_or(0) as f64;
                let previous_count = frequency.get(previous).copied().unwrap_or(0) as f64;
                let mut next_score =
                    score + ((count + 0.1) / (previous_count + vocabulary)).ln();
                if *option == base && base != '\u{FFFD}' {
                    next_score += 1.5;
                }
                let agreement = candidates[index]
                    .iter()
                    .filter(|value| *value == option)
                    .count()
                    .saturating_sub(1);
                next_score += agreement as f64 * 0.25;
                let mut next_text = text.clone();
                next_text.push(*option);
                next.push((next_score, *option, next_text));
            }
        }
        next.sort_by(|left, right| right.0.total_cmp(&left.0));
        next.truncate(32);
        beam = next;
    }
    beam.into_iter()
        .next()
        .map(|(_, _, text)| text)
        .unwrap_or_else(|| base.to_string())
}

fn sanitize_scripts(text: &str) -> String {
    text.chars()
        .filter(|ch| {
            ch.is_ascii()
                || ('\u{3400}'..='\u{4DBF}').contains(ch)
                || ('\u{4E00}'..='\u{9FFF}').contains(ch)
                || ('\u{2000}'..='\u{206F}').contains(ch)
                || ('\u{3000}'..='\u{303F}').contains(ch)
                || ('\u{FF00}'..='\u{FFEF}').contains(ch)
        })
        .collect()
}

fn load_models(root: &Path) -> Result<(HashMap<char, u64>, HashMap<(char, char), u64>)> {
    let corpus = load_json(&root.join("extracted/text_decoded/cn.json"))?;
    let mut frequency = HashMap::new();
    let mut bigrams = HashMap::new();
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
            let chars = text.chars().collect::<Vec<_>>();
            for ch in &chars {
                *frequency.entry(*ch).or_default() += 1;
            }
            for pair in chars.windows(2) {
                *bigrams.entry((pair[0], pair[1])).or_default() += 1;
            }
        }
    }
    Ok((frequency, bigrams))
}

pub fn load(root: &Path) -> Result<HashMap<TextKey, String>> {
    let raw = root.join("extracted/descriptions_raw");
    let description_keys = root.join("keys/rt2_description");
    let shared_cn_key = root.join("keys/rt2_rust/cn.ks.bin");
    if !raw.is_dir() || !shared_cn_key.is_file() {
        return Ok(HashMap::new());
    }

    let primary = "P_9_PS69N";
    let secondary = "0_E_P4DE3";
    let primary_default = decode_table(
        &raw.join(format!("{primary}.rt2")),
        &description_keys.join(format!("{primary}.rt2.ks.bin")),
    )?;
    let primary_cn = decode_table(&raw.join(format!("{primary}.rt2")), &shared_cn_key)?;
    let secondary_default = decode_table(
        &raw.join(format!("{secondary}.rt2")),
        &description_keys.join(format!("{secondary}.rt2.ks.bin")),
    )?;
    let secondary_cn = decode_table(&raw.join(format!("{secondary}.rt2")), &shared_cn_key)?;
    let (frequency, bigrams) = load_models(root)?;

    let rows = primary_cn
        .par_iter()
        .filter_map(|(key, _)| {
            let base = primary_default.get(key)?;
            let converted = to_simplified(
                secondary_default
                    .get(key)
                    .or_else(|| secondary_cn.get(key))
                    .map(String::as_str)
                    .unwrap_or(""),
            );
            let alternatives = vec![
                primary_cn.get(key).cloned().unwrap_or_default(),
                converted,
                to_simplified(secondary_cn.get(key).map(String::as_str).unwrap_or("")),
            ];
            Some((
                key.clone(),
                sanitize_scripts(&merge(base, &alternatives, &frequency, &bigrams)),
            ))
        })
        .collect::<HashMap<_, _>>();
    Ok(rows)
}
