use super::*;

pub fn t_touch_modifier_str(s: NomSpan) -> PResult<Vec<char>> {
    use nom::character::complete::one_of;
    use nom::multi::many0;

    let (s, variants) = many0(ws(one_of("f")))(s)?;

    Ok((s, variants))
}

pub fn t_touch_param(s: NomSpan) -> PResult<TouchParams> {
    let (s, sensor) = t_touch_sensor(s)?;
    let (s, start_loc) = nom_locate::position(s)?;
    let (s, modifier_str) = t_touch_modifier_str(s)?;
    let (s, end_loc) = nom_locate::position(s)?;

    let mut modifier = TouchModifier::default();
    let span: Span = (start_loc, end_loc).into();
    for x in &modifier_str {
        match *x {
            'f' => set_flag_or_warn(s.extra, &mut modifier.is_firework, 'f', NoteType::Touch, span),
            _ => unreachable!(),
        }
    }

    Ok((s, TouchParams { sensor, modifier }))
}

pub fn t_touch(s: NomSpan) -> PResult<Option<SpRawNoteInsn>> {
    let (s, start_loc) = nom_locate::position(s)?;
    let (s, params) = t_touch_param(s)?;
    let (s, end_loc) = nom_locate::position(s)?;

    let span = (start_loc, end_loc);
    Ok((s, Some(RawNoteInsn::Touch(params).with_span(span))))
}

#[cfg(test)]
mod tests {
    use super::super::tests::{test_parser_err, test_parser_ok, test_parser_warn};
    use super::*;
    use std::error::Error;

    #[test]
    fn test_t_touch_param() -> Result<(), Box<dyn Error>> {
        use touch::t_touch_param;
        assert_eq!(
            test_parser_ok(t_touch_param, "B7", " ,"),
            TouchParams {
                sensor: ('B', Some(6)).try_into().unwrap(),
                modifier: Default::default(),
            }
        );
        assert_eq!(
            test_parser_ok(t_touch_param, "C 1 f", ""),
            TouchParams {
                sensor: ('C', None).try_into().unwrap(),
                modifier: TouchModifier { is_firework: true },
            }
        );

        test_parser_err(t_touch_param, "");
        test_parser_err(t_touch_param, " A1");
        test_parser_err(t_touch_param, "Af2");
        test_parser_warn(t_touch_param, "D1 ff,");

        Ok(())
    }
}
