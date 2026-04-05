use crate::insn::TouchSensor;

/// 2D position in normalized coordinates (kept for Python-side heatmap rendering).
#[derive(Copy, Clone, Debug)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// Global export index for a TouchSensor.
///
/// This ordering is the contract for exported `[T, 33, C]` tensors and must stay
/// aligned with Python-side `training/model.py::_PIXEL_POSITIONS`:
/// A1..A8, B1..B8, C, D1..D8, E1..E8.
pub fn sensor_index(sensor: &TouchSensor) -> u8 {
    match (sensor.group(), sensor.index()) {
        ('A', Some(i)) => i,
        ('B', Some(i)) => i + 8,
        ('C', None) => 16,
        ('D', Some(i)) => i + 17,
        ('E', Some(i)) => i + 25,
        _ => unreachable!(),
    }
}

/// Total number of touch sensors.
pub const NUM_SENSORS: usize = 33;

/// Raw pixel coordinates from the maimai DX touch panel.
const PIXEL_POSITIONS: [(f64, f64); 33] = [
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

const CENTER: f64 = 720.0;
const SCALE: f64 = 600.0;

fn normalize(px: f64, py: f64) -> Position {
    Position {
        x: (px - CENTER) / SCALE,
        y: (CENTER - py) / SCALE,
    }
}

/// Sensor layout: provides normalized 2D positions for the 33 touch sensors.
/// Keys (1-8) map to the same indices as A-ring sensors (0-7).
pub struct SensorLayout {
    positions: [Position; NUM_SENSORS],
}

impl Default for SensorLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl SensorLayout {
    pub fn new() -> Self {
        let positions = std::array::from_fn(|i| {
            let (px, py) = PIXEL_POSITIONS[i];
            normalize(px, py)
        });
        Self { positions }
    }

    /// Get position of a touch sensor by its global index (0-32).
    pub fn position(&self, index: u8) -> Position {
        self.positions[index as usize]
    }

    /// Get all 33 positions as a slice.
    pub fn positions(&self) -> &[Position; 33] {
        &self.positions
    }
}
