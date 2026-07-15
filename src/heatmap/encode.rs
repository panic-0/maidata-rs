use ndarray::{s, Array2};
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

pub const NUM_TAP_FEATURES: usize = 8;
pub const NUM_TOUCH_FEATURES: usize = NUM_SENSORS;
pub const NUM_HOLD_FEATURES: usize = 2;
pub const NUM_SLIDE_FEATURES: usize = NUM_SENSORS;

pub const TAP_FEATURE_OFFSET: usize = 0;
pub const TOUCH_FEATURE_OFFSET: usize = TAP_FEATURE_OFFSET + NUM_TAP_FEATURES;
pub const HOLD_FEATURE_OFFSET: usize = TOUCH_FEATURE_OFFSET + NUM_TOUCH_FEATURES;
pub const SLIDE_FEATURE_OFFSET: usize = HOLD_FEATURE_OFFSET + NUM_HOLD_FEATURES;
pub const NUM_FEATURES: usize = SLIDE_FEATURE_OFFSET + NUM_SLIDE_FEATURES;

// Kept as an export-dimension alias for older call sites.
pub const NUM_CHANNELS: usize = NUM_FEATURES;

pub fn quantize_frames(frames: &Array2<f32>) -> Result<Array2<u8>, Box<dyn std::error::Error>> {
    let (t, features) = frames.dim();
    assert_eq!(features, NUM_FEATURES);

    let mut data = Vec::with_capacity(t * NUM_FEATURES);
    for fi in 0..t {
        for feature in 0..NUM_FEATURES {
            data.push(quantize_feature(frames[[fi, feature]], feature, fi)?);
        }
    }

    Ok(Array2::from_shape_vec((t, NUM_FEATURES), data).unwrap())
}

pub fn compact_quantized_frames(
    frames: &Array2<f32>,
) -> Result<(Array2<u8>, Vec<u32>), Box<dyn std::error::Error>> {
    let (t, features) = frames.dim();
    assert_eq!(features, NUM_FEATURES);

    let mut indices = Vec::new();
    let mut data = Vec::new();

    for fi in 0..t {
        let row = frames.slice(s![fi, ..]);
        if row.iter().all(|&v| v == 0.0) {
            continue;
        }

        indices.push(fi as u32);
        for feature in 0..NUM_FEATURES {
            data.push(quantize_feature(row[feature], feature, fi)?);
        }
    }

    let arr = Array2::from_shape_vec((indices.len(), NUM_FEATURES), data).unwrap();
    Ok((arr, indices))
}

pub fn quantize_feature(
    value: f32,
    feature: usize,
    frame: usize,
) -> Result<u8, Box<dyn std::error::Error>> {
    if value < 0.0 {
        return Err(
            format!("negative heatmap value {value} at frame {frame}, feature {feature}").into(),
        );
    }

    if feature < HOLD_FEATURE_OFFSET {
        if value > 255.0 {
            return Err(format!(
                "instant count {value} exceeds u8 at frame {frame}, feature {feature}"
            )
            .into());
        }
        return Ok(value as u8);
    }

    debug_assert!(feature < NUM_FEATURES);
    if value > 1.0 + 1e-4 {
        return Err(
            format!("occupancy {value} exceeds 1.0 at frame {frame}, feature {feature}").into(),
        );
    }
    Ok((value.min(1.0) * 255.0) as u8)
}

/// Encoder: converts materialized notes into `[T, 76]` frame features.
///
/// Feature layout:
/// - `0..8`: tap counts for keys A1..A8
/// - `8..41`: touch counts for sensors A1..A8, B1..B8, C, D1..D8, E1..E8
/// - `41..43`: two-hand hold/touch-hold occupancy ratios, ordered `c1 >= c2`
/// - `43..76`: slide occupancy ratios for the same 33-sensor order as `sensor_index()`
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
        Self { frame_dt: FRAME_DT }
    }

    pub fn frame_dt(&self) -> f64 {
        self.frame_dt
    }

    /// Encode materialized notes into `[T, 76]` array.
    pub fn encode(&self, notes: &[Note]) -> Array2<f32> {
        let max_time = chart_duration(notes);
        let t = ((max_time / self.frame_dt).ceil() as usize).max(1);
        let mut frames = Array2::zeros((t, NUM_FEATURES));
        let mut hold_intervals = Vec::new();
        let mut slide_intervals_by_sensor: Vec<Vec<(f64, f64)>> = vec![Vec::new(); NUM_SENSORS];

        for note in notes {
            match note {
                Note::Bpm(_) => {}
                Note::Tap(p) => self.encode_tap(&mut frames, p),
                Note::Touch(p) => self.encode_touch(&mut frames, p),
                Note::Hold(p) => {
                    self.encode_hold_head(&mut frames, p);
                    hold_intervals.push((p.ts, p.ts + p.dur));
                }
                Note::TouchHold(p) => {
                    self.encode_touch_hold_head(&mut frames, p);
                    hold_intervals.push((p.ts, p.ts + p.dur));
                }
                Note::SlideTrack(p) => {
                    self.collect_slide_intervals(&mut slide_intervals_by_sensor, p)
                }
            }
        }

        self.encode_hand_occupancy(&mut frames, &hold_intervals);
        self.encode_slide_occupancy(&mut frames, &slide_intervals_by_sensor);
        frames
    }

    fn encode_tap(&self, frames: &mut Array2<f32>, tap: &MaterializedTap) {
        let fi = time_to_frame(tap.ts, self.frame_dt);
        if fi >= frames.dim().0 {
            return;
        }
        let feature = TAP_FEATURE_OFFSET + tap.key.index() as usize;
        frames[[fi, feature]] += 1.0;
    }

    fn encode_touch(&self, frames: &mut Array2<f32>, touch: &MaterializedTouch) {
        let fi = time_to_frame(touch.ts, self.frame_dt);
        if fi >= frames.dim().0 {
            return;
        }
        let feature = TOUCH_FEATURE_OFFSET + sensor_index(&touch.sensor) as usize;
        frames[[fi, feature]] += 1.0;
    }

    fn encode_hold_head(&self, frames: &mut Array2<f32>, hold: &MaterializedHold) {
        let fi = time_to_frame(hold.ts, self.frame_dt);
        if fi >= frames.dim().0 {
            return;
        }
        let feature = TAP_FEATURE_OFFSET + hold.key.index() as usize;
        frames[[fi, feature]] += 1.0;
    }

    fn encode_touch_hold_head(&self, frames: &mut Array2<f32>, th: &MaterializedTouchHold) {
        let fi = time_to_frame(th.ts, self.frame_dt);
        if fi >= frames.dim().0 {
            return;
        }
        let feature = TOUCH_FEATURE_OFFSET + sensor_index(&th.sensor) as usize;
        frames[[fi, feature]] += 1.0;
    }

    fn collect_slide_intervals(
        &self,
        intervals_by_sensor: &mut [Vec<(f64, f64)>],
        track: &MaterializedSlideTrack,
    ) {
        let events = match expand_slide_path(track) {
            Some(e) => e,
            None => return,
        };

        for (sensor, ev_start, ev_end) in events {
            if ev_end <= ev_start {
                continue;
            }
            let si = sensor_index(&sensor) as usize;
            intervals_by_sensor[si].push((ev_start, ev_end));
        }
    }

    fn encode_hand_occupancy(&self, frames: &mut Array2<f32>, intervals: &[(f64, f64)]) {
        if intervals.is_empty() {
            return;
        }

        for fi in 0..frames.dim().0 {
            let fs = frame_start(fi, self.frame_dt);
            let fe = fs + self.frame_dt;
            let (first, second) = two_hand_frame_coverage(intervals, fs, fe);
            frames[[fi, HOLD_FEATURE_OFFSET]] = first as f32;
            frames[[fi, HOLD_FEATURE_OFFSET + 1]] = second as f32;
        }
    }

    fn encode_slide_occupancy(
        &self,
        frames: &mut Array2<f32>,
        intervals_by_sensor: &[Vec<(f64, f64)>],
    ) {
        for (si, intervals) in intervals_by_sensor.iter().enumerate() {
            if intervals.is_empty() {
                continue;
            }
            for fi in 0..frames.dim().0 {
                let fs = frame_start(fi, self.frame_dt);
                let fe = fs + self.frame_dt;
                let coverage = union_frame_coverage(intervals, fs, fe);
                frames[[fi, SLIDE_FEATURE_OFFSET + si]] = coverage as f32;
            }
        }
    }
}

// --- helpers ---

fn time_to_frame(t: TimestampInSeconds, dt: f64) -> usize {
    ((t / dt).floor()) as usize
}

fn frame_start(fi: usize, dt: f64) -> f64 {
    fi as f64 * dt
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

fn two_hand_frame_coverage(
    intervals: &[(f64, f64)],
    frame_start: f64,
    frame_end: f64,
) -> (f64, f64) {
    let mut points = Vec::new();
    for &(start, end) in intervals {
        let start = start.max(frame_start);
        let end = end.min(frame_end);
        if end > start {
            points.push(start);
            points.push(end);
        }
    }

    if points.is_empty() {
        return (0.0, 0.0);
    }

    points.sort_by(f64::total_cmp);
    points.dedup_by(|a, b| (*a - *b).abs() < f64::EPSILON);

    let mut first = 0.0;
    let mut second = 0.0;
    for window in points.windows(2) {
        let start = window[0];
        let end = window[1];
        if end <= start {
            continue;
        }

        let mid = (start + end) * 0.5;
        let active = intervals
            .iter()
            .filter(|&&(iv_start, iv_end)| iv_start < mid && mid < iv_end)
            .count();
        let dur = end - start;
        if active >= 1 {
            first += dur;
        }
        if active >= 2 {
            second += dur;
        }
    }

    let frame_len = frame_end - frame_start;
    ((first / frame_len).min(1.0), (second / frame_len).min(1.0))
}

fn union_frame_coverage(intervals: &[(f64, f64)], frame_start: f64, frame_end: f64) -> f64 {
    let mut clipped: Vec<(f64, f64)> = intervals
        .iter()
        .filter_map(|&(start, end)| {
            let start = start.max(frame_start);
            let end = end.min(frame_end);
            (end > start).then_some((start, end))
        })
        .collect();

    if clipped.is_empty() {
        return 0.0;
    }

    clipped.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.total_cmp(&b.1)));
    let mut total = 0.0;
    let mut cur_start = clipped[0].0;
    let mut cur_end = clipped[0].1;

    for &(start, end) in &clipped[1..] {
        if start <= cur_end {
            cur_end = cur_end.max(end);
        } else {
            total += cur_end - cur_start;
            cur_start = start;
            cur_end = end;
        }
    }
    total += cur_end - cur_start;

    (total / (frame_end - frame_start)).min(1.0)
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
        let d = hit_area.push_distance;
        let frac0 = cum / total_dist;
        let frac1 = (cum + d) / total_dist;
        let t0 = track.start_ts + frac0 * track.dur;
        let t1 = track.start_ts + frac1 * track.dur;
        for sensor in &hit_area.hit_points {
            events.push((*sensor, t0, t1));
        }
        cum += d + hit_area.release_distance;
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
            vertical_flip: false,
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
            let d = hit_area.push_distance;
            let frac0 = cum / total_dist;
            let frac1 = (cum + d) / total_dist;
            let t0 = track.start_ts + frac0 * track.dur;
            let t1 = track.start_ts + frac1 * track.dur;
            for sensor in &hit_area.hit_points {
                events.push((*sensor, t0, t1));
            }
            cum += d + hit_area.release_distance;
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
