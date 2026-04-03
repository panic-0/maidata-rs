use super::duration::{t_dur_spec, t_dur_spec_absolute, t_dur_spec_num_beats};
use super::*;

fn t_slide_dur_spec_simple(s: NomSpan) -> PResult<Option<SlideDuration>> {
    let (s, dur) = t_dur_spec(s)?;

    Ok((s, dur.map(SlideDuration::Simple)))
}

fn t_slide_dur_spec_custom_bpm(s: NomSpan) -> PResult<Option<SlideDuration>> {
    use nom::character::complete::char;
    use nom::number::complete::double;

    let (s, start_loc) = nom_locate::position(s)?;
    let (s, bpm) = ws(double)(s)?;
    let (s, _) = ws(char('#'))(s)?;
    let (s, dur) = ws(double)(s)?;
    let (s, end_loc) = nom_locate::position(s)?;

    if !bpm.is_finite() || bpm <= 0.0 {
        s.extra.borrow_mut().add_error(
            PError::InvalidBpm(bpm.to_string()),
            (start_loc, end_loc).into(),
        );
        return Ok((s, None));
    }
    if !dur.is_finite() || dur <= 0.0 {
        s.extra.borrow_mut().add_error(
            PError::InvalidDuration(format!("#{dur}")),
            (start_loc, end_loc).into(),
        );
        return Ok((s, None));
    }
    Ok((
        s,
        Some(SlideDuration::Custom(
            SlideStopTimeSpec::Bpm(bpm),
            Duration::Seconds(dur),
        )),
    ))
}

fn t_slide_dur_spec_custom_seconds(s: NomSpan) -> PResult<Option<SlideDuration>> {
    use nom::branch::alt;
    use nom::bytes::complete::tag;
    use nom::character::complete::char;
    use nom::number::complete::double;
    use nom::sequence::preceded;

    let (s, start_loc) = nom_locate::position(s)?;
    let (s, sec) = ws(double)(s)?;
    let (s, dur) = ws(alt((
        preceded(tag("##"), ws(t_dur_spec_num_beats)),
        preceded(char('#'), t_dur_spec_absolute), // like "##0.5", no need to use ws
    )))(s)?;
    let (s, end_loc) = nom_locate::position(s)?;

    // following cases are possible in this combinator:
    //
    // - `[160#2.0]` -> stop time=(as in BPM 160) dur=2.0s
    // - `[3##1.5]` -> stop time=(absolute 3s) dur=1.5s
    // - `[3##4:1]` -> stop time=(absolute 3s) dur=4:1
    // - `[3.0##160#4:1]` -> stop time=(absolute 3s) dur=4:1(as in BPM 160)

    // sec can be 0.0
    if !sec.is_finite() || sec < 0.0 {
        s.extra.borrow_mut().add_error(
            PError::InvalidSlideStopTime(sec.to_string()),
            (start_loc, end_loc).into(),
        );
        return Ok((s, None));
    }
    Ok((
        s,
        dur.map(|dur| SlideDuration::Custom(SlideStopTimeSpec::Seconds(sec), dur)),
    ))
}

// NOTE: must run after t_slide_dur_simple
fn t_slide_dur_spec_custom(s: NomSpan) -> PResult<Option<SlideDuration>> {
    // TODO: following cases are possible in this combinator:
    //
    // - `[160#2.0]` -> stop time=(as in BPM 160) dur=2.0s
    // - `[3##1.5]` -> stop time=(absolute 3s) dur=1.5s
    // - `[3##4:1]` -> stop time=(absolute 3s) dur=4:1
    // - `[3.0##160#4:1]` -> stop time=(absolute 3s) dur=4:1(as in BPM 160)
    nom::branch::alt((t_slide_dur_spec_custom_seconds, t_slide_dur_spec_custom_bpm))(s)
}

pub fn t_slide_dur(s: NomSpan) -> PResult<Option<SlideDuration>> {
    use nom::branch::alt;

    let (s, dur) = expect_ws_delimited(
        ws(alt((t_slide_dur_spec_simple, t_slide_dur_spec_custom))),
        "slide duration",
        "[",
        "]",
    )(s)?;

    Ok((s, dur.flatten()))
}

// FxE[dur]
// covers everything except FVRE
macro_rules! define_slide_segment {
    (@ $fn_name: ident, $recog: expr, $variant: ident) => {
        #[allow(unused_imports)]
        pub fn $fn_name(s: NomSpan) -> PResult<Option<SlideSegment>> {
            use nom::character::complete::char;
            use nom::bytes::complete::tag;

            let (s, _) = $recog(s)?;
            let (s, destination) = ws(t_key).expect(PError::MissingSlideDestinationKey)(s)?;

            Ok((
                s,
                destination.map(|destination| SlideSegment::$variant(SlideSegmentParams {
                    destination,
                    interim: None,
                })),
            ))
        }
    };

    ($fn_name: ident, char $ch: expr, $variant: ident) => {
        define_slide_segment!(@ $fn_name, char($ch), $variant);
    };

    ($fn_name: ident, tag $tag: expr, $variant: ident) => {
        define_slide_segment!(@ $fn_name, tag($tag), $variant);
    };
}

define_slide_segment!(t_slide_segment_line, char '-', Line);
define_slide_segment!(t_slide_segment_arc, char '^', Arc);
define_slide_segment!(t_slide_segment_circ_left, char '<', CircumferenceLeft);
define_slide_segment!(t_slide_segment_circ_right, char '>', CircumferenceRight);
define_slide_segment!(t_slide_segment_v, char 'v', V);
define_slide_segment!(t_slide_segment_p, char 'p', P);
define_slide_segment!(t_slide_segment_q, char 'q', Q);
define_slide_segment!(t_slide_segment_s, char 's', S);
define_slide_segment!(t_slide_segment_z, char 'z', Z);
define_slide_segment!(t_slide_segment_pp, tag "pp", Pp);
define_slide_segment!(t_slide_segment_qq, tag "qq", Qq);
define_slide_segment!(t_slide_segment_spread, char 'w', Spread);

pub fn t_slide_segment_angle(s: NomSpan) -> PResult<Option<SlideSegment>> {
    use nom::character::complete::char;

    let (s, _) = char('V')(s)?;
    let (s, interim) = ws(t_key).expect(PError::MissingSlideDestinationKey)(s)?;
    let (s, destination) = ws(t_key).expect(PError::MissingSlideDestinationKey)(s)?;

    if interim.is_some() && destination.is_none() {
        let (_, loc) = nom_locate::position(s)?;
        s.extra
            .borrow_mut()
            .add_error(PError::MissingSlideAngleDestinationKey, (loc, loc).into());
    }

    Ok((
        s,
        destination.and_then(|destination| {
            interim.map(|interim| {
                SlideSegment::Angle(SlideSegmentParams {
                    destination,
                    interim: Some(interim),
                })
            })
        }),
    ))
}

pub fn t_slide_segment(s: NomSpan) -> PResult<Option<SlideSegment>> {
    nom::branch::alt((
        t_slide_segment_line,
        t_slide_segment_arc,
        t_slide_segment_circ_left,
        t_slide_segment_circ_right,
        t_slide_segment_v,
        // NOTE: pp and qq must be before p and q
        t_slide_segment_pp,
        t_slide_segment_qq,
        t_slide_segment_p,
        t_slide_segment_q,
        t_slide_segment_s,
        t_slide_segment_z,
        t_slide_segment_angle,
        t_slide_segment_spread,
    ))(s)
}

pub fn t_slide_track_modifier(
    s: NomSpan,
    mut modifier: SlideTrackModifier,
) -> PResult<SlideTrackModifier> {
    use nom::character::complete::one_of;
    use nom::multi::many0;

    let (s, start_loc) = nom_locate::position(s)?;
    let (s, variants) = many0(ws(one_of("b")))(s)?;
    let (s, end_loc) = nom_locate::position(s)?;
    let span: Span = (start_loc, end_loc).into();
    for x in &variants {
        match *x {
            'b' => set_flag_or_warn(s.extra, &mut modifier.is_break, 'b', NoteType::Slide, span),
            _ => unreachable!(),
        }
    }

    Ok((s, modifier))
}

// TODO: refactor
pub fn t_slide_segment_group(
    s: NomSpan,
) -> PResult<(Vec<SlideSegment>, Option<SlideDuration>, SlideTrackModifier)> {
    // TODO: track with different speed
    let (s, segments) = ws_list1(t_slide_segment)(s)?;
    let segments = segments.into_iter().flatten().collect::<Vec<_>>();
    // TODO: warn if have modifier before dur
    let (s, modifier) = t_slide_track_modifier(s, SlideTrackModifier::default())?;
    let (s, dur) = ws(t_slide_dur).expect(PError::MissingDuration(NoteType::Slide))(s)?;
    let (s, modifier) = t_slide_track_modifier(s, modifier)?;

    Ok((s, (segments, dur.flatten(), modifier)))
}

pub fn validate_slide_track(start_key: Key, track: &SlideTrack) -> bool {
    use crate::transform::normalize::normalize_slide_track;

    // TODO: split
    normalize_slide_track(start_key, track).is_some()
}

pub fn t_slide_track(s: NomSpan, start_key: Option<Key>) -> PResult<Option<SlideTrack>> {
    // TODO: track with different speed
    let (s, start_loc) = nom_locate::position(s)?;
    let (s, groups) = ws_list1(t_slide_segment_group)(s)?;
    let (s, end_loc) = nom_locate::position(s)?;
    // it is slightly different from the official syntax
    let modifier = groups
        .iter()
        .fold(SlideTrackModifier::default(), |mut acc, (_, _, x)| {
            if acc.is_break && x.is_break {
                s.extra.borrow_mut().add_warning(
                    PWarning::DuplicateModifier('b', NoteType::Slide),
                    (start_loc, end_loc).into(),
                );
            }
            acc.is_break |= x.is_break;
            acc
        });
    if groups.len() > 1 {
        // TODO: message
        s.extra.borrow_mut().add_warning(
            PWarning::MultipleSlideTrackGroups,
            (start_loc, end_loc).into(),
        );
    }
    let dur = groups.iter().fold(None, |acc, (_, dur, _)| {
        if let Some(acc) = acc {
            if let Some(dur) = dur {
                let result: Option<SlideDuration> = acc + *dur;
                if result.is_none() {
                    s.extra.borrow_mut().add_error(
                        PError::IncompatibleDurations(NoteType::Slide),
                        (start_loc, end_loc).into(),
                    );
                    // TODO
                    return Some(*dur);
                }
                result
            } else {
                Some(acc)
            }
        } else {
            *dur
        }
    });
    let segments = groups
        .into_iter()
        .flat_map(|(segments, _, _)| segments)
        .collect::<Vec<_>>();
    if segments.is_empty() || dur.is_none() {
        // no extra error
        return Ok((s, None));
    }

    let track = SlideTrack {
        segments,
        dur: dur.unwrap(),
        modifier,
    };

    if let Some(start_key) = start_key {
        if !validate_slide_track(start_key, &track) {
            s.extra.borrow_mut().add_error(
                PError::InvalidSlideTrack(format!("{start_key}{track}")),
                (start_loc, end_loc).into(),
                // "invalid slide track instruction".to_string(),
            );
            return Ok((s, None));
        }
    }

    Ok((s, Some(track)))
}

pub fn t_slide_sep_track(s: NomSpan, start_key: Option<Key>) -> PResult<Option<SlideTrack>> {
    use nom::character::complete::char;

    let (s, _) = char('*')(s)?;
    let (s, track) = ws(move |s| t_slide_track(s, start_key)).expect(PError::MissingSlideTrack)(s)?;

    Ok((s, track.flatten()))
}

/// return (modifier, is_sudden)
pub fn t_slide_head_modifier_str(s: NomSpan) -> PResult<Vec<NomSpan>> {
    use nom::branch::alt;
    use nom::bytes::complete::tag;
    use nom::multi::many0;

    let (s, variants) = many0(ws(alt((tag("b"), tag("x"), tag("@"), tag("?"), tag("!")))))(s)?;

    Ok((s, variants))
}

struct SlideHeadModifier {
    tap_modifier: TapModifier,
    is_sudden: bool,
}

fn parse_slide_head_modifier(
    s: NomSpan,
    modifier_str: &[NomSpan],
    span: Span,
) -> SlideHeadModifier {
    let mut tap_modifier = TapModifier::default();
    let mut is_sudden = false;
    for x in modifier_str {
        match *x.fragment() {
            "b" => set_flag_or_warn(s.extra, &mut tap_modifier.is_break, 'b', NoteType::Slide, span),
            "x" => set_flag_or_warn(s.extra, &mut tap_modifier.is_ex, 'x', NoteType::Slide, span),
            "!" => set_flag_or_warn(s.extra, &mut is_sudden, '!', NoteType::Slide, span),
            _ => (),
        }
        let shape = match *x.fragment() {
            "@" => Some(TapShape::Ring),
            "?" => Some(TapShape::Invalid),
            "!" => Some(TapShape::Invalid),
            _ => None,
        };
        if let Some(shape) = shape {
            if tap_modifier.shape.is_some() {
                s.extra
                    .borrow_mut()
                    .add_error(PError::DuplicateShapeModifier(NoteType::Slide), span);
            } else {
                tap_modifier.shape = Some(shape);
            }
        }
    }
    SlideHeadModifier {
        tap_modifier,
        is_sudden,
    }
}

pub fn t_slide(s: NomSpan) -> PResult<Option<SpRawNoteInsn>> {
    use nom::combinator::opt;
    use nom::multi::many0;

    let (s, start_loc) = nom_locate::position(s)?;
    let (s, start_key) = ws(opt(t_key))(s)?;
    let (s, start_modifier_str) = t_slide_head_modifier_str(s)?;
    let (s, first_track) = ws(move |s| t_slide_track(s, start_key))(s)?;
    let (s, rest_track) = many0(move |s| t_slide_sep_track(s, start_key))(s)?;
    let (s, end_loc) = nom_locate::position(s)?;

    if start_key.is_none() {
        s.extra
            .borrow_mut()
            .add_error(PError::MissingSlideStartKey, (start_loc, end_loc).into());
    }

    let span: Span = (start_loc, end_loc).into();
    let head = parse_slide_head_modifier(s, &start_modifier_str, span);
    let start = start_key.map(|start_key| TapParams {
        key: start_key,
        modifier: head.tap_modifier,
    });

    let tracks = {
        let mut tmp = Vec::with_capacity(rest_track.len() + 1);
        tmp.push(first_track);
        tmp.extend(rest_track);
        tmp.into_iter()
            .flatten()
            .map(|mut x| {
                assert!(!x.modifier.is_sudden);
                x.modifier.is_sudden = head.is_sudden;
                Ok(x)
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    if tracks.is_empty() {
        return Ok((s, None));
    }

    Ok((
        s,
        start.map(|start| RawNoteInsn::Slide(SlideParams { start, tracks }).with_span(span)),
    ))
}

#[cfg(test)]
mod tests {
    use super::super::tests::{test_parser_err, test_parser_ok, test_parser_warn};
    use super::*;

    #[test]
    fn test_t_slide_dur() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            test_parser_ok(t_slide_dur, "[ 4 : 3 ]", " ,").unwrap(),
            SlideDuration::Simple(Duration::NumBeats(NumBeatsParams {
                bpm: None,
                divisor: 4,
                num: 3
            }))
        );

        assert_eq!(
            test_parser_ok(t_slide_dur, "[#2.5]", " ,").unwrap(),
            SlideDuration::Simple(Duration::Seconds(2.5))
        );

        assert_eq!(
            test_parser_ok(t_slide_dur, "[ 120.0 #4: 1]", " ,").unwrap(),
            SlideDuration::Simple(Duration::NumBeats(NumBeatsParams {
                bpm: Some(120.0),
                divisor: 4,
                num: 1
            }))
        );

        assert_eq!(
            test_parser_ok(t_slide_dur, "[ 160 #2.0 ]", " ,").unwrap(),
            SlideDuration::Custom(SlideStopTimeSpec::Bpm(160.0), Duration::Seconds(2.0))
        );
        test_parser_err(t_slide_dur, "[0#2.0]");
        test_parser_err(t_slide_dur, "[inf#2.0]");
        // [160##2.0] is valid, it is in the next group

        assert_eq!(
            test_parser_ok(t_slide_dur, "[ 3.0## 1.5 ]", " ,").unwrap(),
            SlideDuration::Custom(SlideStopTimeSpec::Seconds(3.0), Duration::Seconds(1.5))
        );
        assert_eq!(
            test_parser_ok(t_slide_dur, "[0##2.0]", "").unwrap(),
            SlideDuration::Custom(SlideStopTimeSpec::Seconds(0.0), Duration::Seconds(2.0))
        );
        test_parser_err(t_slide_dur, "[nan##2.0]");
        test_parser_err(t_slide_dur, "[3.0# #1.5]");
        test_parser_err(t_slide_dur, "[3.0###1.5]");
        // [3.0#1.5] is valid, it is in the previous group

        assert_eq!(
            test_parser_ok(t_slide_dur, "[ 3.0## 4 : 1 ]", " ,").unwrap(),
            SlideDuration::Custom(
                SlideStopTimeSpec::Seconds(3.0),
                Duration::NumBeats(NumBeatsParams {
                    bpm: None,
                    divisor: 4,
                    num: 1
                })
            )
        );
        assert_eq!(
            test_parser_ok(t_slide_dur, "[0##4:1]", "").unwrap(),
            SlideDuration::Custom(
                SlideStopTimeSpec::Seconds(0.0),
                Duration::NumBeats(NumBeatsParams {
                    bpm: None,
                    divisor: 4,
                    num: 1
                })
            )
        );
        test_parser_err(t_slide_dur, "[-1##4:1]");
        test_parser_err(t_slide_dur, "[3.0# #4:1]");
        test_parser_err(t_slide_dur, "[3.0###4:1]");

        assert_eq!(
            test_parser_ok(t_slide_dur, "[ 3.0 ##160 #4 : 1 ]", " ,").unwrap(),
            SlideDuration::Custom(
                SlideStopTimeSpec::Seconds(3.0),
                Duration::NumBeats(NumBeatsParams {
                    bpm: Some(160.0),
                    divisor: 4,
                    num: 1
                })
            )
        );
        assert_eq!(
            test_parser_ok(t_slide_dur, "[0##160#4:1]", "").unwrap(),
            SlideDuration::Custom(
                SlideStopTimeSpec::Seconds(0.0),
                Duration::NumBeats(NumBeatsParams {
                    bpm: Some(160.0),
                    divisor: 4,
                    num: 1
                })
            )
        );
        test_parser_err(t_slide_dur, "[inf##160#4:1]");
        test_parser_err(t_slide_dur, "[2##0.0#4:1]");
        test_parser_err(t_slide_dur, "[3.0# #160#4:1]");
        test_parser_err(t_slide_dur, "[3.0###160#4:1]");
        test_parser_err(t_slide_dur, "[3.0##160##4:1]");

        test_parser_err(t_slide_dur, "[3.0#160##4:1]");
        test_parser_err(t_slide_dur, "[3.0#160#1.0]");
        test_parser_err(t_slide_dur, "[3.0#160##1.0]");
        test_parser_err(t_slide_dur, "[3.0#4:1##160.0]");
        test_parser_err(t_slide_dur, "[4:1#3.0##160.0]");

        Ok(())
    }

    #[test]
    fn test_t_slide_angle_missing_destination() -> Result<(), Box<dyn std::error::Error>> {
        // V with only one key: interim=5 parsed but no destination key
        // should produce both MissingSlideAngleDestination warning and MissingSlideDestinationKey error
        let state = std::cell::RefCell::new(crate::State::default());
        let s = NomSpan::new_extra("1V5-8[1:1],", &state);
        let _ = t_slide(s).unwrap();
        let state = state.into_inner();
        assert!(state.has_errors());
        assert!(state.errors.iter().any(|w| matches!(**w, crate::diag::PError::MissingSlideAngleDestinationKey)));
        Ok(())
    }
}
