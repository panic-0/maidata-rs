use ndarray::Array3;
use std::ops::Deref;

use crate::insn::TouchSensor;
use crate::judge::slide_data_getter::SLIDE_DATA_GETTER;
use crate::materialize::{
    MaterializedHold, MaterializedSlideTrack, MaterializedTap, MaterializedTouch,
    MaterializedTouchHold, Note, TimestampInSeconds,
};
use crate::transform::transform::{Transformable, Transformer};
use crate::transform::{
    NormalizedSlideSegment, NormalizedSlideSegmentParams, NormalizedSlideSegmentShape,
    NormalizedSlideTrack,
};

use super::sensor::{sensor_index, NUM_SENSORS};

pub const FRAME_DT: f64 = 0.2;
pub const NUM_CHANNELS: usize = 5;

// Channel indices
pub const CH_TAP_INSTANT: usize = 0;
pub const CH_TOUCH_INSTANT: usize = 1;
pub const CH_HOLD: usize = 2;
pub const CH_SLIDE: usize = 3;
pub const CH_BREAK: usize = 4;

/// Encoder: converts materialized notes into `[T, 33, 5]` sensor-channel values (f32).
pub struct HeatmapEncoder {
    frame_dt: f64,
}

impl Default for HeatmapEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl HeatmapEncoder {
    pub fn new() -> Self {
        Self {
            frame_dt: FRAME_DT,
        }
    }

    pub fn frame_dt(&self) -> f64 {
        self.frame_dt
    }

    /// Encode materialized notes into `[T, 33, 5]` array.
    pub fn encode(&self, notes: &[Note]) -> Array3<f32> {
        let max_time = chart_duration(notes);
        let t = ((max_time / self.frame_dt).ceil() as usize).max(1);
        let mut frames = Array3::zeros((t, NUM_SENSORS, NUM_CHANNELS));

        for note in notes {
            match note {
                Note::Bpm(_) => {}
                Note::Tap(p) => self.encode_tap(&mut frames, p),
                Note::Touch(p) => self.encode_touch(&mut frames, p),
                Note::Hold(p) => self.encode_hold(&mut frames, p),
                Note::TouchHold(p) => self.encode_touch_hold(&mut frames, p),
                Note::SlideTrack(p) => self.encode_slide(&mut frames, p),
            }
        }
        frames
    }

    fn encode_tap(&self, frames: &mut Array3<f32>, tap: &MaterializedTap) {
        let fi = time_to_frame(tap.ts, self.frame_dt);
        if fi >= frames.dim().0 {
            return;
        }
        // Key i → sensor index i (A-ring)
        let si = tap.key.index() as usize;
        frames[[fi, si, CH_TAP_INSTANT]] += 1.0;
        if tap.is_break {
            frames[[fi, si, CH_BREAK]] += 1.0;
        }
    }

    fn encode_touch(&self, frames: &mut Array3<f32>, touch: &MaterializedTouch) {
        let fi = time_to_frame(touch.ts, self.frame_dt);
        if fi >= frames.dim().0 {
            return;
        }
        let si = sensor_index(&touch.sensor) as usize;
        frames[[fi, si, CH_TOUCH_INSTANT]] += 1.0;
    }

    fn encode_hold(&self, frames: &mut Array3<f32>, hold: &MaterializedHold) {
        let f0 = time_to_frame(hold.ts, self.frame_dt);
        let f1 = time_to_frame_ceil(hold.ts + hold.dur, self.frame_dt);
        let f1 = f1.min(frames.dim().0);
        let si = hold.key.index() as usize;
        for fi in f0..f1 {
            let overlap = frame_overlap(hold.ts, hold.ts + hold.dur, fi, self.frame_dt);
            let coverage = (overlap / self.frame_dt) as f32;
            frames[[fi, si, CH_HOLD]] += coverage;
        }
        if hold.is_break {
            let fi = time_to_frame(hold.ts, self.frame_dt);
            if fi < frames.dim().0 {
                frames[[fi, si, CH_BREAK]] += 1.0;
            }
        }
    }

    fn encode_touch_hold(&self, frames: &mut Array3<f32>, th: &MaterializedTouchHold) {
        let f0 = time_to_frame(th.ts, self.frame_dt);
        let f1 = time_to_frame_ceil(th.ts + th.dur, self.frame_dt);
        let f1 = f1.min(frames.dim().0);
        let si = sensor_index(&th.sensor) as usize;
        for fi in f0..f1 {
            let overlap = frame_overlap(th.ts, th.ts + th.dur, fi, self.frame_dt);
            let coverage = (overlap / self.frame_dt) as f32;
            frames[[fi, si, CH_HOLD]] += coverage;
        }
    }

    fn encode_slide(&self, frames: &mut Array3<f32>, track: &MaterializedSlideTrack) {
        let num_frames = frames.dim().0;

        // start_tap
        if let Some(ref tap) = track.start_tap {
            let fi = time_to_frame(tap.ts, self.frame_dt);
            if fi < num_frames {
                let si = tap.key.index() as usize;
                frames[[fi, si, CH_TAP_INSTANT]] += 1.0;
                if tap.is_break {
                    frames[[fi, si, CH_BREAK]] += 1.0;
                }
            }
        }

        // slide body
        let events = match expand_slide_path(track) {
            Some(e) => e,
            None => return,
        };

        let slide_start = track.start_ts;
        let slide_end = track.start_ts + track.dur;

        for (sensor, ev_start, ev_end) in &events {
            let f0 = time_to_frame(*ev_start, self.frame_dt);
            let f1 = time_to_frame_ceil(*ev_end, self.frame_dt).max(f0 + 1);
            let f1 = f1.min(num_frames);
            let si = sensor_index(sensor) as usize;
            for fi in f0..f1 {
                let overlap = frame_overlap(slide_start, slide_end, fi, self.frame_dt);
                let coverage = (overlap / self.frame_dt) as f32;
                frames[[fi, si, CH_SLIDE]] += coverage;
            }
        }
    }
}

// --- helpers ---

fn time_to_frame(t: TimestampInSeconds, dt: f64) -> usize {
    ((t / dt).floor()) as usize
}

fn time_to_frame_ceil(t: TimestampInSeconds, dt: f64) -> usize {
    ((t / dt).ceil()) as usize
}

fn frame_start(fi: usize, dt: f64) -> f64 {
    fi as f64 * dt
}

fn frame_overlap(ev_start: f64, ev_end: f64, fi: usize, dt: f64) -> f64 {
    let fs = frame_start(fi, dt);
    let fe = fs + dt;
    (ev_end.min(fe) - ev_start.max(fs)).max(0.0)
}

fn chart_duration(notes: &[Note]) -> f64 {
    notes
        .iter()
        .map(|n| match n {
            Note::Bpm(_) => 0.0,
            Note::Tap(p) => p.ts,
            Note::Touch(p) => p.ts,
            Note::Hold(p) => p.ts + p.dur,
            Note::TouchHold(p) => p.ts + p.dur,
            Note::SlideTrack(p) => p.start_ts + p.dur,
        })
        .fold(0.0f64, f64::max)
}

// --- slide path expansion ---

fn expand_slide_path(track: &MaterializedSlideTrack) -> Option<Vec<(TouchSensor, f64, f64)>> {
    let is_fan = track
        .segments
        .iter()
        .any(|s| s.shape == NormalizedSlideSegmentShape::Fan);

    if is_fan {
        return expand_fan_slide(track);
    }

    let norm_track = materialized_to_norm_track(&track.segments)?;
    let slide_data = SLIDE_DATA_GETTER.get(&norm_track)?;
    let total_dist = slide_data.total_distance();
    if total_dist <= 0.0 {
        return None;
    }

    let mut events = Vec::new();
    let mut cum = 0.0;

    for hit_area in slide_data.deref() {
        let d = hit_area.push_distance + hit_area.release_distance;
        let frac0 = cum / total_dist;
        let frac1 = (cum + d) / total_dist;
        let t0 = track.start_ts + frac0 * track.dur;
        let t1 = track.start_ts + frac1 * track.dur;
        for sensor in &hit_area.hit_points {
            events.push((*sensor, t0, t1));
        }
        cum += d;
    }
    Some(events)
}

fn expand_fan_slide(track: &MaterializedSlideTrack) -> Option<Vec<(TouchSensor, f64, f64)>> {
    assert!(track.segments.len() == 1);
    let seg = &track.segments[0];
    let mut events = Vec::new();
    for &rotation in &[7u8, 0, 1] {
        let transformer = Transformer {
            rotation,
            flip: false,
        };
        let dest = seg.destination.transform(transformer);
        let norm_seg = NormalizedSlideSegment::new(
            seg.shape,
            NormalizedSlideSegmentParams {
                start: seg.start,
                destination: dest,
            },
        );
        let slide_data = SLIDE_DATA_GETTER.get_by_segment(&norm_seg)?;
        let total_dist = slide_data.total_distance();
        if total_dist <= 0.0 {
            continue;
        }
        let mut cum = 0.0;
        for hit_area in slide_data.deref() {
            let d = hit_area.push_distance + hit_area.release_distance;
            let frac0 = cum / total_dist;
            let frac1 = (cum + d) / total_dist;
            let t0 = track.start_ts + frac0 * track.dur;
            let t1 = track.start_ts + frac1 * track.dur;
            for sensor in &hit_area.hit_points {
                events.push((*sensor, t0, t1));
            }
            cum += d;
        }
    }
    Some(events)
}

fn materialized_to_norm_track(
    segments: &[crate::materialize::MaterializedSlideSegment],
) -> Option<NormalizedSlideTrack> {
    Some(NormalizedSlideTrack {
        segments: segments
            .iter()
            .map(|s| {
                NormalizedSlideSegment::new(
                    s.shape,
                    NormalizedSlideSegmentParams {
                        start: s.start,
                        destination: s.destination,
                    },
                )
            })
            .collect(),
    })
}
