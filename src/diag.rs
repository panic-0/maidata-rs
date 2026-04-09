use crate::insn::NoteType;
use crate::span::{Sp, Span};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PWarning {
    DuplicateModifier(char, NoteType),
    MultipleSlideTrackGroups,
    MissingSlideStartKey,
}

impl std::fmt::Display for PWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PWarning::DuplicateModifier(c, t) => {
                write!(f, "duplicate `{c}` modifier in {t} instruction")
            }
            PWarning::MultipleSlideTrackGroups => {
                write!(f, "multiple slide track groups in slide instruction")
            }
            PWarning::MissingSlideStartKey => {
                write!(f, "missing start key in slide instruction")
            }
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "message", rename_all = "snake_case")]
pub enum PError {
    UnknownChar(char),

    MissingBefore {
        token: String,
        context: String,
    },
    MissingAfter {
        token: String,
        context: String,
    },
    MissingBetween {
        token: String,
        open: String,
        close: String,
    },

    MissingBeatCount, // [divisor:num]
    MissingDuration(NoteType),
    MissingNote,
    MissingSlideStartKey,
    MissingSlideTrack,
    MissingSlideDestinationKey,
    MissingSlideAngleDestinationKey,

    InvalidBpm(String),
    InvalidBeatDivisor(String),
    InvalidDuration(String),
    InvalidSlideStopTime(String),
    InvalidSlideTrack(String),

    DuplicateShapeModifier(NoteType),
    IncompatibleDurations(NoteType),
}

impl std::fmt::Display for PError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PError::UnknownChar(c) => write!(f, "unknown character `{c}`"),

            PError::MissingBefore { token, context } => {
                write!(f, "missing {token} before {context}")
            }
            PError::MissingAfter { token, context } => {
                write!(f, "missing {token} after {context}")
            }
            PError::MissingBetween { token, open, close } => {
                write!(f, "missing {token} between {open} and {close}")
            }

            PError::MissingBeatCount => write!(f, "missing beat count"),
            PError::MissingDuration(t) => write!(f, "missing {t} duration"),
            PError::MissingNote => write!(f, "missing note"),
            PError::MissingSlideStartKey => write!(f, "missing slide start key"),
            PError::MissingSlideTrack => write!(f, "missing slide track"),
            PError::MissingSlideDestinationKey => {
                write!(f, "missing slide destination key")
            }
            PError::MissingSlideAngleDestinationKey => {
                write!(f, "missing destination key in V-shaped slide")
            }

            PError::InvalidBpm(s) => write!(f, "invalid bpm {s}"),
            PError::InvalidBeatDivisor(s) => write!(f, "invalid beat divisor `{s}`"),
            PError::InvalidDuration(s) => write!(f, "invalid duration `{s}`"),
            PError::InvalidSlideStopTime(s) => write!(f, "invalid slide stop time {s}"),
            PError::InvalidSlideTrack(s) => write!(f, "invalid slide track `{s}`"),

            PError::DuplicateShapeModifier(t) => {
                write!(f, "duplicate {t} shape modifier")
            }
            PError::IncompatibleDurations(t) => write!(f, "incompatible {t} durations"),
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
