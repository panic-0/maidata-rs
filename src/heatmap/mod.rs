pub mod encode;
pub mod sensor;

pub use encode::{HeatmapEncoder, FRAME_DT, NUM_CHANNELS};
pub use sensor::{sensor_index, Position, SensorLayout, NUM_SENSORS};

#[cfg(test)]
mod tests;
