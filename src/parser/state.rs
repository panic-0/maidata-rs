use super::{Sp, Span};
use crate::insn::NoteType;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PWarning {
    DuplicateModifier(char, NoteType),
    MultipleSlideTrackGroups,
    MissingSlideStartKey,

    MissingDuration(NoteType),
}

impl std::fmt::Display for PWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PWarning::DuplicateModifier(c, t) => {
                write!(f, "duplicate `{}` modifier in {} instruction", c, t)
            }
            PWarning::MultipleSlideTrackGroups => {
                write!(f, "multiple slide track groups in slide instruction")
            }
            PWarning::MissingSlideStartKey => {
                write!(f, "missing start key in slide instruction")
            }

            PWarning::MissingDuration(t) => write!(f, "missing {} duration", t),
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "message", rename_all = "snake_case")]
pub enum PError {
    UnknownChar(char),

    // TODO: rename
    ExpectedBefore {
        expected: String,
        location: String,
    },
    // TODO: rename
    ExpectedAfter {
        expected: String,
        location: String,
    },
    // TODO: rename
    ExpectedBetween {
        expected: String,
        previous: String,
        next: String,
    },

    // TODO: rename
    MissingBeatsNum, // [divisor:num]
    MissingDuration(NoteType),
    MissingNote,
    MissingSlideStartKey,
    MissingSlideTrack,
    MissingSlideDestinationKey,

    InvalidBpm(String),
    InvalidBeatDivisor(String),
    InvalidDuration(String),
    InvalidSlideStopTime(String),
    InvalidSlideTrack(String),

    DuplicateShapeModifier(NoteType),
    // TODO: rename
    DurationMismatch(NoteType), // [4:1] + [#2.0]
}

impl std::fmt::Display for PError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PError::UnknownChar(c) => write!(f, "unknown character `{}`", c),

            PError::ExpectedBefore {
                expected,
                location: after,
            } => {
                write!(f, "expected {} before {}", expected, after)
            }
            PError::ExpectedAfter {
                expected,
                location: before,
            } => {
                write!(f, "expected {} after {}", expected, before)
            }
            PError::ExpectedBetween {
                expected,
                previous: before,
                next: after,
            } => write!(f, "expected {} between {} and {}", expected, before, after),

            PError::MissingBeatsNum => write!(f, "missing number of beats"),
            PError::MissingDuration(t) => write!(f, "missing {} duration", t),
            PError::MissingNote => write!(f, "missing note"),
            PError::MissingSlideStartKey => write!(f, "missing slide start key"),
            PError::MissingSlideTrack => write!(f, "missing slide track"),
            PError::MissingSlideDestinationKey => {
                write!(f, "missing slide destination key")
            }

            PError::InvalidBpm(s) => write!(f, "invalid bpm {}", s),
            PError::InvalidBeatDivisor(s) => write!(f, "invalid beat divisor `{}`", s),
            PError::InvalidDuration(s) => write!(f, "invalid duration `{}`", s),
            PError::InvalidSlideStopTime(s) => write!(f, "invalid slide stop time {}", s),
            PError::InvalidSlideTrack(s) => write!(f, "invalid slide track `{}`", s),

            PError::DuplicateShapeModifier(t) => {
                write!(f, "duplicate {} shape modifier", t)
            }
            PError::DurationMismatch(t) => write!(f, "{} duration mismatch", t),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub warnings: Vec<Sp<PWarning>>,
    pub errors: Vec<Sp<PError>>,
}

impl State {
    pub fn add_warning(&mut self, warning: PWarning, span: Span) {
        self.warnings.push(Sp::new(warning, span));
    }

    pub fn add_error(&mut self, error: PError, span: Span) {
        self.errors.push(Sp::new(error, span));
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn has_messages(&self) -> bool {
        self.has_warnings() || self.has_errors()
    }
}
