use super::duration::t_dur;
use super::*;

pub fn t_hold_modifier_str(s: NomSpan) -> PResult<Vec<char>> {
    use nom::character::complete::one_of;
    use nom::multi::many0;

    let (s, variants) = many0(ws(one_of("bx")))(s)?;

    Ok((s, variants))
}

pub fn t_hold(s: NomSpan) -> PResult<Option<SpRawNoteInsn>> {
    use nom::character::complete::char;

    let (s, start_loc) = nom_locate::position(s)?;
    let (s, key) = t_key(s)?;
    let (s, pre_mods) = t_hold_modifier_str(s)?;
    let (s, _) = ws(char('h'))(s)?;
    let (s, post_mods) = t_hold_modifier_str(s)?;
    let (s, dur) = ws(t_dur).expect(PError::MissingDuration(NoteType::Hold))(s)?;
    let (s, end_loc) = nom_locate::position(s)?;

    let mut modifier = HoldModifier::default();
    for x in pre_mods.iter().chain(&post_mods) {
        match *x {
            'b' => {
                if modifier.is_break {
                    s.extra.borrow_mut().add_warning(
                        PWarning::DuplicateModifier('b', NoteType::Hold),
                        (start_loc, end_loc).into(),
                    );
                }
                modifier.is_break = true;
            }
            'x' => {
                if modifier.is_ex {
                    s.extra.borrow_mut().add_warning(
                        PWarning::DuplicateModifier('x', NoteType::Hold),
                        (start_loc, end_loc).into(),
                    );
                }
                modifier.is_ex = true;
            }
            _ => unreachable!(),
        }
    }

    let span = (start_loc, end_loc);
    Ok((
        s,
        dur.flatten()
            .map(|dur| RawNoteInsn::Hold(HoldParams { key, dur, modifier }).with_span(span)),
    ))
}

#[cfg(test)]
mod tests {
    use super::super::tests::{test_parser_err, test_parser_ok, test_parser_warn};
    use super::*;
    use std::error::Error;

    #[test]
    fn test_t_hold() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            *test_parser_ok(t_hold, "1h[1:1]", "").unwrap(),
            RawNoteInsn::Hold(HoldParams {
                key: 0.try_into().unwrap(),
                dur: Duration::NumBeats(NumBeatsParams {
                    bpm: None,
                    divisor: 1,
                    num: 1
                }),
                modifier: HoldModifier::default(),
            })
        );
        // modifier before h
        assert_eq!(
            *test_parser_ok(t_hold, "1bh[1:1]", "").unwrap(),
            RawNoteInsn::Hold(HoldParams {
                key: 0.try_into().unwrap(),
                dur: Duration::NumBeats(NumBeatsParams {
                    bpm: None,
                    divisor: 1,
                    num: 1
                }),
                modifier: HoldModifier {
                    is_break: true,
                    is_ex: false,
                },
            })
        );
        // modifier after h
        assert_eq!(
            *test_parser_ok(t_hold, "1hb[1:1]", "").unwrap(),
            RawNoteInsn::Hold(HoldParams {
                key: 0.try_into().unwrap(),
                dur: Duration::NumBeats(NumBeatsParams {
                    bpm: None,
                    divisor: 1,
                    num: 1
                }),
                modifier: HoldModifier {
                    is_break: true,
                    is_ex: false,
                },
            })
        );
        // modifier on both sides
        assert_eq!(
            *test_parser_ok(t_hold, "1b hx[1:1]", "").unwrap(),
            RawNoteInsn::Hold(HoldParams {
                key: 0.try_into().unwrap(),
                dur: Duration::NumBeats(NumBeatsParams {
                    bpm: None,
                    divisor: 1,
                    num: 1
                }),
                modifier: HoldModifier {
                    is_break: true,
                    is_ex: true,
                },
            })
        );
        // duplicate modifier warning
        test_parser_warn(t_hold, "1bbh[1:1],");
        test_parser_warn(t_hold, "1bh b [1:1],");
        // missing duration
        test_parser_err(t_hold, "1h");
        Ok(())
    }
}
