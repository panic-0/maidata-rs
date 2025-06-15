pub mod hold;
pub mod slide;
pub mod tap;
pub mod touch;
pub mod touch_hold;

pub use hold::*;
pub use slide::*;
pub use tap::*;
pub use touch::*;
pub use touch_hold::*;

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Key {
    index: u8,
}

impl Serialize for Key {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{}", self))
    }
}

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D>(deserializer: D) -> Result<Key, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let x = s.parse().map_err(serde::de::Error::custom)?;
        Key::new(x).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.index + 1)
    }
}

impl Key {
    pub fn new(x: u8) -> Result<Self, KeyParseError> {
        match x {
            0..=7 => Ok(Key { index: x }),
            _ => Err(KeyParseError::InvalidKey(x)),
        }
    }

    pub fn index(&self) -> u8 {
        self.index
    }
}

#[derive(Clone, Debug)]
pub enum KeyParseError {
    InvalidKey(u8),
}

impl std::fmt::Display for KeyParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyParseError::InvalidKey(x) => write!(f, "invalid key: {}", x),
        }
    }
}

impl std::convert::TryFrom<u8> for Key {
    type Error = KeyParseError;

    fn try_from(x: u8) -> Result<Self, Self::Error> {
        Key::new(x)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct TouchSensor {
    group: char,
    index: Option<u8>,
}

impl Serialize for TouchSensor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{}", self))
    }
}

impl<'de> Deserialize<'de> for TouchSensor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let mut chars = s.chars();
        if s.len() == 1 {
            let group = chars.next().unwrap();
            TouchSensor::new(group, None).map_err(serde::de::Error::custom)
        } else if s.len() == 2 {
            let group = chars.next().unwrap();
            let index = chars.next().unwrap().to_digit(10).map(|x| x as u8);
            TouchSensor::new(group, index).map_err(serde::de::Error::custom)
        } else {
            Err(serde::de::Error::custom("invalid touch sensor"))
        }
    }
}

impl std::fmt::Display for TouchSensor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.group)?;
        if let Some(index) = self.index {
            write!(f, "{}", index + 1)?;
        }
        Ok(())
    }
}

impl TouchSensor {
    pub fn new(group: char, index: Option<u8>) -> Result<Self, TouchSensorParseError> {
        if let ('A' | 'B' | 'D' | 'E', Some(0..=7)) = (group, index) {
            return Ok(TouchSensor { group, index });
        }
        if let ('C', None) = (group, index) {
            return Ok(TouchSensor { group, index });
        }
        Err(TouchSensorParseError::InvalidTouchSensor(group, index))
    }

    pub const fn new_unchecked(group: char, index: Option<u8>) -> Self {
        TouchSensor { group, index }
    }

    pub fn group(&self) -> char {
        self.group
    }
    pub fn index(&self) -> Option<u8> {
        self.index
    }
}

#[derive(Clone, Debug)]
pub enum TouchSensorParseError {
    InvalidTouchSensor(char, Option<u8>),
}

impl std::fmt::Display for TouchSensorParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TouchSensorParseError::InvalidTouchSensor(group, index) => {
                write!(
                    f,
                    "invalid touch sensor: {}{}",
                    group,
                    index.map_or(String::new(), |x| x.to_string())
                )
            }
        }
    }
}

impl std::convert::TryFrom<(char, Option<u8>)> for TouchSensor {
    type Error = TouchSensorParseError;

    fn try_from(x: (char, Option<u8>)) -> Result<Self, Self::Error> {
        TouchSensor::new(x.0, x.1)
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Duration {
    NumBeats(NumBeatsParams),
    Seconds(f64),
}

impl Default for Duration {
    fn default() -> Self {
        Duration::NumBeats(NumBeatsParams {
            bpm: None,
            divisor: 1,
            num: 0,
        })
    }
}

impl Duration {
    pub fn bpm(&self) -> Option<f64> {
        match self {
            Self::NumBeats(params) => params.bpm,
            Self::Seconds(_) => None,
        }
    }
}

impl std::ops::Add<Duration> for Duration {
    type Output = Option<Duration>;

    fn add(self, rhs: Duration) -> Self::Output {
        match (self, rhs) {
            (Self::NumBeats(lhs), Self::NumBeats(rhs)) => {
                if lhs.bpm.is_some() && rhs.bpm.is_some() && lhs.bpm != rhs.bpm {
                    return None;
                }
                let gcd = |mut a: u32, mut b: u32| {
                    while b != 0 {
                        let t = b;
                        b = a % b;
                        a = t;
                    }
                    a
                };
                let divisor = lhs.divisor / gcd(lhs.divisor, rhs.divisor) * rhs.divisor;
                let num = lhs.num * (divisor / lhs.divisor) + rhs.num * (divisor / rhs.divisor);
                let gcd = gcd(num, divisor);
                Some(Self::NumBeats(NumBeatsParams {
                    bpm: lhs.bpm.or(rhs.bpm),
                    divisor: divisor / gcd,
                    num: num / gcd,
                }))
            }
            (Self::Seconds(lhs), Self::Seconds(rhs)) => Some(Self::Seconds(lhs + rhs)),
            _ => None,
        }
    }
}

impl std::fmt::Display for Duration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NumBeats(params) => write!(f, "{}", params),
            Self::Seconds(seconds) => write!(f, "#{}", seconds),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct NumBeatsParams {
    pub bpm: Option<f64>,
    pub divisor: u32,
    pub num: u32,
}

impl std::fmt::Display for NumBeatsParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(bpm) = self.bpm {
            write!(f, "{}#", bpm)?;
        }
        write!(f, "{}:{}", self.divisor, self.num)
    }
}
