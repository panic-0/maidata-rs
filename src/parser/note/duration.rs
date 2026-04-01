use super::*;

pub fn t_dur_spec_num_beats_params(s: NomSpan) -> PResult<Option<NumBeatsParams>> {
    use nom::character::complete::{char, digit1};

    // TODO: support floating point
    let (s, start_loc) = nom_locate::position(s)?;
    let (s, divisor_str) = digit1(s)?;
    let (s, _) = ws(char(':'))(s)?;
    let (s, num_str) = ws(digit1).expect(PError::MissingBeatsNum)(s)?;
    let (s, end_loc) = nom_locate::position(s)?;
    if num_str.is_none() {
        return Ok((s, None));
    }

    // TODO: handle conversion errors
    let divisor = divisor_str.fragment().parse().unwrap();
    let num = num_str.unwrap().fragment().parse().unwrap();

    if divisor == 0 {
        s.extra.borrow_mut().add_error(
            PError::InvalidBeatDivisor(format!("{divisor}:{num}")),
            (start_loc, end_loc).into(),
        );
        return Ok((s, None));
    }
    Ok((
        s,
        Some(NumBeatsParams {
            bpm: None,
            divisor,
            num,
        }),
    ))
}

pub fn t_dur_spec_bpm_num_beats_params(s: NomSpan) -> PResult<Option<NumBeatsParams>> {
    use nom::character::complete::char;
    use nom::number::complete::double;

    let (s, start_loc) = nom_locate::position(s)?;
    let (s, bpm) = double(s)?;
    let (s, _) = ws(char('#'))(s)?;
    let (s, mut dur) = ws(t_dur_spec_num_beats_params)(s)?;
    let (s, end_loc) = nom_locate::position(s)?;

    if let Some(dur) = dur.as_mut() {
        dur.bpm = Some(bpm);
    }

    if !bpm.is_finite() || bpm <= 0.0 {
        s.extra.borrow_mut().add_error(
            PError::InvalidBpm(bpm.to_string()),
            (start_loc, end_loc).into(),
        );
        return Ok((s, None));
    }
    Ok((s, dur))
}

pub fn t_dur_spec_num_beats(s: NomSpan) -> PResult<Option<Duration>> {
    use nom::branch::alt;

    alt((t_dur_spec_num_beats_params, t_dur_spec_bpm_num_beats_params))(s)
        .map(|(s, dur)| (s, dur.map(Duration::NumBeats)))
}

pub fn t_dur_spec_absolute(s: NomSpan) -> PResult<Option<Duration>> {
    let (s, start_loc) = nom_locate::position(s)?;
    let (s, dur) = t_absolute_duration(s)?;
    let (s, end_loc) = nom_locate::position(s)?;

    // dur can be 0
    if !dur.is_finite() || dur < 0.0 {
        s.extra.borrow_mut().add_error(
            PError::InvalidDuration(format!("#{dur}")),
            (start_loc, end_loc).into(),
        );
        return Ok((s, None));
    }
    Ok((s, Some(Duration::Seconds(dur))))
}

pub fn t_dur_spec(s: NomSpan) -> PResult<Option<Duration>> {
    use nom::branch::alt;

    alt((t_dur_spec_num_beats, t_dur_spec_absolute))(s)
}

pub fn t_dur(s: NomSpan) -> PResult<Option<Duration>> {
    let (s, dur) = expect_ws_delimited(t_dur_spec, "duration specification", "[", "]")(s)?;

    Ok((s, dur.flatten()))
}

#[cfg(test)]
mod tests {
    use super::super::tests::{test_parser_err, test_parser_ok};
    use super::*;

    #[test]
    fn test_t_dur() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            test_parser_ok(t_dur, "[ 4 : 3 ]", " ,").unwrap(),
            Duration::NumBeats(NumBeatsParams {
                bpm: None,
                divisor: 4,
                num: 3
            })
        );
        assert_eq!(
            test_parser_ok(t_dur, "[1:0]", "").unwrap(),
            Duration::NumBeats(NumBeatsParams {
                bpm: None,
                divisor: 1,
                num: 0
            })
        );
        test_parser_err(t_dur, "[1:]");
        test_parser_err(t_dur, "[0:1]");
        test_parser_err(t_dur, " [4:3]");
        test_parser_err(t_dur, "[4.5:2]");
        test_parser_err(t_dur, "[4:2.5]");

        assert_eq!(
            test_parser_ok(t_dur, "[#2.5]", " ,").unwrap(),
            Duration::Seconds(2.5)
        );
        assert_eq!(
            test_parser_ok(t_dur, "[ # 1 ]", "").unwrap(),
            Duration::Seconds(1.0)
        );
        assert_eq!(
            test_parser_ok(t_dur, "[#.0]", "").unwrap(),
            Duration::Seconds(0.0)
        );
        test_parser_err(t_dur, "[#-1]");
        test_parser_err(t_dur, "[#inf]");
        test_parser_err(t_dur, "[#nan]");
        test_parser_err(t_dur, "[#2.5.0]");
        test_parser_err(t_dur, "[#2 .5]");

        assert_eq!(
            test_parser_ok(t_dur, "[ 120.0 #4: 1]", " ,").unwrap(),
            Duration::NumBeats(NumBeatsParams {
                bpm: Some(120.0),
                divisor: 4,
                num: 1
            })
        );
        test_parser_err(t_dur, "[0#1:1]");
        test_parser_err(t_dur, "[120#0:1]");
        test_parser_err(t_dur, "[120#4:1.5]");

        test_parser_err(t_dur, "[4:1#160]");
        test_parser_err(t_dur, "[4:1#4:1]");
        test_parser_err(t_dur, "[160#2.0]");
        test_parser_err(t_dur, "[3.0##1.5]");
        test_parser_err(t_dur, "[3.0##4:1]");
        test_parser_err(t_dur, "[3.0##160#4:1]");

        Ok(())
    }
}
