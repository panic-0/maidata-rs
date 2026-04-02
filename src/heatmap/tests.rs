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
    assert!(frames.dim().0 >= 1);
    // Key 0 → sensor 0 (A-ring), tap_instant channel
    assert!(
        frames[[0, 0, CH_TAP_INSTANT]] > 0.0,
        "sensor 0 tap_instant should be > 0"
    );
    // Other sensors should be zero
    for si in 1..NUM_SENSORS {
        assert_eq!(frames[[0, si, CH_TAP_INSTANT]], 0.0);
    }
    // Other channels zero
    for ch in 0..NUM_CHANNELS {
        if ch != CH_TAP_INSTANT {
            assert_eq!(frames[[0, 0, ch]], 0.0);
        }
    }
}

#[test]
fn test_encode_break_tap() {
    let encoder = HeatmapEncoder::new();
    let notes = vec![Note::Tap(MaterializedTap {
        ts: 0.0,
        key: Key::new(3).unwrap(),
        shape: MaterializedTapShape::Ring,
        is_break: true,
        is_ex: false,
        is_each: false,
    })];
    let frames = encoder.encode(&notes);
    assert!(frames[[0, 3, CH_TAP_INSTANT]] > 0.0);
    assert!(frames[[0, 3, CH_BREAK]] > 0.0);
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
    // C sensor = index 16
    assert!(frames[[0, 16, CH_TOUCH_INSTANT]] > 0.0);
}

#[test]
fn test_encode_hold_coverage() {
    let encoder = HeatmapEncoder::new();
    // Hold from t=0.0 to t=0.5, key 2
    let notes = vec![Note::Hold(MaterializedHold {
        ts: 0.0,
        dur: 0.5,
        key: Key::new(2).unwrap(),
        is_break: false,
        is_ex: false,
        is_each: false,
    })];
    let frames = encoder.encode(&notes);
    // Frames 0, 1 fully covered, frame 2 partially (0.1/0.2 = 0.5)
    assert!(
        frames[[0, 2, CH_HOLD]] > 0.9,
        "frame 0 hold = {}",
        frames[[0, 2, CH_HOLD]]
    );
    assert!(
        frames[[1, 2, CH_HOLD]] > 0.9,
        "frame 1 hold = {}",
        frames[[1, 2, CH_HOLD]]
    );
    assert!(
        frames[[2, 2, CH_HOLD]] > 0.1,
        "frame 2 hold = {}",
        frames[[2, 2, CH_HOLD]]
    );
}

#[test]
fn test_encode_accumulates() {
    let encoder = HeatmapEncoder::new();
    // Two taps at same key and time
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
        (frames[[0, 0, CH_TAP_INSTANT]] - 2.0).abs() < 0.01,
        "expected 2.0, got {}",
        frames[[0, 0, CH_TAP_INSTANT]]
    );
}
