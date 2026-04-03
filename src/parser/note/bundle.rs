use super::*;
use hold::t_hold;
use slide::t_slide;
use tap::t_tap;
use touch::t_touch;
use touch_hold::t_touch_hold;

pub fn t_bundle_note(s: NomSpan) -> PResult<Option<SpRawNoteInsn>> {
    // NOTE: tap and touch must come last as it can match on the simplest key, blocking holds and slides from parsing
    nom::branch::alt((t_hold, t_touch_hold, t_slide, t_tap, t_touch))(s)
}

pub fn t_bundle_sep_note(s: NomSpan) -> PResult<Option<SpRawNoteInsn>> {
    use nom::character::complete::char;

    let (s, _) = char('/')(s)?;
    let (s, note) = ws(t_bundle_note).expect(PError::MissingNote)(s)?;

    Ok((s, note.flatten()))
}

pub fn t_bundle(s: NomSpan) -> PResult<Option<SpRawInsn>> {
    use nom::multi::many0;

    let (s, start_loc) = nom_locate::position(s)?;
    let (s, first) = t_bundle_note(s)?;
    let (s, rest) = many0(ws(t_bundle_sep_note))(s)?;
    let (s, _) = ws(t_note_sep)(s)?;
    let (s, end_loc) = nom_locate::position(s)?;

    let notes: Vec<_> = std::iter::once(first).chain(rest).flatten().collect();
    if notes.is_empty() {
        return Ok((s, None));
    }

    let span = (start_loc, end_loc);
    // TODO: check len before flattening?
    Ok((s, Some(RawInsn::Notes(notes).with_span(span))))
}
