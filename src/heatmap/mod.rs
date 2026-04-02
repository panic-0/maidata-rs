pub mod encode;
pub mod sensor;

pub use encode::{HeatmapEncoder, FRAME_DT, NUM_CHANNELS};
pub use sensor::{sensor_index, SensorLayout, NUM_SENSORS, Position};

#[cfg(test)]
mod tests;
