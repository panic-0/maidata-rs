use crate::heatmap::encode::*;
use crate::heatmap::sensor::*;
use crate::insn::{Key, TouchSensor};
use crate::materialize::*;

#[test]
fn test_sensor_layout_center() {
    let layout = SensorLayout::new();
    let c = layout.position(16);
    assert!(c.x.abs() < 1e-9);
    assert!(c.y.abs() < 1e-9);
}

#[test]
fn test_sensor_layout_d_ring_radius() {
    let layout = SensorLayout::new();
    for i in 17..25u8 {
        let p = layout.position(i);
        let r = (p.x * p.x + p.y * p.y).sqrt();
        assert!(
            (r - 0.98).abs() < 0.05,
            "D{} radius = {r}, expected ~0.98",
            i - 17 + 1
        );
    }
}

#[test]
fn test_sensor_layout_d1_top() {
    let layout = SensorLayout::new();
    let d1 = layout.position(17);
    assert!(d1.x.abs() < 0.05, "D1.x = {}", d1.x);
    assert!(d1.y > 0.9, "D1.y = {}", d1.y);
}

#[test]
fn test_sensor_index_matches_export_contract() {
    let ordered = [
        TouchSensor::new('A', Some(0)).unwrap(),
        TouchSensor::new('A', Some(7)).unwrap(),
        TouchSensor::new('B', Some(0)).unwrap(),
        TouchSensor::new('B', Some(7)).unwrap(),
        TouchSensor::new('C', None).unwrap(),
        TouchSensor::new('D', Some(0)).unwrap(),
        TouchSensor::new('D', Some(7)).unwrap(),
        TouchSensor::new('E', Some(0)).unwrap(),
        TouchSensor::new('E', Some(7)).unwrap(),
    ];
    let expected = [0, 7, 8, 15, 16, 17, 24, 25, 32];

    for (sensor, expected_index) in ordered.iter().zip(expected) {
        assert_eq!(
            sensor_index(sensor),
            expected_index,
            "sensor {sensor} should map to export index {expected_index}"
        );
    }
}

#[test]
fn test_encode_single_tap() {
    let encoder = HeatmapEncoder::new();
    let notes = vec![Note::Tap(MaterializedTap {
        ts: 0.1,
        key: Key::new(0).unwrap(),
        shape: MaterializedTapShape::Ring,
        is_break: false,
        is_ex: false,
        is_each: false,
    })];
    let frames = encoder.encode(&notes);

    assert_eq!(frames.dim().1, NUM_FEATURES);
    assert!(frames[[0, TAP_FEATURE_OFFSET]] > 0.0);
    for key in 1..NUM_TAP_FEATURES {
        assert_eq!(frames[[0, TAP_FEATURE_OFFSET + key]], 0.0);
    }
    for feature in 0..NUM_FEATURES {
        if feature != TAP_FEATURE_OFFSET {
            assert_eq!(frames[[0, feature]], 0.0);
        }
    }
}

#[test]
fn test_encode_touch() {
    let encoder = HeatmapEncoder::new();
    let notes = vec![Note::Touch(MaterializedTouch {
        ts: 0.1,
        sensor: TouchSensor::new('C', None).unwrap(),
        is_each: false,
    })];
    let frames = encoder.encode(&notes);

    assert!(frames[[0, TOUCH_FEATURE_OFFSET + 16]] > 0.0);
}

#[test]
fn test_encode_hold_head_and_coverage() {
    let encoder = HeatmapEncoder::new();
    let notes = vec![Note::Hold(MaterializedHold {
        ts: 0.0,
        dur: 0.5,
        key: Key::new(2).unwrap(),
        is_break: false,
        is_ex: false,
        is_each: false,
    })];
    let frames = encoder.encode(&notes);

    assert!(frames[[0, TAP_FEATURE_OFFSET + 2]] > 0.0);
    assert!(frames[[0, HOLD_FEATURE_OFFSET]] > 0.9);
    assert!(frames[[1, HOLD_FEATURE_OFFSET]] > 0.9);
    assert!(frames[[2, HOLD_FEATURE_OFFSET]] > 0.1);
}

#[test]
fn test_encode_touch_hold_head_and_coverage() {
    let encoder = HeatmapEncoder::new();
    let notes = vec![Note::TouchHold(MaterializedTouchHold {
        ts: 0.0,
        dur: 0.2,
        sensor: TouchSensor::new('C', None).unwrap(),
        is_each: false,
    })];
    let frames = encoder.encode(&notes);

    assert!(frames[[0, TOUCH_FEATURE_OFFSET + 16]] > 0.0);
    assert!(frames[[0, HOLD_FEATURE_OFFSET]] > 0.9);
}

#[test]
fn test_encode_two_hand_hold_occupancy() {
    let encoder = HeatmapEncoder::new();
    let notes = vec![
        Note::Hold(MaterializedHold {
            ts: 0.0,
            dur: 0.2,
            key: Key::new(0).unwrap(),
            is_break: false,
            is_ex: false,
            is_each: false,
        }),
        Note::Hold(MaterializedHold {
            ts: 0.1,
            dur: 0.2,
            key: Key::new(1).unwrap(),
            is_break: false,
            is_ex: false,
            is_each: false,
        }),
        Note::Hold(MaterializedHold {
            ts: 0.1,
            dur: 0.2,
            key: Key::new(2).unwrap(),
            is_break: false,
            is_ex: false,
            is_each: false,
        }),
    ];
    let frames = encoder.encode(&notes);

    assert!((frames[[0, HOLD_FEATURE_OFFSET]] - 1.0).abs() < 0.01);
    assert!((frames[[0, HOLD_FEATURE_OFFSET + 1]] - 0.5).abs() < 0.01);
    assert!(frames[[0, HOLD_FEATURE_OFFSET]] >= frames[[0, HOLD_FEATURE_OFFSET + 1]]);
}

#[test]
fn test_encode_accumulates_tap_counts() {
    let encoder = HeatmapEncoder::new();
    let notes = vec![
        Note::Tap(MaterializedTap {
            ts: 0.1,
            key: Key::new(0).unwrap(),
            shape: MaterializedTapShape::Ring,
            is_break: false,
            is_ex: false,
            is_each: false,
        }),
        Note::Tap(MaterializedTap {
            ts: 0.1,
            key: Key::new(0).unwrap(),
            shape: MaterializedTapShape::Ring,
            is_break: false,
            is_ex: false,
            is_each: false,
        }),
    ];
    let frames = encoder.encode(&notes);

    assert!(
        (frames[[0, TAP_FEATURE_OFFSET]] - 2.0).abs() < 0.01,
        "expected 2.0, got {}",
        frames[[0, TAP_FEATURE_OFFSET]]
    );
}
