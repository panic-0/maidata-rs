pub mod app;
pub mod container;
pub mod diag;
pub mod heatmap;
pub mod insn;
pub mod judge;
pub mod maidata_file;
pub mod materialize;
mod parser;
pub mod span;
pub mod transform;
#[macro_use]
extern crate enum_map;

pub use diag::{PError, PWarning, State};
pub use maidata_file::{AssociatedBeatmapData, BeatmapData, Maidata};
pub use span::{Span, Sp, VecSp};

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
pub enum Difficulty {
    /// The EASY difficulty.
    Easy = 1,
    /// The BASIC difficulty.
    Basic = 2,
    /// The ADVANCED difficulty.
    Advanced = 3,
    /// The EXPERT difficulty.
    Expert = 4,
    /// The MASTER difficulty.
    Master = 5,
    /// The Re:MASTER difficulty.
    ReMaster = 6,
    /// The ORIGINAL difficulty, previously called mai:EDIT in 2simai.
    Original = 7,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Level {
    /// The "Lv.X" form.
    Normal(u8),
    /// The "Lv.X+" form.
    Plus(u8),
    /// The special "Lv.<any char>" form.
    Char(char),
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Level::*;
        match self {
            Normal(lv) => write!(f, "{lv}")?,
            Plus(lv) => write!(f, "{lv}+")?,
            Char(lv) => write!(f, "{lv}")?,
        }

        Ok(())
    }
}
