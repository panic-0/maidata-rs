use super::note::{FanSlide, Hold, Note, Slide, Tap, Touch, TouchHold};
use super::slide_data_getter::SLIDE_DATA_GETTER;
use crate::materialize::{
    MaterializedSlideSegment, MaterializedSlideTrack, Note as MaterializedNote,
};
use crate::transform::{
    NormalizedSlideSegment, NormalizedSlideSegmentParams, NormalizedSlideSegmentShape,
    NormalizedSlideTrack,
};

/// Convert a materialized note into a judge note.
pub fn convert_note(note: MaterializedNote) -> Result<Note, &'static str> {
    match note {
        MaterializedNote::Bpm(_) => todo!(""),
        MaterializedNote::Tap(t) => Ok(Note::Tap(Tap::new(t.key, t.ts, t.is_break, t.is_ex))),
        MaterializedNote::Touch(t) => Ok(Note::Touch(Touch::new(t.sensor, t.ts))),
        MaterializedNote::SlideTrack(s) => {
            if s.segments
                .iter()
                .any(|segment| segment.shape == NormalizedSlideSegmentShape::Fan)
            {
                Ok(Note::FanSlide(convert_fan_slide(s)?))
            } else {
                Ok(Note::Slide(convert_slide(s)?))
            }
        }
        MaterializedNote::Hold(h) => Ok(Note::Hold(Hold::new(
            h.key,
            h.ts,
            h.ts + h.dur,
            h.is_break,
            h.is_ex,
        ))),
        MaterializedNote::TouchHold(h) => Ok(Note::TouchHold(TouchHold::new(
            h.sensor,
            h.ts,
            h.ts + h.dur,
        ))),
    }
}

fn convert_slide(m: MaterializedSlideTrack) -> Result<Slide, &'static str> {
    if m.segments
        .iter()
        .any(|segment| segment.shape == NormalizedSlideSegmentShape::Fan)
    {
        return Err("Fan Slide is not supported");
    }
    let normalized_track = NormalizedSlideTrack {
        segments: m
            .segments
            .iter()
            .map(materialized_to_normalized_slide_segment)
            .collect::<Vec<_>>(),
    };

    // Why check head???
    let head_segment = normalized_track.segments[0];
    let tail_segment = normalized_track.segments.last().unwrap();
    let head_is_thunder = head_segment.shape() == NormalizedSlideSegmentShape::ThunderL
        || head_segment.shape() == NormalizedSlideSegmentShape::ThunderR;
    let mut distance =
        (tail_segment.params().destination.index() + 8 - head_segment.params().start.index()) % 8;
    if head_segment.shape() == NormalizedSlideSegmentShape::ThunderR {
        distance = (8 - distance) % 8;
    }

    Ok(Slide::new(
        SLIDE_DATA_GETTER
            .get_path(&normalized_track)
            .ok_or("Slide path not found")?,
        m.ts,
        m.start_ts + m.dur,
        m.is_break,
        head_is_thunder && (1..=4).contains(&distance),
        head_is_thunder && distance == 4,
    ))
}

fn convert_fan_slide(m: MaterializedSlideTrack) -> Result<FanSlide, &'static str> {
    if m.segments.len() != 1 {
        return Err("Fan Slide must have only one group and one segment");
    }
    let segment = m.segments[0];
    if segment.shape != NormalizedSlideSegmentShape::Fan {
        return Err("Fan Slide must have a fan segment");
    }
    let sub_slides = [
        MaterializedSlideSegment {
            start: ((segment.start.index() + 7) % 8).try_into().unwrap(),
            destination: segment.destination,
            shape: NormalizedSlideSegmentShape::Fan,
        },
        MaterializedSlideSegment {
            start: segment.start,
            destination: segment.destination,
            shape: NormalizedSlideSegmentShape::Fan,
        },
        MaterializedSlideSegment {
            start: ((segment.start.index() + 1) % 8).try_into().unwrap(),
            destination: segment.destination,
            shape: NormalizedSlideSegmentShape::Fan,
        },
    ]
    .iter()
    .map(|seg| {
        SLIDE_DATA_GETTER
            .get_path_by_segment(&materialized_to_normalized_slide_segment(seg))
            .ok_or("Slide path not found")
            .map(|path| Slide::from_path(path, m.ts, m.start_ts + m.dur, m.is_break))
    })
    .collect::<Result<Vec<_>, _>>()?;

    Ok(FanSlide::new(sub_slides))
}

fn materialized_to_normalized_slide_segment(
    segment: &MaterializedSlideSegment,
) -> NormalizedSlideSegment {
    NormalizedSlideSegment::new(
        segment.shape,
        NormalizedSlideSegmentParams {
            start: segment.start,
            destination: segment.destination,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::insn::{Key, TouchSensor};
    use crate::materialize::{
        MaterializedHold, MaterializedTap, MaterializedTapShape, MaterializedTouch,
        MaterializedTouchHold,
    };

    fn key(i: u8) -> Key {
        Key::new(i).unwrap()
    }

    #[test]
    fn test_convert_tap() {
        let m = MaterializedTap {
            ts: 1.0,
            key: key(0),
            shape: MaterializedTapShape::Ring,
            is_break: false,
            is_ex: false,
            is_each: false,
        };
        let note = convert_note(MaterializedNote::Tap(m)).unwrap();
        match note {
            Note::Tap(tap) => {
                assert_eq!(tap.appear_time, 1.0);
                assert!(!tap._is_break);
                assert!(!tap._is_ex);
            }
            _ => panic!("expected Tap"),
        }
    }

    #[test]
    fn test_convert_ex_tap() {
        let m = MaterializedTap {
            ts: 0.5,
            key: key(3),
            shape: MaterializedTapShape::Star,
            is_break: true,
            is_ex: true,
            is_each: false,
        };
        let note = convert_note(MaterializedNote::Tap(m)).unwrap();
        match note {
            Note::Tap(tap) => {
                assert_eq!(tap.appear_time, 0.5);
                assert!(tap._is_break);
                assert!(tap._is_ex);
            }
            _ => panic!("expected Tap"),
        }
    }

    #[test]
    fn test_convert_touch() {
        let sensor = TouchSensor::new('C', None).unwrap();
        let m = MaterializedTouch {
            ts: 2.0,
            sensor,
            is_each: true,
        };
        let note = convert_note(MaterializedNote::Touch(m)).unwrap();
        match note {
            Note::Touch(touch) => {
                assert_eq!(touch.appear_time, 2.0);
                assert_eq!(touch.sensor, sensor);
            }
            _ => panic!("expected Touch"),
        }
    }

    #[test]
    fn test_convert_hold() {
        let m = MaterializedHold {
            ts: 1.0,
            dur: 0.5,
            key: key(2),
            is_break: false,
            is_ex: false,
            is_each: false,
        };
        let note = convert_note(MaterializedNote::Hold(m)).unwrap();
        match note {
            Note::Hold(hold) => {
                assert_eq!(hold.appear_time, 1.0);
                assert_eq!(hold.tail_time, 1.5);
            }
            _ => panic!("expected Hold"),
        }
    }

    #[test]
    fn test_convert_touch_hold() {
        let sensor = TouchSensor::new('A', Some(4)).unwrap();
        let m = MaterializedTouchHold {
            ts: 0.0,
            dur: 1.0,
            sensor,
            is_each: false,
        };
        let note = convert_note(MaterializedNote::TouchHold(m)).unwrap();
        match note {
            Note::TouchHold(th) => {
                assert_eq!(th.appear_time, 0.0);
                assert_eq!(th.tail_time, 1.0);
                assert_eq!(th.sensor, sensor);
            }
            _ => panic!("expected TouchHold"),
        }
    }

    #[test]
    fn test_convert_slide_straight() {
        let m = MaterializedSlideTrack {
            ts: 0.0,
            start_ts: 0.5,
            dur: 1.0,
            segments: vec![MaterializedSlideSegment {
                start: key(0),
                destination: key(4),
                shape: NormalizedSlideSegmentShape::Straight,
            }],
            is_break: false,
            is_sudden: false,
            is_each: false,
        };
        let note = convert_note(MaterializedNote::SlideTrack(m)).unwrap();
        match note {
            Note::Slide(slide) => {
                assert_eq!(slide.appear_time, 0.0);
                assert_eq!(slide.tail_time, 1.5);
                assert!(!slide._is_break);
                assert!(!slide.path.is_empty());
            }
            _ => panic!("expected Slide"),
        }
    }

    #[test]
    fn test_convert_slide_fan() {
        let m = MaterializedSlideTrack {
            ts: 0.0,
            start_ts: 0.5,
            dur: 1.0,
            segments: vec![MaterializedSlideSegment {
                start: key(0),
                destination: key(4),
                shape: NormalizedSlideSegmentShape::Fan,
            }],
            is_break: false,
            is_sudden: false,
            is_each: false,
        };
        let note = convert_note(MaterializedNote::SlideTrack(m)).unwrap();
        match note {
            Note::FanSlide(fan) => {
                assert_eq!(fan.sub_slides.len(), 3);
            }
            _ => panic!("expected FanSlide"),
        }
    }

    #[test]
    fn test_convert_slide_fan_requires_single_segment() {
        let m = MaterializedSlideTrack {
            ts: 0.0,
            start_ts: 0.5,
            dur: 1.0,
            segments: vec![
                MaterializedSlideSegment {
                    start: key(0),
                    destination: key(4),
                    shape: NormalizedSlideSegmentShape::Fan,
                },
                MaterializedSlideSegment {
                    start: key(4),
                    destination: key(0),
                    shape: NormalizedSlideSegmentShape::Straight,
                },
            ],
            is_break: false,
            is_sudden: false,
            is_each: false,
        };
        assert!(convert_note(MaterializedNote::SlideTrack(m)).is_err());
    }

    #[test]
    fn test_convert_slide_multi_segment() {
        let m = MaterializedSlideTrack {
            ts: 0.0,
            start_ts: 0.5,
            dur: 1.5,
            segments: vec![
                MaterializedSlideSegment {
                    start: key(0),
                    destination: key(2),
                    shape: NormalizedSlideSegmentShape::Straight,
                },
                MaterializedSlideSegment {
                    start: key(2),
                    destination: key(4),
                    shape: NormalizedSlideSegmentShape::Straight,
                },
            ],
            is_break: true,
            is_sudden: false,
            is_each: false,
        };
        let note = convert_note(MaterializedNote::SlideTrack(m)).unwrap();
        match note {
            Note::Slide(slide) => {
                assert_eq!(slide.appear_time, 0.0);
                assert_eq!(slide.tail_time, 2.0);
                assert!(slide._is_break);
            }
            _ => panic!("expected Slide"),
        }
    }
}
