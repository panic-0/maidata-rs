use crate::insn::{Key, TouchSensor};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NormalizedTapParams {
    pub key: Key,
}

impl std::fmt::Display for NormalizedTapParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.key)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NormalizedTouchParams {
    pub sensor: TouchSensor,
}

impl std::fmt::Display for NormalizedTouchParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.sensor)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NormalizedHoldParams {
    pub key: Key,
}

impl std::fmt::Display for NormalizedHoldParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}h", self.key)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NormalizedTouchHoldParams {
    pub sensor: TouchSensor,
}

impl std::fmt::Display for NormalizedTouchHoldParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}h", self.sensor)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NormalizedSlideParams {
    pub start: NormalizedTapParams,
    pub tracks: Vec<NormalizedSlideTrack>,
}

impl std::fmt::Display for NormalizedSlideParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}",
            self.start,
            self.tracks
                .iter()
                .map(|x| format!("{x}"))
                .collect::<Vec<_>>()
                .join("*")
        )
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NormalizedSlideTrack {
    pub segments: Vec<NormalizedSlideSegment>,
}

impl std::fmt::Display for NormalizedSlideTrack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.segments
                .iter()
                .map(|x| format!("{x}"))
                .collect::<Vec<_>>()
                .join("")
        )
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NormalizedSlideSegment {
    shape: NormalizedSlideSegmentShape,
    params: NormalizedSlideSegmentParams,
}

impl NormalizedSlideSegment {
    pub fn new(shape: NormalizedSlideSegmentShape, params: NormalizedSlideSegmentParams) -> Self {
        Self { shape, params }
    }

    pub fn shape(&self) -> NormalizedSlideSegmentShape {
        self.shape
    }

    pub fn params(&self) -> &NormalizedSlideSegmentParams {
        &self.params
    }
}

impl std::fmt::Display for NormalizedSlideSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.shape() {
            NormalizedSlideSegmentShape::Straight => write!(f, "-{}", self.params.destination),
            NormalizedSlideSegmentShape::CircleL => {
                let end_index = self.params.destination.index();
                let upper = !(2..6).contains(&end_index);
                write!(
                    f,
                    "{}{}",
                    if upper { '<' } else { '>' },
                    self.params.destination
                )
            }
            NormalizedSlideSegmentShape::CircleR => {
                let end_index = self.params.destination.index();
                let upper = !(2..6).contains(&end_index);
                write!(
                    f,
                    "{}{}",
                    if upper { '>' } else { '<' },
                    self.params.destination
                )
            }
            NormalizedSlideSegmentShape::CurveL => {
                write!(f, "p{}", self.params.destination)
            }
            NormalizedSlideSegmentShape::CurveR => {
                write!(f, "q{}", self.params.destination)
            }
            NormalizedSlideSegmentShape::ThunderL => {
                write!(f, "s{}", self.params.destination)
            }
            NormalizedSlideSegmentShape::ThunderR => {
                write!(f, "z{}", self.params.destination)
            }
            NormalizedSlideSegmentShape::Corner => write!(f, "v{}", self.params.destination),
            NormalizedSlideSegmentShape::BendL => {
                write!(f, "qq{}", self.params.destination)
            }
            NormalizedSlideSegmentShape::BendR => {
                write!(f, "pp{}", self.params.destination)
            }
            NormalizedSlideSegmentShape::SkipL => {
                let interim = Key::new((self.params.start.index() + 6) % 8).unwrap();
                write!(f, "V{}{}", interim, self.params.destination)
            }
            NormalizedSlideSegmentShape::SkipR => {
                let interim = Key::new((self.params.start.index() + 2) % 8).unwrap();
                write!(f, "V{}{}", interim, self.params.destination)
            }
            NormalizedSlideSegmentShape::Fan => write!(f, "w{}", self.params.destination),
        }
    }
}

#[derive(
    Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Enum, Serialize, Deserialize,
)]
pub enum NormalizedSlideSegmentShape {
    Straight, // -
    CircleL,  // counterclockwise
    CircleR,  // clockwise
    CurveL,   // p
    CurveR,   // q
    ThunderL, // s
    ThunderR, // z
    Corner,   // v
    BendL,    // qq
    BendR,    // pp
    SkipL,    // 1V7
    SkipR,    // 1V3
    Fan,      // w
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NormalizedSlideSegmentParams {
    pub start: Key,
    pub destination: Key,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum NormalizedNote {
    Tap(NormalizedTapParams),
    Touch(NormalizedTouchParams),
    Hold(NormalizedHoldParams),
    TouchHold(NormalizedTouchHoldParams),
    Slide(NormalizedSlideParams),
}

impl std::fmt::Display for NormalizedNote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NormalizedNote::Tap(params) => write!(f, "{params}"),
            NormalizedNote::Touch(params) => write!(f, "{params}"),
            NormalizedNote::Hold(params) => write!(f, "{params}"),
            NormalizedNote::TouchHold(params) => write!(f, "{params}"),
            NormalizedNote::Slide(params) => write!(f, "{params}"),
        }
    }
}
