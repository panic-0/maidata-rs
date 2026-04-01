use super::{
    key_to_sensor, JudgeNote, JudgeType, OnSensorResult, Timing, TouchSensorStates, JUDGE_DATA,
};
use crate::insn::TouchSensor;

#[derive(Clone, Debug)]
pub struct Tap {
    pub sensor: TouchSensor,
    pub appear_time: f64,
    pub _is_break: bool,
    pub _is_ex: bool,

    judge_type: JudgeType,

    result: Option<Timing>,
}

impl Tap {
    pub fn new(key: crate::insn::Key, appear_time: f64, is_break: bool, is_ex: bool) -> Self {
        Self {
            sensor: key_to_sensor(key),
            appear_time,
            _is_break: is_break,
            _is_ex: is_ex,
            judge_type: if is_ex {
                JudgeType::ExTap
            } else {
                JudgeType::Tap
            },
            result: None,
        }
    }
}

impl JudgeNote for Tap {
    fn get_start_time(&self) -> f64 {
        self.appear_time + JUDGE_DATA.judge_param(self.judge_type).as_ref()[Timing::TooFast]
    }

    fn get_end_time(&self) -> f64 {
        self.appear_time + JUDGE_DATA.judge_param(self.judge_type).as_ref()[Timing::LateGood]
    }

    fn get_sensor(&self) -> Option<TouchSensor> {
        Some(self.sensor)
    }

    fn on_sensor(&mut self, current_time: f64) -> OnSensorResult {
        assert!(self.result.is_none());
        if self.is_too_fast(current_time) {
            return OnSensorResult::TooFast;
        }
        self.result = Some(JUDGE_DATA.get_timing(self.judge_type, current_time - self.appear_time));
        if self.result != Some(Timing::TooLate) {
            OnSensorResult::Consumed
        } else {
            OnSensorResult::TooLate
        }
    }

    fn judge(&mut self, _simulator: &TouchSensorStates, current_time: f64) {
        assert!(self.result.is_none());
        if self.is_too_late(current_time) {
            self.result = Some(Timing::TooLate);
        }
    }

    fn get_judge_result(&self) -> Option<Timing> {
        self.result
    }
}
