use super::*;

pub fn t_tap_modifier_str(s: NomSpan) -> PResult<Vec<NomSpan>> {
    use nom::branch::alt;
    use nom::bytes::complete::tag;
    use nom::multi::many0;

    let (s, variants) = many0(ws(alt((tag("b"), tag("x"), tag("$$"), tag("$")))))(s)?;

    Ok((s, variants))
}

pub fn t_tap_param(s: NomSpan) -> PResult<TapParams> {
    let (s, key) = t_key(s)?;
    let (s, start_loc) = nom_locate::position(s)?;
    let (s, modifier_str) = t_tap_modifier_str(s)?;
    let (s, end_loc) = nom_locate::position(s)?;

    let mut modifier = TapModifier::default();
    let span: Span = (start_loc, end_loc).into();
    for x in &modifier_str {
        match *x.fragment() {
            "b" => set_flag_or_warn(&s.extra, &mut modifier.is_break, 'b', NoteType::Tap, span),
            "x" => set_flag_or_warn(&s.extra, &mut modifier.is_ex, 'x', NoteType::Tap, span),
            _ => (),
        }
        let shape = match *x.fragment() {
            "$" => Some(TapShape::Star),
            "$$" => Some(TapShape::StarSpin),
            _ => None,
        };
        if let Some(shape) = shape {
            if modifier.shape.is_some() {
                s.extra.borrow_mut().add_error(
                    PError::DuplicateShapeModifier(NoteType::Tap),
                    (start_loc, end_loc).into(),
                );
            } else {
                modifier.shape = Some(shape);
            }
        }
    }

    Ok((s, TapParams { key, modifier }))
}

pub fn t_tap(s: NomSpan) -> PResult<Option<SpRawNoteInsn>> {
    let (s, start_loc) = nom_locate::position(s)?;
    let (s, params) = t_tap_param(s)?;
    let (s, end_loc) = nom_locate::position(s)?;

    let span = (start_loc, end_loc);
    Ok((s, Some(RawNoteInsn::Tap(params).with_span(span))))
}

pub fn t_tap_multi_simplified_every(s: NomSpan) -> PResult<SpRawNoteInsn> {
    let (s, start_loc) = nom_locate::position(s)?;
    let (s, key) = t_key(s)?;
    let (s, end_loc) = nom_locate::position(s)?;

    let span = (start_loc, end_loc);
    Ok((
        s,
        RawNoteInsn::Tap(TapParams {
            key,
            // all taps are regular ones when using simplified notation
            modifier: Default::default(),
        })
        .with_span(span),
    ))
}

pub fn t_tap_multi_simplified(s: NomSpan) -> PResult<Option<SpRawInsn>> {
    let (s, start_loc) = nom_locate::position(s)?;
    // all whitespaces are ignored, including those inside a taps bundle
    // we must parse every key individually (also for getting proper span info)
    let (s, notes) = ws_list1(t_tap_multi_simplified_every)(s)?;
    let (s, _) = ws(t_note_sep)(s)?;
    let (s, end_loc) = nom_locate::position(s)?;

    let span = (start_loc, end_loc);
    Ok((s, Some(RawInsn::Notes(notes).with_span(span))))
}

#[cfg(test)]
mod tests {
    use super::super::tests::{test_parser_err, test_parser_ok, test_parser_warn};
    use super::*;
    use std::error::Error;

    #[test]
    fn test_t_tap_param() -> Result<(), Box<dyn Error>> {
        use tap::t_tap_param;
        assert_eq!(
            test_parser_ok(t_tap_param, "1", " ,"),
            TapParams {
                key: 0.try_into().unwrap(),
                modifier: Default::default(),
            }
        );
        assert_eq!(
            test_parser_ok(t_tap_param, "1 b x", ""),
            TapParams {
                key: 0.try_into().unwrap(),
                modifier: TapModifier {
                    is_break: true,
                    is_ex: true,
                    shape: None,
                },
            }
        );
        assert_eq!(
            test_parser_ok(t_tap_param, "1 x", ""),
            TapParams {
                key: 0.try_into().unwrap(),
                modifier: TapModifier {
                    is_break: false,
                    is_ex: true,
                    shape: None,
                },
            }
        );

        test_parser_err(t_tap_param, "");
        test_parser_err(t_tap_param, " 1");
        test_parser_err(t_tap_param, "x1");
        test_parser_warn(t_tap_param, "1xx");

        Ok(())
    }
}
