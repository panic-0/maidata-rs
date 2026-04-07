use std::{iter, vec};

use super::Note;
use crate::materialize::{
    MaterializedBpm, MaterializedHold, MaterializedSlideSegment, MaterializedSlideTrack,
    MaterializedTap, MaterializedTapShape, MaterializedTouch, MaterializedTouchHold,
};
use crate::span::WithSpan;
use crate::{insn, transform, Sp};

pub struct MaterializationContext {
    // TODO: is slides' default stop time really independent of BPM changes?
    // currently it is dependent -- add a separate non-changing value (initialized by the "wholebpm"
    // thing) to move to independent
    curr_beat_dur: f64,
    curr_note_dur: f64,
    curr_ts: f64,
}

impl MaterializationContext {
    pub fn with_offset(offset_secs: f64) -> Self {
        Self {
            curr_beat_dur: 0.0,
            curr_note_dur: 0.0,
            curr_ts: offset_secs,
        }
    }

    /// Materialize a list of raw instructions into notes.
    pub fn materialize_insns<'a, I: IntoIterator<Item = &'a Sp<insn::RawInsn>>>(
        &mut self,
        insns: I,
    ) -> Vec<Sp<Note>> {
        insns
            .into_iter()
            .flat_map(|insn| self.materialize_raw_insn(insn))
            .collect()
    }

    /// Read in one raw instruction and materialize into note(s) if applicable.
    fn materialize_raw_insn(&mut self, insn: &Sp<insn::RawInsn>) -> Vec<Sp<Note>> {
        use std::ops::Deref;
        match insn.deref() {
            insn::RawInsn::Bpm(params) => {
                self.set_bpm(params.new_bpm);
                vec![Note::Bpm(MaterializedBpm {
                    ts: self.curr_ts,
                    bpm: params.new_bpm,
                })
                .with_span(insn.span())]
            }
            insn::RawInsn::BeatDivisor(params) => {
                match params {
                    insn::BeatDivisorParams::NewDivisor(new_divisor) => {
                        self.set_beat_divisor(*new_divisor);
                    }
                    insn::BeatDivisorParams::NewAbsoluteDuration(new_note_dur) => {
                        self.curr_note_dur = *new_note_dur;
                    }
                }
                vec![]
            }
            insn::RawInsn::Rest => {
                // currently rests don't materialize to anything
                let _ = self.advance_time();
                vec![]
            }
            insn::RawInsn::EndMark => {
                // TODO: make later materialize calls return error?
                vec![]
            }
            insn::RawInsn::Notes(raw_notes) => {
                let ts = self.advance_time();
                let is_each = raw_notes.len() > 1;
                // TODO
                let is_slide_each = raw_notes.iter().fold(0, |acc, x| {
                    acc + match x.deref() {
                        insn::RawNoteInsn::Slide(params) => params.tracks.len(),
                        _ => 0,
                    }
                }) > 1;
                raw_notes
                    .iter()
                    .flat_map(|raw_note| {
                        self.materialize_raw_note(ts, raw_note, is_each, is_slide_each)
                    })
                    .map(|note| note.with_span(insn.span()))
                    .collect()
            }
        }
    }

    fn set_bpm(&mut self, new_bpm: f64) {
        self.curr_beat_dur = bpm_to_beat_dur(new_bpm);
    }

    fn set_beat_divisor(&mut self, new_divisor: u32) {
        self.curr_note_dur = divide_beat(self.curr_beat_dur, new_divisor);
    }

    /// Advances timestamp by one "note", return the timestamp before advancing (that of the
    /// current note being materialized).
    fn advance_time(&mut self) -> f64 {
        let res = self.curr_ts;
        self.curr_ts += self.curr_note_dur;
        res
    }

    fn materialize_raw_note(
        &self,
        ts: f64,
        raw_note: &insn::RawNoteInsn,
        is_each: bool,
        is_slide_each: bool,
    ) -> Vec<Note> {
        match raw_note {
            insn::RawNoteInsn::Tap(params) => {
                let m_params = materialize_tap_params(ts, params, false, is_each);
                vec![Note::Tap(m_params)]
            }
            insn::RawNoteInsn::Touch(params) => {
                let m_params = materialize_touch_params(ts, params, is_each);
                vec![Note::Touch(m_params)]
            }
            insn::RawNoteInsn::Slide(params) => {
                materialize_slide(ts, self.curr_beat_dur, params, is_each, is_slide_each)
            }
            insn::RawNoteInsn::Hold(params) => {
                let m_params = materialize_hold_params(ts, self.curr_beat_dur, params, is_each);
                vec![Note::Hold(m_params)]
            }
            insn::RawNoteInsn::TouchHold(params) => {
                let m_params =
                    materialize_touch_hold_params(ts, self.curr_beat_dur, params, is_each);
                vec![Note::TouchHold(m_params)]
            }
        }
    }
}

fn bpm_to_beat_dur(bpm: f64) -> f64 {
    60.0 / bpm
}

fn divide_beat(beat_dur: f64, beat_divisor: u32) -> f64 {
    beat_dur * 4.0 / (beat_divisor as f64)
}

fn materialize_tap_params(
    ts: f64,
    p: &insn::TapParams,
    is_slide_star: bool,
    is_each: bool,
) -> MaterializedTap {
    let shape = match p.modifier.shape {
        Some(insn::TapShape::Ring) => MaterializedTapShape::Ring,
        Some(insn::TapShape::Star) => MaterializedTapShape::Star,
        Some(insn::TapShape::StarSpin) => MaterializedTapShape::StarSpin,
        // TODO: handle invalid shape
        Some(insn::TapShape::Invalid) => MaterializedTapShape::Invalid,
        None => {
            if is_slide_star {
                MaterializedTapShape::Star
            } else {
                MaterializedTapShape::Ring
            }
        }
    };

    MaterializedTap {
        ts,
        key: p.key,
        shape,
        is_break: p.modifier.is_break,
        is_ex: p.modifier.is_ex,
        is_each,
    }
}

fn materialize_touch_params(ts: f64, p: &insn::TouchParams, is_each: bool) -> MaterializedTouch {
    MaterializedTouch {
        ts,
        sensor: p.sensor,
        is_each,
    }
}

/// slide insn -> `vec![star tap, track, track, ...]`
fn materialize_slide(
    ts: f64,
    beat_dur: f64,
    p: &insn::SlideParams,
    is_each: bool,
    is_slide_each: bool,
) -> Vec<Note> {
    let mut start_tap = Some(materialize_tap_params(ts, &p.start, true, is_each));
    if p.tracks.is_empty() {
        return vec![Note::Tap(start_tap.unwrap())];
    }

    // TODO
    iter::once(Note::Tap(start_tap.unwrap()))
        .chain(p.tracks.iter().map(|track| {
            Note::SlideTrack(materialize_slide_track(
                ts,
                beat_dur,
                p.start.key,
                start_tap.take(),
                track,
                is_slide_each,
            ))
        }))
        .collect()
}

fn materialize_slide_track(
    ts: f64,
    beat_dur: f64,
    mut start_key: insn::Key,
    start_tap: Option<MaterializedTap>,
    track: &insn::SlideTrack,
    is_each: bool,
) -> MaterializedSlideTrack {
    // in simai, stop time is actually encoded (overridden) in the duration spec of individual
    // slide track
    //
    // take care of this, falling back to beat duration of current bpm
    let stop_time = match track.dur {
        insn::SlideDuration::Simple(duration) => duration.bpm().map_or(beat_dur, bpm_to_beat_dur),
        insn::SlideDuration::Custom(st, _) => stop_time_spec_to_dur(st),
    };

    let start_ts = ts + stop_time;

    let segments = track
        .segments
        .iter()
        .map(|segment| {
            // TODO: handle normalization error
            let normalized =
                transform::normalize::normalize_slide_segment(start_key, segment).unwrap();
            start_key = segment.params().destination;
            materialize_slide_segment(normalized)
        })
        .collect();

    MaterializedSlideTrack {
        ts,
        start_ts,
        dur: materialize_duration(track.dur.slide_duration(), beat_dur),
        start_tap,
        segments,
        is_break: track.modifier.is_break,
        is_sudden: track.modifier.is_sudden,
        is_each,
    }
}

fn materialize_slide_segment(
    normalized: crate::transform::NormalizedSlideSegment,
) -> MaterializedSlideSegment {
    MaterializedSlideSegment {
        start: normalized.params().start,
        destination: normalized.params().destination,
        shape: normalized.shape(),
    }
}

fn materialize_hold_params(
    ts: f64,
    beat_dur: f64,
    p: &insn::HoldParams,
    is_each: bool,
) -> MaterializedHold {
    MaterializedHold {
        ts,
        dur: materialize_duration(p.dur, beat_dur),
        key: p.key,
        is_break: p.modifier.is_break,
        is_ex: p.modifier.is_ex,
        is_each,
    }
}

fn materialize_touch_hold_params(
    ts: f64,
    beat_dur: f64,
    p: &insn::TouchHoldParams,
    is_each: bool,
) -> MaterializedTouchHold {
    MaterializedTouchHold {
        ts,
        dur: materialize_duration(p.dur, beat_dur),
        sensor: p.sensor,
        is_each,
    }
}

fn materialize_duration(x: insn::Duration, beat_dur: f64) -> f64 {
    match x {
        insn::Duration::NumBeats(p) => {
            let beat_dur = p.bpm.map_or(beat_dur, bpm_to_beat_dur);
            divide_beat(beat_dur, p.divisor) * (p.num as f64)
        }
        insn::Duration::Seconds(x) => x,
    }
}

fn stop_time_spec_to_dur(x: insn::SlideStopTimeSpec) -> f64 {
    match x {
        insn::SlideStopTimeSpec::Bpm(override_bpm) => bpm_to_beat_dur(override_bpm),
        insn::SlideStopTimeSpec::Seconds(x) => x,
    }
}
