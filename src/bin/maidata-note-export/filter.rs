use crate::types::{note_time_range, FlatNote};

/// Check that at no moment more than 2 notes overlap.
/// Returns the first time where >2 notes are active, if any.
pub fn find_simultaneous_violation(notes: &[FlatNote]) -> Option<f64> {
    if notes.is_empty() {
        return None;
    }

    // Collect all unique event times
    let mut times: Vec<f64> = Vec::new();
    for note in notes {
        let (ts, exit_ts) = note_time_range(note);
        times.push(ts);
        if exit_ts > ts {
            times.push(exit_ts);
        }
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    times.dedup();

    for t in &times {
        let count = notes
            .iter()
            .filter(|n| {
                let (ts, exit_ts) = note_time_range(n);
                if ts == exit_ts {
                    ts == *t
                } else {
                    *t >= ts && *t < exit_ts
                }
            })
            .count();
        if count > 2 {
            return Some(*t);
        }
    }
    None
}

#[cfg(test)]
pub fn has_too_many_simultaneous(notes: &[FlatNote]) -> bool {
    find_simultaneous_violation(notes).is_some()
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simultaneous_empty() {
        assert!(!has_too_many_simultaneous(&[]));
    }

    #[test]
    fn test_simultaneous_two_taps_same_time() {
        let notes = vec![
            FlatNote::Tap {
                ts: 1.0,
                key: 0,
                x: 0.0,
                y: 0.0,
                is_star: false,
            },
            FlatNote::Tap {
                ts: 1.0,
                key: 4,
                x: 0.0,
                y: 0.0,
                is_star: false,
            },
        ];
        assert!(!has_too_many_simultaneous(&notes));
    }

    #[test]
    fn test_simultaneous_three_taps_same_time() {
        let notes = vec![
            FlatNote::Tap {
                ts: 1.0,
                key: 0,
                x: 0.0,
                y: 0.0,
                is_star: false,
            },
            FlatNote::Tap {
                ts: 1.0,
                key: 4,
                x: 0.0,
                y: 0.0,
                is_star: false,
            },
            FlatNote::Tap {
                ts: 1.0,
                key: 6,
                x: 0.0,
                y: 0.0,
                is_star: false,
            },
        ];
        assert!(has_too_many_simultaneous(&notes));
    }

    #[test]
    fn test_simultaneous_taps_different_times() {
        let notes = vec![
            FlatNote::Tap {
                ts: 1.0,
                key: 0,
                x: 0.0,
                y: 0.0,
                is_star: false,
            },
            FlatNote::Tap {
                ts: 2.0,
                key: 4,
                x: 0.0,
                y: 0.0,
                is_star: false,
            },
            FlatNote::Tap {
                ts: 3.0,
                key: 6,
                x: 0.0,
                y: 0.0,
                is_star: false,
            },
        ];
        assert!(!has_too_many_simultaneous(&notes));
    }

    #[test]
    fn test_simultaneous_hold_overlaps_tap() {
        let notes = vec![
            FlatNote::Hold {
                ts: 1.0,
                dur: 2.0,
                key: 0,
                x: 0.0,
                y: 0.0,
            },
            FlatNote::Tap {
                ts: 1.5,
                key: 4,
                x: 0.0,
                y: 0.0,
                is_star: false,
            },
        ];
        assert!(!has_too_many_simultaneous(&notes));
    }

    #[test]
    fn test_simultaneous_hold_overlaps_two_taps() {
        let notes = vec![
            FlatNote::Hold {
                ts: 1.0,
                dur: 2.0,
                key: 0,
                x: 0.0,
                y: 0.0,
            },
            FlatNote::Tap {
                ts: 1.5,
                key: 4,
                x: 0.0,
                y: 0.0,
                is_star: false,
            },
            FlatNote::Tap {
                ts: 1.5,
                key: 6,
                x: 0.0,
                y: 0.0,
                is_star: false,
            },
        ];
        assert!(has_too_many_simultaneous(&notes));
    }

    #[test]
    fn test_simultaneous_slide_does_not_overlap_later_tap() {
        let notes = vec![
            FlatNote::SlideTrack {
                ts: 1.0,
                end_ts: 1.2,
                x: 0.0,
                y: 0.0,
                segments: vec![],
                judge_areas: vec![],
            },
            FlatNote::Tap {
                ts: 1.5,
                key: 4,
                x: 0.0,
                y: 0.0,
                is_star: false,
            },
        ];
        assert!(!has_too_many_simultaneous(&notes));
    }
}
