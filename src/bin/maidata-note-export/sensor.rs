use maidata::insn::{Key, TouchSensor};

// ── Sensor positions (from sensor_position.md) ──────────────────────────

pub const SENSOR_POSITIONS: [(f64, f64); 33] = [
    // A1 ~ A8
    (967.50, 180.50),
    (1260.50, 472.00),
    (1260.50, 967.00),
    (969.00, 1259.50),
    (473.00, 1259.00),
    (182.00, 967.50),
    (181.00, 471.50),
    (473.50, 179.50),
    // B1 ~ B8
    (829.50, 443.00),
    (997.50, 612.50),
    (997.50, 826.00),
    (827.50, 995.50),
    (613.50, 995.50),
    (444.00, 827.00),
    (444.50, 612.50),
    (613.50, 443.50),
    // C
    (720.00, 720.00),
    // D1 ~ D8
    (720.50, 128.00),
    (1149.50, 291.50),
    (1312.50, 719.50),
    (1149.00, 1147.00),
    (720.50, 1312.00),
    (292.50, 1148.00),
    (129.50, 718.50),
    (292.00, 292.50),
    // E1 ~ E8
    (720.50, 310.00),
    (1010.00, 429.50),
    (1130.50, 718.00),
    (1010.00, 1007.50),
    (720.00, 1129.50),
    (431.00, 1007.50),
    (311.50, 718.00),
    (431.00, 429.50),
];

pub fn key_to_sensor_idx(key: Key) -> usize {
    key.index() as usize
}

pub fn touch_sensor_to_idx(sensor: &TouchSensor) -> usize {
    let group = sensor.group();
    let idx = sensor.index().unwrap_or(0) as usize;
    match group {
        'A' => idx,
        'B' => 8 + idx,
        'C' => 16,
        'D' => 17 + idx,
        'E' => 25 + idx,
        _ => panic!("unknown sensor group"),
    }
}

pub fn sensor_position(sensor: &TouchSensor) -> (f64, f64) {
    SENSOR_POSITIONS[touch_sensor_to_idx(sensor)]
}

pub fn key_position(key: Key) -> (f64, f64) {
    SENSOR_POSITIONS[key_to_sensor_idx(key)]
}

pub fn centroid_of_sensors(sensors: &[TouchSensor]) -> (f64, f64) {
    let (sx, sy): (f64, f64) = sensors
        .iter()
        .map(sensor_position)
        .fold((0.0, 0.0), |(ax, ay), (x, y)| (ax + x, ay + y));
    let n = sensors.len() as f64;
    (sx / n, sy / n)
}

// ── Key neighbor helpers ─────────────────────────────────────────────────

pub fn left_neighbor(key: Key) -> Key {
    Key::new((key.index() + 7) % 8).unwrap()
}

pub fn right_neighbor(key: Key) -> Key {
    Key::new((key.index() + 1) % 8).unwrap()
}

pub fn parse_sensor_idx(name: &str) -> usize {
    let group = name.as_bytes()[0];
    let idx = if name.len() > 1 {
        name[1..].parse::<usize>().unwrap() - 1
    } else {
        0
    };
    match group {
        b'A' => idx,
        b'B' => 8 + idx,
        b'C' => 16,
        b'D' => 17 + idx,
        b'E' => 25 + idx,
        _ => panic!("unknown sensor group"),
    }
}

pub const MERGE_DIST_THRESHOLD: f64 = 350.0; // pixels

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sensor_idx() {
        assert_eq!(parse_sensor_idx("A1"), 0);
        assert_eq!(parse_sensor_idx("A8"), 7);
        assert_eq!(parse_sensor_idx("B1"), 8);
        assert_eq!(parse_sensor_idx("B8"), 15);
        assert_eq!(parse_sensor_idx("C"), 16);
        assert_eq!(parse_sensor_idx("D1"), 17);
        assert_eq!(parse_sensor_idx("E1"), 25);
        assert_eq!(parse_sensor_idx("E8"), 32);
    }

    #[test]
    fn test_key_position_matches_sensor_position() {
        let key_pos = key_position(Key::new(0).unwrap());
        let sensor_pos = SENSOR_POSITIONS[0];
        assert_eq!(key_pos, sensor_pos);
    }

    #[test]
    fn test_all_key_positions_in_circle() {
        let center = (720.0, 720.0);
        for i in 0..8 {
            let pos = SENSOR_POSITIONS[i];
            let dist = ((pos.0 - center.0).powi(2) + (pos.1 - center.1).powi(2)).sqrt();
            assert!(
                dist > 400.0 && dist < 700.0,
                "A{} at dist {} from center",
                i + 1,
                dist
            );
        }
    }
}
