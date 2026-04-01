use super::{
    key_to_sensor, JudgeNote, JudgeType, OnSensorResult, Timing, TouchSensorStates, JUDGE_DATA,
};
use crate::insn::TouchSensor;

#[derive(Clone, Debug)]
pub struct Hold {
    pub sensor: TouchSensor,
    pub appear_time: f64,
    pub tail_time: f64,
    pub _is_break: bool,
    pub _is_ex: bool,

    head_judge_type: JudgeType,
    pub head_result: Option<Timing>,
    pub prev_state: Option<bool>,
    prev_time: Option<f64>,
    release_time: f64,

    result: Option<Timing>,
}

impl Hold {
    pub fn new(key: crate::insn::Key, appear_time: f64, tail_time: f64, is_break: bool, is_ex: bool) -> Self {
        Self {
            sensor: key_to_sensor(key),
            appear_time,
            tail_time,
            _is_break: is_break,
            _is_ex: is_ex,
            head_judge_type: if is_ex {
                JudgeType::ExTap
            } else {
                JudgeType::Tap
            },
            head_result: None,
            prev_state: None,
            prev_time: None,
            release_time: 0.0,
            result: None,
        }
    }
}

impl JudgeNote for Hold {
    fn get_start_time(&self) -> f64 {
        self.appear_time + JUDGE_DATA.judge_param(self.head_judge_type).as_ref()[Timing::TooFast]
    }

    fn get_end_time(&self) -> f64 {
        f64::max(
            self.appear_time
                + JUDGE_DATA.judge_param(self.head_judge_type).as_ref()[Timing::LateGood],
            self.tail_time,
        )
    }

    fn get_sensor(&self) -> Option<TouchSensor> {
        Some(self.sensor)
    }

    fn on_sensor(&mut self, current_time: f64) -> OnSensorResult {
        assert!(self.result.is_none());
        if current_time < self.get_start_time() {
            return OnSensorResult::TooFast;
        }
        self.head_result =
            Some(JUDGE_DATA.get_timing(self.head_judge_type, current_time - self.appear_time));
        if self.head_result != Some(Timing::TooLate) {
            OnSensorResult::Consumed
        } else {
            OnSensorResult::TooLate
        }
    }

    fn judge(&mut self, simulator: &TouchSensorStates, current_time: f64) {
        assert!(self.result.is_none());
        let curr_state = simulator.sensor_is_on(self.sensor);
        if current_time < self.appear_time + JUDGE_DATA.judge_hold_head_s() {
            self.prev_state = Some(curr_state);
            self.prev_time = Some(current_time);
            return;
        }
        assert!(self.prev_state.is_some());
        let prev_state = self.prev_state.unwrap();
        let prev_time = self.prev_time.unwrap();
        if !prev_state
            && self.appear_time + JUDGE_DATA.judge_hold_head_s()
                <= self.tail_time - JUDGE_DATA.judge_hold_tail_s()
        {
            self.release_time += f64::max(
                f64::min(
                    current_time,
                    self.tail_time - JUDGE_DATA.judge_hold_tail_s(),
                ) - f64::max(prev_time, self.appear_time + JUDGE_DATA.judge_hold_head_s()),
                0.0,
            );
        }
        self.prev_state = Some(curr_state);
        self.prev_time = Some(current_time);
        if self.is_too_late(current_time) {
            self.result = Some(JUDGE_DATA.get_hold_timing(
                self.tail_time - self.appear_time,
                self.release_time,
                self.head_result.unwrap_or(Timing::TooLate),
                false,
            ));
        }
    }

    fn get_judge_result(&self) -> Option<Timing> {
        self.result
    }
}
