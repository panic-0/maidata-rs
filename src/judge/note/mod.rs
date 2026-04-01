pub mod fan_slide;
pub mod hold;
pub mod judge_data;
pub mod slide;
pub mod tap;
pub mod touch;
pub mod touch_hold;

pub use fan_slide::FanSlide;
pub use hold::Hold;
pub use judge_data::JUDGE_DATA;
pub use slide::Slide;
pub use tap::Tap;
pub use touch::Touch;
pub use touch_hold::TouchHold;

use crate::insn::{Key, TouchSensor};
use enum_map::{Enum, EnumMap};
use std::collections::HashMap;

pub(crate) fn key_to_sensor(key: Key) -> TouchSensor {
    TouchSensor::new('A', Some(key.index())).unwrap()
}

pub const FRAME_RATE: f64 = 60.0;

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug, Enum)]
pub enum Timing {
    TooFast,
    FastGood,
    FastGreat3rd,
    FastGreat2nd,
    FastGreat,
    FastPerfect2nd,
    FastPerfect,
    Critical,
    LatePerfect,
    LatePerfect2nd,
    LateGreat,
    LateGreat2nd,
    LateGreat3rd,
    LateGood,
    TooLate,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Enum)]
pub enum JudgeType {
    Tap,
    Touch,
    Slide,
    ExTap,
}

#[derive(Clone, Debug)]
pub struct JudgeParam {
    judge_flame_list: EnumMap<Timing, f64>,
}

impl JudgeParam {
    fn new(judge_flame_list: [f64; 15]) -> Self {
        Self {
            judge_flame_list: EnumMap::from_fn(|i| judge_flame_list[i as usize] / FRAME_RATE),
        }
    }
}

impl AsRef<EnumMap<Timing, f64>> for JudgeParam {
    fn as_ref(&self) -> &EnumMap<Timing, f64> {
        &self.judge_flame_list
    }
}

pub fn get_all_sensors() -> Vec<TouchSensor> {
    let mut sensors = Vec::new();
    for group in "ABDE".chars() {
        for i in 0..8 {
            sensors.push(TouchSensor::new(group, Some(i)).unwrap());
        }
    }
    sensors.push(TouchSensor::new('C', None).unwrap());
    sensors
}

#[derive(Clone, Debug)]
pub struct TouchSensorStates {
    sensors: HashMap<TouchSensor, bool>,
}

impl TouchSensorStates {
    pub fn new() -> Self {
        Self {
            sensors: get_all_sensors()
                .into_iter()
                .map(|sensor| (sensor, false))
                .collect(),
        }
    }

    pub fn sensor_is_on(&self, sensor: TouchSensor) -> bool {
        *self.sensors.get(&sensor).unwrap()
    }

    pub fn activate_sensor(&mut self, sensor: TouchSensor) {
        self.sensors.insert(sensor, true);
    }

    pub fn deactivate_sensor(&mut self, sensor: TouchSensor) {
        self.sensors.insert(sensor, false);
    }
}

impl Default for TouchSensorStates {
    fn default() -> Self {
        Self::new()
    }
}

// call on_sensor() when sensor is on (tap, touch, hold head) (sorting by get_judge_start_time())
// call judge() when:
// + sensor is changed (hold body, slide)
// + note is too late (all)
// don't call on_sensor() or judge() when note's result is already determined
pub trait JudgeNote {
    fn get_start_time(&self) -> f64;
    fn get_end_time(&self) -> f64;
    fn is_too_fast(&self, current_time: f64) -> bool {
        current_time < self.get_start_time()
    }
    fn is_too_late(&self, current_time: f64) -> bool {
        current_time >= self.get_end_time()
    }

    fn get_sensor(&self) -> Option<TouchSensor> {
        None
    }
    // return true if consumed
    fn on_sensor(&mut self, _current_time: f64) -> OnSensorResult {
        OnSensorResult::TooLate
    }
    fn judge(&mut self, _getter: &TouchSensorStates, _current_time: f64);

    fn get_judge_result(&self) -> Option<Timing>;
}

#[derive(Clone, Debug)]
pub enum Note {
    Tap(Tap),
    Touch(Touch),
    Slide(Slide),
    FanSlide(FanSlide),
    Hold(Hold),
    TouchHold(TouchHold),
}

impl Note {
    fn get_impl(&self) -> &dyn JudgeNote {
        match self {
            Note::Tap(t) => t,
            Note::Touch(t) => t,
            Note::Slide(s) => s,
            Note::FanSlide(f) => f,
            Note::Hold(h) => h,
            Note::TouchHold(h) => h,
        }
    }

    fn get_impl_mut(&mut self) -> &mut dyn JudgeNote {
        match self {
            Note::Tap(t) => t,
            Note::Touch(t) => t,
            Note::Slide(s) => s,
            Note::FanSlide(f) => f,
            Note::Hold(h) => h,
            Note::TouchHold(h) => h,
        }
    }
}

#[derive(Clone, Debug)]
pub enum OnSensorResult {
    TooFast,
    Consumed,
    TooLate,
}

impl JudgeNote for Note {
    fn get_start_time(&self) -> f64 {
        self.get_impl().get_start_time()
    }

    fn get_end_time(&self) -> f64 {
        self.get_impl().get_end_time()
    }

    fn is_too_fast(&self, current_time: f64) -> bool {
        self.get_impl().is_too_fast(current_time)
    }

    fn is_too_late(&self, current_time: f64) -> bool {
        self.get_impl().is_too_late(current_time)
    }

    fn get_sensor(&self) -> Option<TouchSensor> {
        self.get_impl().get_sensor()
    }

    fn on_sensor(&mut self, current_time: f64) -> OnSensorResult {
        self.get_impl_mut().on_sensor(current_time)
    }

    fn judge(&mut self, simulator: &TouchSensorStates, current_time: f64) {
        self.get_impl_mut().judge(simulator, current_time)
    }

    fn get_judge_result(&self) -> Option<Timing> {
        self.get_impl().get_judge_result()
    }
}
