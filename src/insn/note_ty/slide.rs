use super::*;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum SlideStopTimeSpec {
    Bpm(f64),
    Seconds(f64),
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum SlideDuration {
    Simple(Duration),
    Custom(SlideStopTimeSpec, Duration),
}

impl SlideDuration {
    pub fn valid(&self) -> bool {
        match self {
            SlideDuration::Simple(_) => true,
            SlideDuration::Custom(spec, dur) => match spec {
                SlideStopTimeSpec::Bpm(_) => matches!(dur, Duration::Seconds(_)),
                SlideStopTimeSpec::Seconds(_) => true,
            },
        }
    }
}

impl std::ops::Add<SlideDuration> for SlideDuration {
    type Output = Option<SlideDuration>;

    fn add(self, rhs: SlideDuration) -> Self::Output {
        match (self, rhs) {
            (SlideDuration::Simple(lhs), SlideDuration::Simple(rhs)) => {
                (lhs + rhs).map(SlideDuration::Simple)
            }
            (SlideDuration::Simple(lhs), SlideDuration::Custom(rhs_spec, rhs)) => {
                let result = SlideDuration::Custom(rhs_spec, (lhs + rhs)?);
                if result.valid() {
                    Some(result)
                } else {
                    None
                }
            }
            (SlideDuration::Custom(lhs_spec, lhs), SlideDuration::Simple(rhs)) => {
                let result = SlideDuration::Custom(lhs_spec, (lhs + rhs)?);
                if result.valid() {
                    Some(result)
                } else {
                    None
                }
            }
            (SlideDuration::Custom(lhs_spec, lhs), SlideDuration::Custom(rhs_spec, rhs)) => {
                // TODO: Check if this is correct
                if lhs_spec != rhs_spec {
                    return None;
                }
                let result = SlideDuration::Custom(lhs_spec, (lhs + rhs)?);
                if result.valid() {
                    Some(result)
                } else {
                    None
                }
            }
        }
    }
}

impl std::fmt::Display for SlideDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.valid() {
            // panic!("Invalid slide duration spec: {:?} {:?}", spec, duration);
            return Err(std::fmt::Error);
        }
        match self {
            Self::Simple(duration) => write!(f, "{duration}"),
            Self::Custom(spec, duration) => match spec {
                SlideStopTimeSpec::Bpm(bpm) => {
                    if let Duration::Seconds(seconds) = duration {
                        write!(f, "{bpm}#{seconds}")
                    } else {
                        unreachable!()
                    }
                }
                SlideStopTimeSpec::Seconds(seconds) => match duration {
                    Duration::Seconds(dur) => write!(f, "{seconds}##{dur}"),
                    _ => write!(f, "{seconds}##{duration}"),
                },
            },
        }
    }
}

impl SlideDuration {
    pub fn slide_duration(&self) -> Duration {
        match self {
            SlideDuration::Simple(l) => *l,
            SlideDuration::Custom(_, l) => *l,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct SlideParams {
    pub start: TapParams,
    pub tracks: Vec<SlideTrack>,
}

impl std::fmt::Display for SlideParams {
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

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct SlideTrackModifier {
    pub is_break: bool,
    pub is_sudden: bool,
}

impl std::fmt::Display for SlideTrackModifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_break {
            write!(f, "b")?;
        }
        Ok(())
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct SlideTrack {
    pub segments: Vec<SlideSegment>,
    pub dur: SlideDuration,
    pub modifier: SlideTrackModifier,
}

impl std::fmt::Display for SlideTrack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for segment in self.segments.iter() {
            write!(f, "{segment}")?;
        }
        write!(f, "[{}]{}", self.dur, self.modifier)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum SlideSegment {
    Line(SlideSegmentParams),
    Arc(SlideSegmentParams), // ???
    CircumferenceLeft(SlideSegmentParams),
    CircumferenceRight(SlideSegmentParams),
    V(SlideSegmentParams),
    P(SlideSegmentParams),
    Q(SlideSegmentParams),
    S(SlideSegmentParams),
    Z(SlideSegmentParams),
    Pp(SlideSegmentParams),
    Qq(SlideSegmentParams),
    Angle(SlideSegmentParams),
    Spread(SlideSegmentParams),
}

impl std::fmt::Display for SlideSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Line(params) => write!(f, "-{}", params.destination),
            Self::Arc(params) => write!(f, "^{}", params.destination),
            Self::CircumferenceLeft(params) => write!(f, "<{}", params.destination),
            Self::CircumferenceRight(params) => write!(f, ">{}", params.destination),
            Self::V(params) => write!(f, "v{}", params.destination),
            Self::P(params) => write!(f, "p{}", params.destination),
            Self::Q(params) => write!(f, "q{}", params.destination),
            Self::S(params) => write!(f, "s{}", params.destination),
            Self::Z(params) => write!(f, "z{}", params.destination),
            Self::Pp(params) => write!(f, "pp{}", params.destination),
            Self::Qq(params) => write!(f, "qq{}", params.destination),
            Self::Angle(params) => write!(f, "V{}{}", params.interim.unwrap(), params.destination),
            Self::Spread(params) => write!(f, "w{}", params.destination),
        }
    }
}

impl SlideSegment {
    pub fn shape(&self) -> SlideSegmentShape {
        match self {
            Self::Line(_) => SlideSegmentShape::Line,
            Self::Arc(_) => SlideSegmentShape::Arc,
            Self::CircumferenceLeft(_) => SlideSegmentShape::CircumferenceLeft,
            Self::CircumferenceRight(_) => SlideSegmentShape::CircumferenceRight,
            Self::V(_) => SlideSegmentShape::V,
            Self::P(_) => SlideSegmentShape::P,
            Self::Q(_) => SlideSegmentShape::Q,
            Self::S(_) => SlideSegmentShape::S,
            Self::Z(_) => SlideSegmentShape::Z,
            Self::Pp(_) => SlideSegmentShape::Pp,
            Self::Qq(_) => SlideSegmentShape::Qq,
            Self::Angle(_) => SlideSegmentShape::Angle,
            Self::Spread(_) => SlideSegmentShape::Spread,
        }
    }

    pub fn params(&self) -> &SlideSegmentParams {
        match self {
            SlideSegment::Line(p) => p,
            SlideSegment::Arc(p) => p,
            SlideSegment::CircumferenceLeft(p) => p,
            SlideSegment::CircumferenceRight(p) => p,
            SlideSegment::V(p) => p,
            SlideSegment::P(p) => p,
            SlideSegment::Q(p) => p,
            SlideSegment::S(p) => p,
            SlideSegment::Z(p) => p,
            SlideSegment::Pp(p) => p,
            SlideSegment::Qq(p) => p,
            SlideSegment::Angle(p) => p,
            SlideSegment::Spread(p) => p,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum SlideSegmentShape {
    Line,
    Arc,
    CircumferenceLeft,
    CircumferenceRight,
    V,
    P,
    Q,
    S,
    Z,
    Pp,
    Qq,
    Angle,
    Spread,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct SlideSegmentParams {
    pub destination: Key,
    pub interim: Option<Key>,
}
