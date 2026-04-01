use super::{JudgeNote, JudgeType, Timing, TouchSensorStates, JUDGE_DATA};
use crate::insn::TouchSensor;

#[derive(Clone, Debug)]
pub struct Slide {
    pub path: Vec<Vec<TouchSensor>>,
    pub appear_time: f64,
    pub tail_time: f64,
    pub _is_break: bool,

    judge_check_sensor_1: bool,
    judge_check_sensor_3: bool,

    judge_type: JudgeType,
    pub judge_index: usize,
    pub judge_is_on: bool,
    pub judge_sub_sensor: Option<TouchSensor>,

    result: Option<Timing>,
}

impl Slide {
    pub fn new(
        path: Vec<Vec<TouchSensor>>,
        appear_time: f64,
        tail_time: f64,
        is_break: bool,
        judge_check_sensor_1: bool,
        judge_check_sensor_3: bool,
    ) -> Self {
        Self {
            path,
            appear_time,
            tail_time,
            _is_break: is_break,
            judge_check_sensor_1,
            judge_check_sensor_3,
            judge_type: JudgeType::Slide,
            judge_index: 0,
            judge_is_on: false,
            judge_sub_sensor: None,
            result: None,
        }
    }

    pub(crate) fn from_path(
        path: Vec<Vec<TouchSensor>>,
        appear_time: f64,
        tail_time: f64,
        is_break: bool,
    ) -> Self {
        Self::new(path, appear_time, tail_time, is_break, false, false)
    }
}

impl Slide {
    fn check_sensor(&mut self, simulator: &TouchSensorStates, index: usize, is_on: bool) -> bool {
        if index >= self.path.len() {
            return false;
        }
        if !is_on {
            for sensor in self.path[index].iter() {
                if simulator.sensor_is_on(*sensor) {
                    self.judge_index = index;
                    self.judge_is_on = true;
                    self.judge_sub_sensor = Some(*sensor);
                    if self.judge_index == self.path.len() - 1 {
                        self.judge_index = self.path.len();
                    }
                    return true;
                }
            }
        } else {
            assert!(index == self.judge_index && self.judge_is_on);
            if !simulator.sensor_is_on(self.judge_sub_sensor.unwrap()) {
                self.judge_index += 1;
                self.judge_is_on = false;
                self.judge_sub_sensor = None;
                return true;
            }
        }
        false
    }

    pub fn is_next_sensor_check(&self) -> bool {
        if self.judge_is_on {
            return true;
        }
        // TODO: Check if this is correct
        if self.judge_check_sensor_1 && self.judge_index == 1 {
            return false;
        }
        if self.judge_check_sensor_3 && self.judge_index == 3 {
            return false;
        }
        self.path.len() > 3 || self.judge_index + 1 != self.path.len() - 1
    }

    fn compute_judge_result(&self, current_time: f64) -> Option<Timing> {
        if self.judge_index < self.path.len() {
            return None;
        }
        // TODO: Fix Slide Critical timing (depends on slide wait time)
        let mut result = JUDGE_DATA.get_timing(self.judge_type, current_time - self.tail_time);
        if result == Timing::TooFast {
            result = Timing::FastGood;
        }
        Some(result)
    }
}

impl JudgeNote for Slide {
    fn get_start_time(&self) -> f64 {
        // TODO: check if this is correct
        self.appear_time + JUDGE_DATA.judge_param(JudgeType::Tap).as_ref()[Timing::FastGood]
    }

    fn get_end_time(&self) -> f64 {
        self.tail_time + JUDGE_DATA.judge_param(self.judge_type).as_ref()[Timing::LateGood]
    }

    fn judge(&mut self, simulator: &TouchSensorStates, current_time: f64) {
        assert!(self.result.is_none());
        // Do not judge if too late
        if self.is_too_late(current_time) {
            assert!(self.judge_index < self.path.len());
            self.result = Some(if self.judge_index + 1 == self.path.len() {
                Timing::LateGood
            } else {
                Timing::TooLate
            });
            return;
        }

        loop {
            let mut changed = self.check_sensor(simulator, self.judge_index, self.judge_is_on);
            if !changed && self.is_next_sensor_check() {
                changed = self.check_sensor(simulator, self.judge_index + 1, false);
            }
            if !changed || self.judge_index == self.path.len() {
                break;
            }
        }
        if self.judge_index == self.path.len() {
            self.result = self.compute_judge_result(current_time);
            assert!(self.result.is_some());
        }
    }

    fn get_judge_result(&self) -> Option<Timing> {
        self.result
    }
}
