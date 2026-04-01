#[derive(Copy, Clone, PartialEq, Debug)]
pub struct BpmParams {
    pub new_bpm: f64,
}

impl std::fmt::Display for BpmParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.new_bpm)
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum BeatDivisorParams {
    NewDivisor(u32),
    NewAbsoluteDuration(f64),
}

impl std::fmt::Display for BeatDivisorParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NewDivisor(x) => write!(f, "{x}"),
            Self::NewAbsoluteDuration(x) => write!(f, "#{x}"),
        }
    }
}
