use crate::insn::{Key, TouchSensor};
use crate::transform::{
    NormalizedHoldParams, NormalizedSlideParams, NormalizedSlideSegment,
    NormalizedSlideSegmentParams, NormalizedSlideTrack, NormalizedTapParams,
    NormalizedTouchHoldParams, NormalizedTouchParams,
};

use super::{NormalizedNote, NormalizedSlideSegmentShape};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Transformer {
    pub rotation: u8,
    pub flip: bool,
    pub vertical_flip: bool,
}

pub trait Transformable {
    fn transform(&self, transformer: Transformer) -> Self;
}

impl Transformable for Key {
    fn transform(&self, transformer: Transformer) -> Self {
        let mut index = (self.index() + transformer.rotation) % 8;
        if transformer.flip {
            index = 7 - index;
        }
        if transformer.vertical_flip {
            index = (11 - index) % 8;
        }
        index.try_into().unwrap()
    }
}

impl Transformable for TouchSensor {
    fn transform(&self, transformer: Transformer) -> Self {
        let group = self.group();
        if group == 'C' {
            return *self;
        }
        let mut index = (self.index().unwrap() + transformer.rotation) % 8;
        if transformer.flip {
            index = match group {
                'A' | 'B' => 7 - index,
                'D' | 'E' => (8 - index) % 8,
                _ => unreachable!(),
            };
        }
        if transformer.vertical_flip {
            index = match group {
                'A' | 'B' => (11 - index) % 8,
                'D' | 'E' => (12 - index) % 8,
                _ => unreachable!(),
            };
        }
        (group, Some(index)).try_into().unwrap()
    }
}

impl Transformable for NormalizedTapParams {
    fn transform(&self, transformer: Transformer) -> Self {
        NormalizedTapParams {
            key: self.key.transform(transformer),
        }
    }
}

impl Transformable for NormalizedTouchParams {
    fn transform(&self, transformer: Transformer) -> Self {
        NormalizedTouchParams {
            sensor: self.sensor.transform(transformer),
        }
    }
}

impl Transformable for NormalizedHoldParams {
    fn transform(&self, transformer: Transformer) -> Self {
        NormalizedHoldParams {
            key: self.key.transform(transformer),
        }
    }
}

impl Transformable for NormalizedTouchHoldParams {
    fn transform(&self, transformer: Transformer) -> Self {
        NormalizedTouchHoldParams {
            sensor: self.sensor.transform(transformer),
        }
    }
}

impl Transformable for NormalizedSlideSegment {
    fn transform(&self, transformer: Transformer) -> Self {
        let shape = if transformer.flip ^ transformer.vertical_flip {
            match self.shape() {
                NormalizedSlideSegmentShape::Straight => NormalizedSlideSegmentShape::Straight,
                NormalizedSlideSegmentShape::CircleL => NormalizedSlideSegmentShape::CircleR,
                NormalizedSlideSegmentShape::CircleR => NormalizedSlideSegmentShape::CircleL,
                NormalizedSlideSegmentShape::CurveL => NormalizedSlideSegmentShape::CurveR,
                NormalizedSlideSegmentShape::CurveR => NormalizedSlideSegmentShape::CurveL,
                NormalizedSlideSegmentShape::ThunderL => NormalizedSlideSegmentShape::ThunderR,
                NormalizedSlideSegmentShape::ThunderR => NormalizedSlideSegmentShape::ThunderL,
                NormalizedSlideSegmentShape::Corner => NormalizedSlideSegmentShape::Corner,
                NormalizedSlideSegmentShape::BendL => NormalizedSlideSegmentShape::BendR,
                NormalizedSlideSegmentShape::BendR => NormalizedSlideSegmentShape::BendL,
                NormalizedSlideSegmentShape::SkipL => NormalizedSlideSegmentShape::SkipR,
                NormalizedSlideSegmentShape::SkipR => NormalizedSlideSegmentShape::SkipL,
                NormalizedSlideSegmentShape::Fan => NormalizedSlideSegmentShape::Fan,
            }
        } else {
            self.shape()
        };
        NormalizedSlideSegment::new(
            shape,
            NormalizedSlideSegmentParams {
                start: self.params().start.transform(transformer),
                destination: self.params().destination.transform(transformer),
            },
        )
    }
}

impl Transformable for NormalizedSlideTrack {
    fn transform(&self, transformer: Transformer) -> Self {
        NormalizedSlideTrack {
            segments: self
                .segments
                .iter()
                .map(|group| group.transform(transformer))
                .collect(),
        }
    }
}

impl Transformable for NormalizedSlideParams {
    fn transform(&self, transformer: Transformer) -> Self {
        NormalizedSlideParams {
            start: self.start.transform(transformer),
            tracks: self
                .tracks
                .iter()
                .map(|track| track.transform(transformer))
                .collect(),
        }
    }
}

pub fn transform_note(note: &NormalizedNote, transformer: Transformer) -> NormalizedNote {
    match note {
        NormalizedNote::Tap(params) => NormalizedNote::Tap(params.transform(transformer)),
        NormalizedNote::Touch(params) => NormalizedNote::Touch(params.transform(transformer)),
        NormalizedNote::Hold(params) => NormalizedNote::Hold(params.transform(transformer)),
        NormalizedNote::TouchHold(params) => {
            NormalizedNote::TouchHold(params.transform(transformer))
        }
        NormalizedNote::Slide(params) => NormalizedNote::Slide(params.transform(transformer)),
    }
}
