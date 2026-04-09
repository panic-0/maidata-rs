use maidata::materialize::Note as MaterializedNote;
use serde::{Deserialize, Serialize};

fn is_false(b: &bool) -> bool {
    !b
}

// ── Rounding helper ──────────────────────────────────────────────────────

pub fn r4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}

// ── Output types ─────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FlatNote {
    Tap {
        ts: f64,
        key: u8,
        x: f64,
        y: f64,
        #[serde(default, skip_serializing_if = "is_false")]
        is_star: bool,
    },
    Touch {
        ts: f64,
        sensor: String,
        x: f64,
        y: f64,
    },
    Hold {
        ts: f64,
        dur: f64,
        key: u8,
        x: f64,
        y: f64,
    },
    TouchHold {
        ts: f64,
        dur: f64,
        sensor: String,
        x: f64,
        y: f64,
    },
    SlideTrack {
        ts: f64,
        end_ts: f64,
        x: f64,
        y: f64,
        segments: Vec<SlideSegment>,
        judge_areas: Vec<SlideJudgeArea>,
    },
}

#[derive(Serialize)]
pub struct ChartEntry {
    pub song_id: String,
    pub title: String,
    pub level: Option<f64>,
    pub difficulty: String,
    pub notes: Vec<FlatNote>,
    pub slides: Vec<SlideTrack>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<Vec<MaterializedNote>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SlideTrack {
    pub start_ts: f64,
    pub end_ts: f64,
    pub segments: Vec<SlideSegment>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SlideSegment {
    pub shape: String,   // "straight", "circle_l", "curve_r", etc.
    pub start: u8,       // key index
    pub destination: u8, // key index
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SlideJudgeArea {
    pub ts: f64,
    pub exit_ts: f64,
    pub sensors: Vec<String>,
    pub x: f64,
    pub y: f64,
}

// ── Note helpers ─────────────────────────────────────────────────────────

pub fn note_time_range(note: &FlatNote) -> (f64, f64) {
    match note {
        FlatNote::Tap { ts, .. } | FlatNote::Touch { ts, .. } => (*ts, *ts),
        FlatNote::Hold { ts, dur, .. } | FlatNote::TouchHold { ts, dur, .. } => (*ts, ts + dur),
        FlatNote::SlideTrack { ts, end_ts, .. } => (*ts, *end_ts),
    }
}

/// Round f64-valued fields in the JSON (ts, dur, exit_ts, x, y, level) to 4 decimal places.
pub fn round_json_values(v: &mut serde_json::Value) {
    let float_keys = ["ts", "dur", "exit_ts", "x", "y", "level"];
    match v {
        serde_json::Value::Object(obj) => {
            for (k, val) in obj.iter_mut() {
                if k == "raw" {
                    continue;
                }
                if float_keys.contains(&k.as_str()) {
                    if let Some(f) = val.as_f64() {
                        *val = serde_json::Value::Number(
                            serde_json::Number::from_f64(r4(f))
                                .unwrap_or_else(|| serde_json::Number::from_f64(f).unwrap()),
                        );
                    }
                } else {
                    round_json_values(val);
                }
            }
        }
        serde_json::Value::Array(arr) => arr.iter_mut().for_each(round_json_values),
        _ => {}
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_r4() {
        assert_eq!(r4(1.23456), 1.2346);
        assert_eq!(r4(1.23454), 1.2345);
        assert_eq!(r4(0.0), 0.0);
        assert_eq!(r4(182.0), 182.0);
        assert_eq!(r4(1.99999), 2.0);
    }

    #[test]
    fn test_note_time_range_instant() {
        let tap = FlatNote::Tap {
            ts: 1.5,
            key: 0,
            x: 0.0,
            y: 0.0,
            is_star: false,
        };
        assert_eq!(note_time_range(&tap), (1.5, 1.5));

        let touch = FlatNote::Touch {
            ts: 2.0,
            sensor: "E1".into(),
            x: 0.0,
            y: 0.0,
        };
        assert_eq!(note_time_range(&touch), (2.0, 2.0));
    }

    #[test]
    fn test_note_time_range_hold() {
        let hold = FlatNote::Hold {
            ts: 1.0,
            dur: 0.5,
            key: 0,
            x: 0.0,
            y: 0.0,
        };
        assert_eq!(note_time_range(&hold), (1.0, 1.5));
    }

    #[test]
    fn test_note_time_range_slide() {
        let slide = FlatNote::SlideTrack {
            ts: 1.0,
            end_ts: 1.3,
            x: 0.0,
            y: 0.0,
            segments: vec![],
            judge_areas: vec![],
        };
        assert_eq!(note_time_range(&slide), (1.0, 1.3));
    }

    #[test]
    fn test_round_json_values() {
        let mut v = serde_json::json!({
            "ts": 1.23456789,
            "key": 3,
            "x": 182.5,
            "level": 13.7
        });
        round_json_values(&mut v);
        assert_eq!(v["ts"], 1.2346);
        assert_eq!(v["key"], 3, "integer fields should not be changed");
        assert_eq!(v["x"], 182.5);
        assert_eq!(v["level"], 13.7);
    }
}
