//! Garage and Car Collection metadata recovered from GT7's DMSB database.
//!
//! `CarCodeList.dmsb` maps official car ids to car-code strings,
//! `CarType.dmsb` stores six four-slot vehicle-type masks, and
//! `COUNTRY.dmsb` maps the official `carctryN` ids to country metadata.

use anyhow::{bail, Context, Result};
use rayon::prelude::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const CAR_TYPE_IDS: [&str; 24] = [
    "0x00007FAE",
    "0x00007FAF",
    "0x00007FB0",
    "0x00007FB1",
    "0x00007FB2",
    "0x00007FB3",
    "0x00007FB4",
    "0x00007FB5",
    "0x00007FB6",
    "0x00007FB7",
    "0x00007FB8",
    "0x00007FB9",
    "0x00007FBA",
    "0x00007FBB",
    "0x00007FBC",
    "0x00007FBD",
    "0x00007FBE",
    "0x00007FBF",
    "0x00007FC0",
    "0x00007FC1",
    "0x00007FC2",
    "0x00007FC3",
    "0x00007FC4",
    "0x0000A31D",
];

const CAR_TYPE_LABELS: [(&str, &str); 24] = [
    ("Road Car", "公路车"),
    ("Racing Car", "赛车"),
    ("Professionally Tuned", "专业调校"),
    ("Hypercar", "超级跑车"),
    ("Vision Gran Turismo", "Vision Gran Turismo"),
    ("Concept Car", "概念车"),
    ("Safety Car", "安全车"),
    ("Kei Car", "轻型车"),
    ("Electric Car", "电动车"),
    ("Hybrid Car", "混动车"),
    ("Gran Turismo Award", "Gran Turismo大奖"),
    ("SEMA Show", "美国汽车用品改装车展览会"),
    ("GT300", "GT300"),
    ("GT500", "GT500"),
    ("Le Mans", "勒芒"),
    ("Nürburgring 24 Hours", "纽博格林24小时耐力赛"),
    ("WRC", "WRC世界拉力锦标赛"),
    ("Kart", "卡丁车"),
    ("NASCAR", "NASCAR赛车"),
    ("Pikes Peak", "派克峰"),
    ("Midship", "中置引擎"),
    ("Pickup Truck", "小型轻便货车"),
    ("Hatchback", "掀背车"),
    ("Gran Turismo Trophy", "Gran Turismo奖杯"),
];

const SPEC_FIELDS: [(&str, &str, &str); 11] = [
    ("carClass", "Car Class", "车辆级别"),
    ("performancePoint", "Performance Points", "性能点数"),
    ("displacement", "Displacement", "排气量"),
    ("maxPower", "Max Power", "最大马力"),
    ("maxTorque", "Max Torque", "最大扭矩"),
    ("weight", "Weight", "车重"),
    ("driveTrain", "Drivetrain", "传动系统"),
    ("aspirationLong", "Aspiration", "进气形式"),
    ("length", "Length", "全长"),
    ("width", "Width", "全宽"),
    ("height", "Height", "全高"),
];

#[derive(Clone, Debug)]
struct Dmsb {
    data: Vec<u8>,
    rows: Vec<Vec<u8>>,
    table: usize,
}

impl Dmsb {
    fn parse(path: &Path) -> Result<Self> {
        let data = fs::read(path).with_context(|| path.display().to_string())?;
        if data.get(..4) != Some(b"dmsb") {
            bail!("{}: not a DMSB file", path.display());
        }
        let count = u32le(&data, 8)? as usize;
        let table = u64le(&data, 0x18)? as usize;
        if count == 0 || table <= 0x30 || table > data.len() {
            bail!("{}: invalid DMSB table layout", path.display());
        }
        let row_size = (table - 0x30) / count;
        if row_size == 0 || 0x30 + count * row_size != table {
            bail!("{}: unsupported DMSB row size", path.display());
        }
        let rows = (0..count)
            .map(|index| data[0x30 + index * row_size..0x30 + (index + 1) * row_size].to_vec())
            .collect();
        Ok(Self { data, rows, table })
    }

    fn string_at(&self, offset: u32) -> String {
        let start = self.table.saturating_add(offset as usize);
        if start >= self.data.len() {
            return String::new();
        }
        let end = self.data[start..]
            .iter()
            .position(|byte| *byte == 0)
            .map(|length| start + length)
            .unwrap_or(self.data.len());
        String::from_utf8_lossy(&self.data[start..end]).into_owned()
    }

    fn string(&self, row: &[u8], offset: usize) -> Result<String> {
        Ok(self.string_at(u32le(row, offset)?))
    }
}

fn u32le(data: &[u8], offset: usize) -> Result<u32> {
    let bytes = data
        .get(offset..offset + 4)
        .with_context(|| format!("missing u32 at 0x{offset:X}"))?;
    Ok(u32::from_le_bytes(bytes.try_into()?))
}

fn u64le(data: &[u8], offset: usize) -> Result<u64> {
    let bytes = data
        .get(offset..offset + 8)
        .with_context(|| format!("missing u64 at 0x{offset:X}"))?;
    Ok(u64::from_le_bytes(bytes.try_into()?))
}

fn find_dmsb(root: &Path, name: &str) -> Result<PathBuf> {
    let direct = root.join(name);
    if direct.is_file() {
        return Ok(direct);
    }
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .find(|entry| entry.file_type().is_file() && entry.file_name() == name)
        .map(|entry| entry.into_path())
        .with_context(|| format!("{name} not found below {}", root.display()))
}

#[derive(Clone, Debug)]
struct Country {
    id: u32,
    name_en: String,
    name_zh: String,
    code: String,
    region: String,
}

impl Country {
    fn as_json(&self) -> Value {
        json!({
            "id": self.id,
            "nameEn": self.name_en,
            "nameZh": self.name_zh,
            "code": self.code,
            "region": self.region,
        })
    }
}

#[derive(Clone, Debug)]
struct VehicleType {
    index: usize,
    id: &'static str,
    label_en: &'static str,
    label_zh: &'static str,
}

impl VehicleType {
    fn as_json(&self) -> Value {
        json!({
            "index": self.index,
            "id": self.id,
            "nameEn": self.label_en,
            "nameZh": self.label_zh,
            "tagEn": format!("#{}", self.label_en),
            "tagZh": format!("#{}", self.label_zh),
        })
    }
}

fn country_name_zh(code: &str) -> &'static str {
    match code {
        "pdi" => "其他",
        "jp" => "日本",
        "us" => "美国",
        "gb" => "英国",
        "de" => "德国",
        "fr" => "法国",
        "it" => "意大利",
        "au" => "澳大利亚",
        "kr" => "韩国",
        "be" => "比利时",
        "nl" => "荷兰",
        "se" => "瑞典",
        "es" => "西班牙",
        "ca" => "加拿大",
        "at" => "奥地利",
        "ch" => "瑞士",
        "br" => "巴西",
        "hr" => "克罗地亚",
        "cz" => "捷克",
        "cn" => "中国",
        "ae" => "阿联酋",
        _ => "其他",
    }
}

fn country_name_en(code: &str) -> &'static str {
    match code {
        "pdi" => "Other",
        "jp" => "Japan",
        "us" => "USA",
        "gb" => "UK",
        "de" => "Germany",
        "fr" => "France",
        "it" => "Italy",
        "au" => "Australia",
        "kr" => "South Korea",
        "be" => "Belgium",
        "nl" => "Netherlands",
        "se" => "Sweden",
        "es" => "Spain",
        "ca" => "Canada",
        "at" => "Austria",
        "ch" => "Switzerland",
        "br" => "Brazil",
        "hr" => "Croatia",
        "cz" => "Czech Republic",
        "cn" => "China",
        "ae" => "UAE",
        _ => "Other",
    }
}

fn parse_countries(dmsb: &Dmsb) -> Result<HashMap<u32, Country>> {
    let mut countries = HashMap::new();
    for row in &dmsb.rows {
        let id = u32le(row, 0)?;
        let code = dmsb.string(row, 16)?.trim().to_lowercase();
        if code.is_empty() {
            continue;
        }
        countries.insert(
            id,
            Country {
                id,
                name_en: country_name_en(&code).to_string(),
                name_zh: country_name_zh(&code).to_string(),
                code,
                region: dmsb.string(row, 32)?.trim().to_string(),
            },
        );
    }
    Ok(countries)
}

fn parse_car_codes(dmsb: &Dmsb) -> Result<HashMap<u32, (String, Option<u32>)>> {
    let mut result = HashMap::new();
    for row in &dmsb.rows {
        let official_id = u32le(row, 0)?;
        let code = dmsb.string(row, 8)?.trim().to_string();
        if official_id == 0 || code.is_empty() {
            continue;
        }
        let country_id = u32le(row, 32).ok();
        result.insert(official_id, (code, country_id));
    }
    Ok(result)
}

fn parse_vehicle_types(dmsb: &Dmsb) -> Result<HashMap<String, (String, [u32; 6])>> {
    let mut result = HashMap::new();
    for row in &dmsb.rows {
        let code = dmsb.string(row, 8)?.trim().to_string();
        if code.is_empty() {
            continue;
        }
        let mut masks = [0_u32; 6];
        for (index, mask) in masks.iter_mut().enumerate() {
            *mask = u32le(row, 20 + index * 4)?;
        }
        result.insert(code, (String::new(), masks));
    }
    Ok(result)
}

fn decoded_types(masks: [u32; 6]) -> Vec<VehicleType> {
    let slot_bits = [0x0000_0001_u32, 0x0000_0100, 0x0001_0000, 0x0100_0000];
    let mut types = Vec::new();
    for (word, mask) in masks.into_iter().enumerate() {
        for (slot, bit) in slot_bits.into_iter().enumerate() {
            if mask & bit == 0 {
                continue;
            }
            let index = word * 4 + slot;
            let Some((label_en, label_zh)) = CAR_TYPE_LABELS.get(index).copied() else {
                continue;
            };
            types.push(VehicleType {
                index,
                id: CAR_TYPE_IDS[index],
                label_en,
                label_zh,
            });
        }
    }
    types
}

#[derive(Debug)]
pub struct Catalog {
    code_by_official: HashMap<u32, (String, Option<u32>)>,
    type_by_code: HashMap<String, [u32; 6]>,
    countries: HashMap<u32, Country>,
}

impl Catalog {
    pub fn load(dmsb_dir: &Path) -> Result<Self> {
        let country_path = find_dmsb(dmsb_dir, "COUNTRY.dmsb")?;
        let codes_path = find_dmsb(dmsb_dir, "CarCodeList.dmsb")?;
        let types_path = find_dmsb(dmsb_dir, "CarType.dmsb")?;
        let (loaded, types) = rayon::join(
            || -> Result<_> {
                let (countries, codes) = rayon::join(
                    || -> Result<_> { parse_countries(&Dmsb::parse(&country_path)?) },
                    || -> Result<_> { parse_car_codes(&Dmsb::parse(&codes_path)?) },
                );
                Ok((countries?, codes?))
            },
            || -> Result<_> {
                let parsed = parse_vehicle_types(&Dmsb::parse(&types_path)?)?;
                Ok(parsed
                    .into_iter()
                    .map(|(code, (_, masks))| (code, masks))
                    .collect())
            },
        );
        let (countries, code_by_official) = loaded?;
        Ok(Self {
            code_by_official,
            type_by_code: types?,
            countries,
        })
    }

    fn car_code(&self, car: &Value) -> Option<String> {
        if let Some(official_id) = car
            .pointer("/specs/officialId")
            .and_then(Value::as_str)
            .and_then(|value| value.strip_prefix("car"))
            .and_then(|value| value.parse::<u32>().ok())
        {
            if let Some((code, _)) = self.code_by_official.get(&official_id) {
                return Some(code.clone());
            }
        }
        let direct = car
            .pointer("/texts/vehicleCollection/carKey")
            .and_then(Value::as_str)
            .or_else(|| car.get("officialId").and_then(Value::as_str));
        direct
            .filter(|code| self.type_by_code.contains_key(*code))
            .map(str::to_string)
    }

    fn country(&self, car: &Value, code: &str) -> Option<&Country> {
        let from_specs = car
            .pointer("/specs/zh/countryId")
            .or_else(|| car.pointer("/specs/en/countryId"))
            .and_then(Value::as_str)
            .and_then(|value| value.strip_prefix("carctry"))
            .and_then(|value| value.parse::<u32>().ok());
        from_specs
            .or_else(|| {
                self.code_by_official
                    .values()
                    .find(|(candidate, _)| candidate == code)
                    .and_then(|(_, country_id)| *country_id)
            })
            .and_then(|id| self.countries.get(&id))
    }

    pub fn enrich(&self, cars: &mut [Value]) {
        cars.par_iter_mut().for_each(|car| {
            let Some(code) = self.car_code(car) else {
                return;
            };
            let vehicle_types = self
                .type_by_code
                .get(&code)
                .copied()
                .map(decoded_types)
                .unwrap_or_default();
            let country = self.country(car, &code).map(Country::as_json);
            let Some(object) = car.as_object_mut() else {
                return;
            };
            object.insert(
                "garageDetails".into(),
                json!({
                    "source": "GT7 PS5 DMSB",
                    "dataAvailable": true,
                    "carCode": code,
                    "country": country,
                    "vehicleTypes": vehicle_types
                        .iter()
                        .map(VehicleType::as_json)
                        .collect::<Vec<_>>(),
                    "vehicleTypeIds": vehicle_types
                        .iter()
                        .map(|item| item.id)
                        .collect::<Vec<_>>(),
                }),
            );
        });
    }
}

fn collection_json(car: &Value) -> Value {
    let text = car
        .pointer("/texts/vehicleCollection")
        .cloned()
        .unwrap_or(Value::Null);
    json!({
        "titleEn": text.get("titleEn").cloned().unwrap_or(json!("Car Collection")),
        "titleZh": text.get("titleZh").cloned().unwrap_or(json!("车辆收藏")),
        "sourceName": text.get("sourceName").cloned().unwrap_or(json!("")),
        "textIds": text.get("textIds").cloned().unwrap_or(json!({})),
        "catch": text.get("catch").cloned().unwrap_or(json!({})),
        "long": text.get("long").cloned().unwrap_or(json!({})),
    })
}

pub fn report(cars: &[Value]) -> Value {
    let records = cars
        .par_iter()
        .filter(|car| car.get("specs").is_some() || car.get("garageDetails").is_some())
        .map(|car| {
            json!({
                "id": car.get("id").cloned().unwrap_or(json!("")),
                "officialId": car.pointer("/specs/officialId")
                    .cloned()
                    .or_else(|| car.get("officialId").cloned())
                    .unwrap_or(json!("")),
                "nameEn": car.get("nameEn").cloned().unwrap_or(json!("")),
                "nameZh": car.get("nameZh").cloned().unwrap_or(json!("")),
                "brand": car.get("brand").cloned().unwrap_or(json!("")),
                "garage": car.get("garageDetails").cloned().unwrap_or_else(|| json!({
                    "source": "GT7 PS5 DMSB",
                    "dataAvailable": false,
                    "carCode": null,
                    "country": null,
                    "vehicleTypes": [],
                    "vehicleTypeIds": [],
                })),
                "specs": car.get("specs").cloned().unwrap_or(Value::Null),
                "collection": collection_json(car),
            })
        })
        .collect::<Vec<_>>();
    let with_specs = records
        .iter()
        .filter(|car| car.get("specs").and_then(Value::as_object).is_some())
        .count();
    let with_types = records
        .iter()
        .filter(|car| {
            car.pointer("/garage/vehicleTypes")
                .and_then(Value::as_array)
                .map(|items| !items.is_empty())
                .unwrap_or(false)
        })
        .count();
    let with_country = records
        .iter()
        .filter(|car| {
            car.pointer("/garage/country")
                .is_some_and(|value| !value.is_null())
        })
        .count();
    json!({
        "generatedFrom": "GT7 PS5 CarCodeList.dmsb + CarType.dmsb + COUNTRY.dmsb + official GT7 car-list specifications",
        "sourceFiles": ["CarCodeList.dmsb", "CarType.dmsb", "COUNTRY.dmsb"],
        "counts": {
            "cars": records.len(),
            "withSpecifications": with_specs,
            "withVehicleTypes": with_types,
            "withCountry": with_country,
            "vehicleTypeDefinitions": CAR_TYPE_IDS.len(),
        },
        "fieldLabels": SPEC_FIELDS
            .iter()
            .map(|(key, en, zh)| json!({"key": key, "en": en, "zh": zh}))
            .collect::<Vec<_>>(),
        "carTypeLabels": CAR_TYPE_IDS
            .iter()
            .zip(CAR_TYPE_LABELS)
            .enumerate()
            .map(|(index, (id, (en, zh)))| {
                json!({
                    "index": index,
                    "id": id,
                    "nameEn": en,
                    "nameZh": zh,
                    "tagEn": format!("#{en}"),
                    "tagZh": format!("#{zh}"),
                })
            })
            .collect::<Vec<_>>(),
        "cars": records,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_peugeot_205_vehicle_types() {
        let types = decoded_types([0x100, 0, 0, 0, 1, 1]);
        let labels = types.iter().map(|item| item.label_en).collect::<Vec<_>>();
        assert_eq!(labels, ["Racing Car", "WRC", "Midship"]);
    }

    #[test]
    fn garage_json_keeps_four_slot_mask_layout() {
        let labels = CAR_TYPE_IDS
            .iter()
            .zip(CAR_TYPE_LABELS)
            .map(|(id, (en, _))| (*id, en))
            .collect::<HashMap<_, _>>();
        assert_eq!(labels["0x00007FAF"], "Racing Car");
        assert_eq!(labels["0x00007FBE"], "WRC");
        assert_eq!(labels["0x00007FC2"], "Midship");
    }
}
