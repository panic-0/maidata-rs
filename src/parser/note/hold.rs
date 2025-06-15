use super::duration::t_dur;
use super::*;

pub fn t_hold_modifier_str(s: NomSpan) -> PResult<Vec<char>> {
    use nom::character::complete::one_of;
    use nom::multi::many0;

    let (s1, variants) = many0(ws(one_of("bx")))(s)?;

    Ok((if variants.is_empty() { s } else { s1 }, variants))
}

pub fn t_hold(s: NomSpan) -> PResult<Option<SpRawNoteInsn>> {
    use nom::character::complete::char;
    use nom::combinator::{map, opt};

    let (s, start_loc) = nom_locate::position(s)?;
    let (s, key) = t_key(s)?;
    let (s, modifier_str) = t_hold_modifier_str(s)?;
    let (s, _) = ws(char('h'))(s)?;
    let (s, modifier_str) = map(t_hold_modifier_str, |mut m| {
        // TODO
        m.extend(modifier_str.clone());
        m
    })(s)?;
    let (s, dur) = opt(ws(t_dur))(s)?;
    let (s, end_loc) = nom_locate::position(s)?;

    let dur = dur.flatten().or_else(|| {
        s.extra.borrow_mut().add_warning(
            PWarning::MissingDuration(NoteType::TouchHold),
            (start_loc, end_loc).into(),
        );
        Some(Duration::default())
    });

    let mut modifier = HoldModifier::default();
    for x in &modifier_str {
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
        dur.map(|dur| RawNoteInsn::Hold(HoldParams { key, dur, modifier }).with_span(span)),
    ))
}
