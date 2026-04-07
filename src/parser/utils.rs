use super::*;

/// Set a boolean flag, emitting a DuplicateModifier warning if it was already set.
pub fn set_flag_or_warn(
    state: &std::cell::RefCell<crate::diag::State>,
    flag: &mut bool,
    c: char,
    note_type: crate::insn::NoteType,
    span: crate::Span,
) {
    if *flag {
        state
            .borrow_mut()
            .add_warning(crate::diag::PWarning::DuplicateModifier(c, note_type), span);
    }
    *flag = true;
}

/// remove leading whitespace
pub fn ws<'a, F, O>(f: F) -> impl FnMut(NomSpan<'a>) -> PResult<'a, O>
where
    F: 'a + FnMut(NomSpan<'a>) -> PResult<'a, O>,
{
    nom::sequence::preceded(multispace0, f)
}

fn ws_list_impl<'a, F, O>(
    mut f: F,
    require_first: bool,
) -> impl FnMut(NomSpan<'a>) -> PResult<'a, Vec<O>>
where
    F: 'a + FnMut(NomSpan<'a>) -> PResult<'a, O>,
{
    // TODO: nom::multi::separated_list0(multispace0, f) will not work as expected (#1691)
    // wait for nom 8.0.0...
    use nom::Err;
    move |mut i: NomSpan<'a>| {
        let mut res = Vec::new();

        match f(i) {
            Err(Err::Error(_)) if !require_first => return Ok((i, res)),
            Err(e) => return Err(e),
            Ok((i1, o)) => {
                res.push(o);
                i = i1;
            }
        }

        loop {
            match multispace0(i) {
                Err(Err::Error(_)) => return Ok((i, res)),
                Err(e) => return Err(e),
                Ok((i1, _)) => match f(i1) {
                    Err(Err::Error(_)) => return Ok((i, res)),
                    Err(e) => return Err(e),
                    Ok((i2, o)) => {
                        res.push(o);
                        i = i2;
                    }
                },
            }
        }
    }
}

pub fn ws_list0<'a, F, O>(f: F) -> impl FnMut(NomSpan<'a>) -> PResult<'a, Vec<O>>
where
    F: 'a + FnMut(NomSpan<'a>) -> PResult<'a, O>,
{
    ws_list_impl(f, false)
}

pub fn ws_list1<'a, F, O>(f: F) -> impl FnMut(NomSpan<'a>) -> PResult<'a, Vec<O>>
where
    F: 'a + FnMut(NomSpan<'a>) -> PResult<'a, O>,
{
    ws_list_impl(f, true)
}

pub fn expect<'a, F, T>(
    mut parser: F,
    error: PError,
) -> impl FnMut(NomSpan<'a>) -> PResult<'a, Option<T>>
where
    F: FnMut(NomSpan<'a>) -> PResult<'a, T>,
{
    move |input| {
        let error = error.clone();
        let (input, start_loc) = nom_locate::position(input)?;
        match parser(input) {
            Ok((remaining, out)) => Ok((remaining, Some(out))),
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                let (_, end_loc) = nom_locate::position(e.input)?;
                let span = (start_loc, end_loc).into();
                e.input.extra.borrow_mut().add_error(error, span);
                Ok((input, None))
            }
            Err(err) => Err(err),
        }
    }
}

pub trait Expect<'a, T> {
    fn expect(self, error: PError) -> impl FnMut(NomSpan<'a>) -> PResult<'a, Option<T>>;
}

impl<'a, T, U: 'a + FnMut(NomSpan<'a>) -> PResult<'a, T>> Expect<'a, T> for U {
    fn expect(self, error: PError) -> impl FnMut(NomSpan<'a>) -> PResult<'a, Option<T>> {
        expect(self, error)
    }
}

// TODO: refactor
pub fn expect_ws_delimited<'a, F, T>(
    mut inner: F,
    inner_name: &'a str,
    start: &'a str,
    end: &'a str,
) -> impl FnMut(NomSpan<'a>) -> PResult<'a, Option<T>>
where
    F: FnMut(NomSpan<'a>) -> PResult<'a, T>,
{
    use nom::bytes::complete::tag;
    use nom::character::complete::multispace0;
    use nom::combinator::opt;
    move |i| {
        let (i1, open) = opt(tag(start))(i)?;
        let (i2, _) = multispace0(i1)?;
        let (i2, result) = match inner(i2) {
            Ok((i, result)) => (i, Some(result)),
            Err(nom::Err::Error(_)) | Err(nom::Err::Failure(_)) => (i2, None),
            Err(err) => return Err(err),
        };
        let (i3, _) = multispace0(i2)?;
        let (i3, close) = opt(tag(end))(i3)?;

        // `x` / `(`
        if (open.is_none() || result.is_none()) && close.is_none() {
            return Err(nom::Err::Error(nom::error::Error::new(
                i,
                nom::error::ErrorKind::Tag,
            )));
        }
        if open.is_none() {
            let (_, end_loc) = nom_locate::position(i)?;
            i3.extra.borrow_mut().add_error(
                PError::MissingBefore {
                    token: format!("`{start}`"),
                    context: inner_name.to_string(),
                },
                (end_loc, end_loc).into(),
            );
        }
        if result.is_none() {
            let (_, end_loc) = nom_locate::position(i1)?;
            i3.extra.borrow_mut().add_error(
                PError::MissingBetween {
                    token: inner_name.to_string(),
                    open: format!("`{start}`"),
                    close: format!("`{end}`"),
                },
                (end_loc, end_loc).into(),
            );
        }
        if close.is_none() {
            let (_, end_loc) = nom_locate::position(i2)?;
            i3.extra.borrow_mut().add_error(
                PError::MissingAfter {
                    token: format!("`{end}`"),
                    context: inner_name.to_string(),
                },
                (end_loc, end_loc).into(),
            );
            return Ok((i2, None));
        }
        Ok((i3, result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that many0(ws(x)) returns the ORIGINAL input (including leading spaces)
    /// when there are 0 matches. This is critical — the old code had an explicit
    /// `if variants.is_empty() { s } else { s1 }` guard that would be needed
    /// if many0 consumed leading whitespace even on 0 matches.
    #[test]
    fn many0_ws_preserves_input_on_zero_matches() {
        use nom::bytes::complete::tag;

        let state = std::cell::RefCell::new(crate::State::default());

        // Case 1: input has leading spaces, no "b" follows
        let input = "  rest";
        let s = NomSpan::new_extra(input, &state);
        let (rem, vars) = nom::multi::many0(ws(tag("b")))(s).unwrap();
        assert!(vars.is_empty(), "expected 0 matches, got {vars:?}");
        assert_eq!(
            *rem.fragment(),
            "  rest",
            "many0(ws(tag(..))) should NOT consume leading whitespace on 0 matches"
        );

        // Case 2: input has no leading spaces, no "b" follows
        let input2 = "rest";
        let s2 = NomSpan::new_extra(input2, &state);
        let (rem2, vars2) = nom::multi::many0(ws(tag("b")))(s2).unwrap();
        assert!(vars2.is_empty());
        assert_eq!(*rem2.fragment(), "rest");

        // Case 3: input has one "b" with leading space — should consume the space+b
        let input3 = " b rest";
        let s3 = NomSpan::new_extra(input3, &state);
        let (rem3, vars3) = nom::multi::many0(ws(tag("b")))(s3).unwrap();
        assert_eq!(vars3.len(), 1);
        assert_eq!(*rem3.fragment(), " rest");
    }
}
