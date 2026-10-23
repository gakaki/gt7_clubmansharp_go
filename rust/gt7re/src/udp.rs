//! GT7 UDP telemetry packet parser (Packet A/B/T/C).
//!
//! Packet sizes: A = 296, B = 316, T = 344, C = 368 bytes.

use anyhow::{bail, Result};
use serde_json::{json, Map, Value};

pub const SIZE_A: usize = 296;
pub const SIZE_B: usize = 316;
pub const SIZE_T: usize = 344;
pub const SIZE_C: usize = 368;

fn f32_at(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn i32_at(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn i16_at(b: &[u8], o: usize) -> i16 {
    i16::from_le_bytes([b[o], b[o + 1]])
}
fn f32v(b: &[u8], o: usize, n: usize) -> Vec<Value> {
    (0..n).map(|i| json!(f32_at(b, o + i * 4))).collect()
}

pub fn parse(pkt: &[u8]) -> Result<Value> {
    if pkt.len() < SIZE_A {
        bail!("packet too short: {} (need at least {})", pkt.len(), SIZE_A);
    }
    let mut m = Map::new();
    m.insert("packet_size".into(), json!(pkt.len()));
    m.insert(
        "version".into(),
        json!(match pkt.len() {
            SIZE_A => "A",
            SIZE_B => "B",
            SIZE_T => "T",
            SIZE_C => "C",
            _ => "unknown",
        }),
    );
    m.insert(
        "magic".into(),
        json!(format!("0x{:08X}", i32_at(pkt, 0) as u32)),
    );
    m.insert("position".into(), json!(f32v(pkt, 4, 3)));
    m.insert("world_velocity".into(), json!(f32v(pkt, 16, 3)));
    m.insert("rotation".into(), json!(f32v(pkt, 28, 3)));
    m.insert(
        "orientation_relative_to_north".into(),
        json!(f32_at(pkt, 40)),
    );
    m.insert("angular_velocity".into(), json!(f32v(pkt, 44, 3)));
    m.insert("body_height".into(), json!(f32_at(pkt, 56)));
    m.insert("engine_rpm".into(), json!(f32_at(pkt, 60)));
    m.insert(
        "iv".into(),
        json!(pkt[64..68]
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()),
    );
    m.insert("fuel_level".into(), json!(f32_at(pkt, 68)));
    m.insert("fuel_capacity".into(), json!(f32_at(pkt, 72)));
    m.insert("speed".into(), json!(f32_at(pkt, 76)));
    m.insert("boost".into(), json!(f32_at(pkt, 80)));
    m.insert("oil_pressure".into(), json!(f32_at(pkt, 84)));
    m.insert("water_temp".into(), json!(f32_at(pkt, 88)));
    m.insert("oil_temp".into(), json!(f32_at(pkt, 92)));
    m.insert("tyre_temp".into(), json!(f32v(pkt, 96, 4)));
    m.insert("packet_id".into(), json!(i32_at(pkt, 112)));
    m.insert("lap_count".into(), json!(i16_at(pkt, 116)));
    m.insert("total_laps".into(), json!(i16_at(pkt, 118)));
    m.insert("best_laptime_ms".into(), json!(i32_at(pkt, 120)));
    m.insert("last_laptime_ms".into(), json!(i32_at(pkt, 124)));
    m.insert("day_progression_ms".into(), json!(i32_at(pkt, 128)));
    m.insert("race_start_position".into(), json!(i16_at(pkt, 132)));
    m.insert("pre_race_num_cars".into(), json!(i16_at(pkt, 134)));
    m.insert("min_alert_rpm".into(), json!(i16_at(pkt, 136)));
    m.insert("max_alert_rpm".into(), json!(i16_at(pkt, 138)));
    m.insert("calc_max_speed".into(), json!(i16_at(pkt, 140)));
    m.insert("flags".into(), json!(i16_at(pkt, 142) as u16));
    m.insert("current_gear".into(), json!(pkt[144] & 0x0F));
    m.insert("suggested_gear".into(), json!((pkt[144] >> 4) & 0x0F));
    m.insert("throttle".into(), json!(pkt[145]));
    m.insert("brake".into(), json!(pkt[146]));
    m.insert("road_plane".into(), json!(f32v(pkt, 148, 3)));
    m.insert("road_plane_distance".into(), json!(f32_at(pkt, 160)));
    m.insert("wheel_rps".into(), json!(f32v(pkt, 164, 4)));
    m.insert("tyre_radius".into(), json!(f32v(pkt, 180, 4)));
    m.insert("suspension_height".into(), json!(f32v(pkt, 196, 4)));
    m.insert("unknown_floats".into(), json!(f32v(pkt, 212, 8)));
    m.insert("clutch".into(), json!(f32_at(pkt, 244)));
    m.insert("clutch_engagement".into(), json!(f32_at(pkt, 248)));
    m.insert("rpm_clutch_to_gearbox".into(), json!(f32_at(pkt, 252)));
    m.insert("transmission_top_speed".into(), json!(f32_at(pkt, 256)));
    m.insert("gear_ratios".into(), json!(f32v(pkt, 260, 8)));
    m.insert("car_code".into(), json!(i32_at(pkt, 292)));

    if pkt.len() >= SIZE_B {
        m.insert("wheel_rotation".into(), json!(f32_at(pkt, 296)));
        m.insert("steering_angular_velocity".into(), json!(f32_at(pkt, 300)));
        m.insert("sway".into(), json!(f32_at(pkt, 304)));
        m.insert("heave".into(), json!(f32_at(pkt, 308)));
        m.insert("surge".into(), json!(f32_at(pkt, 312)));
    }
    if pkt.len() >= SIZE_T {
        m.insert("throttle_filtered".into(), json!(pkt[316]));
        m.insert("brake_filtered".into(), json!(pkt[317]));
        m.insert("unknown_u8_318".into(), json!(pkt[318]));
        m.insert("unknown_u8_319".into(), json!(pkt[319]));
        m.insert("torque_vectors".into(), json!(f32v(pkt, 320, 4)));
        m.insert("energy_recovery".into(), json!(f32_at(pkt, 336)));
        m.insert("unknown_f32_340".into(), json!(f32_at(pkt, 340)));
    }
    if pkt.len() >= SIZE_C {
        m.insert(
            "surface_type".into(),
            json!(String::from_utf8_lossy(&pkt[344..348]).into_owned()),
        );
        m.insert("current_lap_ms".into(), json!(i32_at(pkt, 348)));
        m.insert("wheel_steering_angle".into(), json!(f32v(pkt, 352, 2)));
        m.insert("wheel_base".into(), json!(f32_at(pkt, 360)));
        m.insert(
            "car_category".into(),
            json!(String::from_utf8_lossy(&pkt[364..368]).into_owned()),
        );
    }
    Ok(Value::Object(m))
}

pub fn layout() -> Value {
    json!({
        "packet_sizes": {"A": SIZE_A, "B": SIZE_B, "T": SIZE_T, "C": SIZE_C},
        "base_offsets": {
            "magic": 0, "position": 4, "world_velocity": 16, "rotation": 28,
            "orientation_relative_to_north": 40, "angular_velocity": 44,
            "body_height": 56, "engine_rpm": 60, "iv": 64, "fuel_level": 68,
            "fuel_capacity": 72, "speed": 76, "boost": 80, "oil_pressure": 84,
            "water_temp": 88, "oil_temp": 92, "tyre_temp": 96, "packet_id": 112,
            "lap_count": 116, "total_laps": 118, "best_laptime_ms": 120,
            "last_laptime_ms": 124, "day_progression_ms": 128,
            "race_start_position": 132, "pre_race_num_cars": 134,
            "min_alert_rpm": 136, "max_alert_rpm": 138, "calc_max_speed": 140,
            "flags": 142, "gears": 144, "throttle": 145, "brake": 146,
            "road_plane": 148, "road_plane_distance": 160, "wheel_rps": 164,
            "tyre_radius": 180, "suspension_height": 196, "clutch": 244,
            "clutch_engagement": 248, "rpm_clutch_to_gearbox": 252,
            "transmission_top_speed": 256, "gear_ratios": 260, "car_code": 292
        },
        "extensions": {
            "B": {"wheel_rotation": 296, "steering_angular_velocity": 300,
                  "sway": 304, "heave": 308, "surge": 312},
            "T": {"throttle_filtered": 316, "brake_filtered": 317,
                  "torque_vectors": 320, "energy_recovery": 336},
            "C": {"surface_type": 344, "current_lap_ms": 348,
                  "wheel_steering_angle": 352, "wheel_base": 360,
                  "car_category": 364}
        },
        "send_heartbeat": "send byte 'A' (0x41) to the PlayStation on UDP port 33740"
    })
}
