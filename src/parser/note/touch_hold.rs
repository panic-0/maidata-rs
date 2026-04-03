use super::duration::t_dur;
use super::*;

pub fn t_touch_hold_modifier_str(s: NomSpan) -> PResult<Vec<char>> {
    use nom::character::complete::one_of;
    use nom::multi::many0;

    let (s, variants) = many0(ws(one_of("f")))(s)?;

    Ok((s, variants))
}

pub fn t_touch_hold(s: NomSpan) -> PResult<Option<SpRawNoteInsn>> {
    use nom::character::complete::char;

    let (s, start_loc) = nom_locate::position(s)?;
    let (s, sensor) = t_touch_sensor(s)?;
    let (s, pre_mods) = t_touch_hold_modifier_str(s)?;
    let (s, _) = ws(char('h'))(s)?;
    let (s, post_mods) = t_touch_hold_modifier_str(s)?;
    let (s, dur) = ws(t_dur).expect(PError::MissingDuration(NoteType::TouchHold))(s)?;
    let (s, end_loc) = nom_locate::position(s)?;

    let mut modifier = TouchHoldModifier::default();
    let span: Span = (start_loc, end_loc).into();
    for x in pre_mods.iter().chain(&post_mods) {
        match *x {
            'f' => set_flag_or_warn(s.extra, &mut modifier.is_firework, 'f', NoteType::TouchHold, span),
            _ => unreachable!(),
        }
    }

    let span = (start_loc, end_loc);
    Ok((
        s,
        dur.flatten().map(|dur| {
            RawNoteInsn::TouchHold(TouchHoldParams {
                sensor,
                dur,
                modifier,
            })
            .with_span(span)
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::super::tests::{test_parser_err, test_parser_ok, test_parser_warn};
    use super::*;
    use std::error::Error;

    #[test]
    fn test_t_touch_hold() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            *test_parser_ok(t_touch_hold, "C1h[1:1]", "").unwrap(),
            RawNoteInsn::TouchHold(TouchHoldParams {
                sensor: ('C', None).try_into().unwrap(),
                dur: Duration::NumBeats(NumBeatsParams {
                    bpm: None,
                    divisor: 1,
                    num: 1
                }),
                modifier: TouchHoldModifier::default(),
            })
        );
        // modifier before h
        assert_eq!(
            *test_parser_ok(t_touch_hold, "C1fh[1:1]", "").unwrap(),
            RawNoteInsn::TouchHold(TouchHoldParams {
                sensor: ('C', None).try_into().unwrap(),
                dur: Duration::NumBeats(NumBeatsParams {
                    bpm: None,
                    divisor: 1,
                    num: 1
                }),
                modifier: TouchHoldModifier { is_firework: true },
            })
        );
        // modifier after h
        assert_eq!(
            *test_parser_ok(t_touch_hold, "C1hf[1:1]", "").unwrap(),
            RawNoteInsn::TouchHold(TouchHoldParams {
                sensor: ('C', None).try_into().unwrap(),
                dur: Duration::NumBeats(NumBeatsParams {
                    bpm: None,
                    divisor: 1,
                    num: 1
                }),
                modifier: TouchHoldModifier { is_firework: true },
            })
        );
        // modifier on both sides (f before and after h -> duplicate warning)
        test_parser_warn(t_touch_hold, "C1f hf[1:1],");
        // duplicate modifier warning
        test_parser_warn(t_touch_hold, "C1ffh[1:1],");
        test_parser_warn(t_touch_hold, "C1hff[1:1],");
        // missing duration
        test_parser_err(t_touch_hold, "C1h");
        Ok(())
    }
}
