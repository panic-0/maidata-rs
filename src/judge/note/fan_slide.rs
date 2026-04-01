use super::slide::Slide;
use super::{JudgeNote, Timing, TouchSensorStates};

// TODO: move to slide.rs
#[derive(Clone, Debug)]
pub struct FanSlide {
    pub sub_slides: Vec<Slide>,
}

impl FanSlide {
    pub fn new(sub_slides: Vec<Slide>) -> Self {
        Self { sub_slides }
    }
}

impl JudgeNote for FanSlide {
    fn get_start_time(&self) -> f64 {
        self.sub_slides
            .iter()
            .map(|slide| slide.get_start_time())
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap()
    }

    fn get_end_time(&self) -> f64 {
        self.sub_slides
            .iter()
            .map(|slide| slide.get_end_time())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap()
    }

    fn judge(&mut self, getter: &TouchSensorStates, current_time: f64) {
        for slide in &mut self.sub_slides {
            slide.judge(getter, current_time);
        }
    }

    fn get_judge_result(&self) -> Option<Timing> {
        if self
            .sub_slides
            .iter()
            .any(|slide| slide.get_judge_result().is_none())
        {
            return None;
        }
        self.sub_slides
            .iter()
            .map(|slide| slide.get_judge_result().unwrap())
            .max()
    }
}
