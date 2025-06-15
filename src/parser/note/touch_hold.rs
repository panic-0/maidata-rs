use super::duration::t_dur;
use super::*;

pub fn t_touch_hold_modifier_str(s: NomSpan) -> PResult<Vec<char>> {
    use nom::character::complete::one_of;
    use nom::multi::many0;

    let (s1, variants) = many0(ws(one_of("f")))(s)?;

    Ok((if variants.is_empty() { s } else { s1 }, variants))
}

pub fn t_touch_hold(s: NomSpan) -> PResult<Option<SpRawNoteInsn>> {
    use nom::character::complete::char;
    use nom::combinator::map;

    let (s, start_loc) = nom_locate::position(s)?;
    let (s, sensor) = t_touch_sensor(s)?;
    let (s, modifier_str) = t_touch_hold_modifier_str(s)?;
    let (s, _) = ws(char('h'))(s)?;
    let (s, modifier_str) = map(t_touch_hold_modifier_str, |mut m| {
        // TODO
        m.extend(modifier_str.clone());
        m
    })(s)?;
    let (s, mut dur) = ws(t_dur)(s)?;
    let (s, end_loc) = nom_locate::position(s)?;

    dur = dur.or_else(|| {
        s.extra.borrow_mut().add_warning(
            PWarning::MissingDuration(NoteType::TouchHold),
            (start_loc, end_loc).into(),
        );
        Some(Duration::default())
    });

    let mut modifier = TouchHoldModifier::default();
    for x in &modifier_str {
        match *x {
            'f' => {
                if modifier.is_firework {
                    s.extra.borrow_mut().add_warning(
                        PWarning::DuplicateModifier('f', NoteType::TouchHold),
                        (start_loc, end_loc).into(),
                    );
                }
                modifier.is_firework = true;
            }
            _ => unreachable!(),
        }
    }

    let span = (start_loc, end_loc);
    Ok((
        s,
        dur.map(|dur| {
            RawNoteInsn::TouchHold(TouchHoldParams {
                sensor,
                dur,
                modifier,
            })
            .with_span(span)
        }),
    ))
}
