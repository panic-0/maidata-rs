use super::*;

/// remove leading whitespace
pub fn ws<'a, F, O>(f: F) -> impl FnMut(NomSpan<'a>) -> PResult<'a, O>
where
    F: 'a + FnMut(NomSpan<'a>) -> PResult<'a, O>,
{
    nom::sequence::preceded(multispace0, f)
}

pub fn ws_list0<'a, F, O>(mut f: F) -> impl FnMut(NomSpan<'a>) -> PResult<'a, Vec<O>>
where
    F: 'a + FnMut(NomSpan<'a>) -> PResult<'a, O>,
{
    // TODO: nom::multi::separated_list0(multispace0, f) will not work as expected (#1691)
    // wait for nom 8.0.0...
    use nom::Err;
    move |mut i: NomSpan<'a>| {
        let mut res = Vec::new();

        match f(i) {
            Err(Err::Error(_)) => return Ok((i, res)),
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

pub fn ws_list1<'a, F, O>(mut f: F) -> impl FnMut(NomSpan<'a>) -> PResult<'a, Vec<O>>
where
    F: 'a + FnMut(NomSpan<'a>) -> PResult<'a, O>,
{
    // TODO: nom::multi::separated_list1(multispace0, f) will not work as expected (#1691)
    // wait for nom 8.0.0...
    use nom::Err;
    move |mut i: NomSpan<'a>| {
        let mut res = Vec::new();

        match f(i) {
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
                PError::ExpectedBefore {
                    expected: format!("`{start}`"),
                    location: inner_name.to_string(),
                },
                (end_loc, end_loc).into(),
            );
        }
        if result.is_none() {
            let (_, end_loc) = nom_locate::position(i1)?;
            i3.extra.borrow_mut().add_error(
                PError::ExpectedBetween {
                    expected: inner_name.to_string(),
                    previous: format!("`{start}`"),
                    next: format!("`{end}`"),
                },
                (end_loc, end_loc).into(),
            );
        }
        if close.is_none() {
            let (_, end_loc) = nom_locate::position(i2)?;
            i3.extra.borrow_mut().add_error(
                PError::ExpectedAfter {
                    expected: format!("`{end}`"),
                    location: inner_name.to_string(),
                },
                (end_loc, end_loc).into(),
            );
            return Ok((i2, None));
        }
        Ok((i3, result))
    }
}
