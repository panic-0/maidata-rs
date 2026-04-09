use crate::sensor::{parse_sensor_idx, MERGE_DIST_THRESHOLD, SENSOR_POSITIONS};
use crate::types::{note_time_range, FlatNote, SlideJudgeArea};

const MERGE_TIME_THRESHOLD: f64 = 1e-6;

#[derive(Debug)]
pub struct MergeError {
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AreaSignature {
    sensors: Vec<usize>,
}

type SlideTrackView<'a> = (f64, f64, f64, f64, &'a Vec<SlideJudgeArea>);

fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

fn touch_index(note: &FlatNote) -> Option<usize> {
    match note {
        FlatNote::Touch { sensor, .. } | FlatNote::TouchHold { sensor, .. } => {
            Some(parse_sensor_idx(sensor))
        }
        _ => None,
    }
}

fn slide_track(note: &FlatNote) -> Option<SlideTrackView<'_>> {
    match note {
        FlatNote::SlideTrack {
            ts,
            end_ts,
            x,
            y,
            judge_areas,
            ..
        } => Some((*ts, *end_ts, *x, *y, judge_areas)),
        _ => None,
    }
}

fn first_area(note: &FlatNote) -> Option<&SlideJudgeArea> {
    slide_track(note).and_then(|(_, _, _, _, judge_areas)| judge_areas.first())
}

fn area_signature(area: &SlideJudgeArea) -> AreaSignature {
    let mut sensors: Vec<usize> = area
        .sensors
        .iter()
        .map(|sensor| parse_sensor_idx(sensor))
        .collect();
    sensors.sort_unstable();
    AreaSignature { sensors }
}

fn same_area(left: &SlideJudgeArea, right: &SlideJudgeArea) -> bool {
    area_signature(left) == area_signature(right)
        && (left.ts - right.ts).abs() <= MERGE_TIME_THRESHOLD
        && (left.exit_ts - right.exit_ts).abs() <= MERGE_TIME_THRESHOLD
        && distance((left.x, left.y), (right.x, right.y)) <= 1e-3
}

fn slide_signatures(note: &FlatNote) -> Vec<AreaSignature> {
    slide_track(note)
        .map(|(_, _, _, _, judge_areas)| judge_areas.iter().map(area_signature).collect())
        .unwrap_or_default()
}

fn _raw_key_index(key: u8) -> usize {
    key as usize
}

fn is_contiguous_subsequence_by<T>(
    haystack: &[T],
    needle: &[T],
    equals: impl Fn(&T, &T) -> bool,
) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|window| {
        window
            .iter()
            .zip(needle.iter())
            .all(|(left, right)| equals(left, right))
    })
}

fn common_prefix_area_len(left: &[SlideJudgeArea], right: &[SlideJudgeArea]) -> usize {
    left.iter()
        .zip(right.iter())
        .take_while(|(a, b)| same_area(a, b))
        .count()
}

fn common_suffix_area_len(left: &[SlideJudgeArea], right: &[SlideJudgeArea]) -> usize {
    left.iter()
        .rev()
        .zip(right.iter().rev())
        .take_while(|(a, b)| same_area(a, b))
        .count()
}

fn suffix_prefix_overlap_len_by<T>(
    left: &[T],
    right: &[T],
    equals: impl Fn(&T, &T) -> bool,
) -> usize {
    let max_len = left.len().min(right.len());
    (2..=max_len)
        .rev()
        .find(|&len| {
            left[left.len() - len..]
                .iter()
                .zip(right[..len].iter())
                .all(|(a, b)| equals(a, b))
        })
        .unwrap_or(0)
}

fn normalized_slide_track(mut judge_areas: Vec<SlideJudgeArea>) -> FlatNote {
    judge_areas.sort_by(|a, b| a.ts.partial_cmp(&b.ts).unwrap());
    let first = judge_areas.first().unwrap();
    let last = judge_areas.last().unwrap();
    FlatNote::SlideTrack {
        ts: first.ts,
        end_ts: last.exit_ts,
        x: first.x,
        y: first.y,
        segments: vec![],
        judge_areas,
    }
}

fn trim_slide_prefix(note: &FlatNote, trim_len: usize) -> Option<FlatNote> {
    let (_, _, _, _, judge_areas) = slide_track(note)?;
    if trim_len == 0 || trim_len >= judge_areas.len() {
        return None;
    }
    Some(normalized_slide_track(judge_areas[trim_len..].to_vec()))
}

fn trim_slide_suffix(note: &FlatNote, trim_len: usize) -> Option<FlatNote> {
    let (ts, _, x, y, judge_areas) = slide_track(note)?;
    if trim_len == 0 || trim_len >= judge_areas.len() {
        return None;
    }
    let keep_len = judge_areas.len() - trim_len;
    let _ = (ts, x, y);
    Some(normalized_slide_track(judge_areas[..keep_len].to_vec()))
}

enum MergeResolution {
    None,
    ReplaceOne(FlatNote),
    ReplaceTwo(FlatNote, FlatNote),
}

fn touch_groups_at_same_time(notes: &[FlatNote], members: &[usize]) -> Vec<Vec<usize>> {
    let mut groups = Vec::new();
    let mut visited = vec![false; members.len()];
    for (i, &member_idx) in members.iter().enumerate() {
        if visited[i] || touch_index(&notes[member_idx]).is_none() {
            continue;
        }
        visited[i] = true;
        let mut stack = vec![i];
        let mut group = Vec::new();
        while let Some(local_idx) = stack.pop() {
            let note_idx = members[local_idx];
            let sensor_a = touch_index(&notes[note_idx]).unwrap();
            group.push(note_idx);
            for (j, &other_idx) in members.iter().enumerate() {
                if visited[j] {
                    continue;
                }
                let Some(sensor_b) = touch_index(&notes[other_idx]) else {
                    continue;
                };
                if distance(SENSOR_POSITIONS[sensor_a], SENSOR_POSITIONS[sensor_b])
                    < MERGE_DIST_THRESHOLD
                {
                    visited[j] = true;
                    stack.push(j);
                }
            }
        }
        groups.push(group);
    }
    groups
}

fn merge_touch_groups(notes: Vec<FlatNote>) -> Vec<FlatNote> {
    let mut result = Vec::new();
    let mut used = vec![false; notes.len()];

    for i in 0..notes.len() {
        if used[i] {
            continue;
        }
        let Some(sensor_i) = touch_index(&notes[i]) else {
            used[i] = true;
            result.push(notes[i].clone());
            continue;
        };
        let ts_i = note_time_range(&notes[i]).0;
        let mut group = vec![i];
        used[i] = true;
        for j in (i + 1)..notes.len() {
            if used[j] || (note_time_range(&notes[j]).0 - ts_i).abs() > MERGE_TIME_THRESHOLD {
                continue;
            }
            let Some(sensor_j) = touch_index(&notes[j]) else {
                continue;
            };
            if distance(SENSOR_POSITIONS[sensor_i], SENSOR_POSITIONS[sensor_j])
                < MERGE_DIST_THRESHOLD
            {
                used[j] = true;
                group.push(j);
            }
        }
        if group.len() == 1 {
            result.push(notes[i].clone());
            continue;
        }
        let mut sx = 0.0;
        let mut sy = 0.0;
        for &idx in &group {
            match &notes[idx] {
                FlatNote::Touch { x, y, .. } | FlatNote::TouchHold { x, y, .. } => {
                    sx += *x;
                    sy += *y;
                }
                _ => {}
            }
        }
        let count = group.len() as f64;
        match &notes[i] {
            FlatNote::Touch { sensor, .. } => result.push(FlatNote::Touch {
                ts: ts_i,
                sensor: sensor.clone(),
                x: sx / count,
                y: sy / count,
            }),
            FlatNote::TouchHold { sensor, dur, .. } => result.push(FlatNote::TouchHold {
                ts: ts_i,
                dur: *dur,
                sensor: sensor.clone(),
                x: sx / count,
                y: sy / count,
            }),
            _ => unreachable!(),
        }
    }

    result.sort_by(|a, b| {
        note_time_range(a)
            .0
            .partial_cmp(&note_time_range(b).0)
            .unwrap()
    });
    result
}

fn delete_tap_and_touch_covered_by_slides(notes: Vec<FlatNote>) -> Vec<FlatNote> {
    let mut buckets: Vec<(f64, Vec<usize>)> = Vec::new();
    for (idx, note) in notes.iter().enumerate() {
        let ts = note_time_range(note).0;
        if let Some((_, members)) = buckets
            .iter_mut()
            .find(|(bucket_ts, _)| (*bucket_ts - ts).abs() <= MERGE_TIME_THRESHOLD)
        {
            members.push(idx);
        } else {
            buckets.push((ts, vec![idx]));
        }
    }

    let mut remove = vec![false; notes.len()];

    for (_, members) in buckets {
        let slide_members: Vec<usize> = members
            .iter()
            .copied()
            .filter(|&idx| matches!(notes[idx], FlatNote::SlideTrack { .. }))
            .collect();

        for slide_idx in &slide_members {
            if let Some(area) = first_area(&notes[*slide_idx]) {
                for &member_idx in &members {
                    if let FlatNote::Tap { key, .. } = &notes[member_idx] {
                        if area
                            .sensors
                            .iter()
                            .any(|sensor| parse_sensor_idx(sensor) == _raw_key_index(*key))
                        {
                            remove[member_idx] = true;
                        }
                    }
                }
            }
        }

        for touch_group in touch_groups_at_same_time(&notes, &members) {
            let should_remove = slide_members.iter().any(|&slide_idx| {
                let Some((_, _, _, _, judge_areas)) = slide_track(&notes[slide_idx]) else {
                    return false;
                };
                touch_group.iter().all(|&touch_idx| {
                    let touch_sensor = touch_index(&notes[touch_idx]).unwrap();
                    judge_areas.iter().any(|area| {
                        area.sensors.iter().any(|sensor| {
                            distance(
                                SENSOR_POSITIONS[touch_sensor],
                                SENSOR_POSITIONS[parse_sensor_idx(sensor)],
                            ) < MERGE_DIST_THRESHOLD
                        })
                    })
                })
            });
            if should_remove {
                for idx in touch_group {
                    remove[idx] = true;
                }
            }
        }
    }

    notes
        .into_iter()
        .enumerate()
        .filter_map(|(idx, note)| (!remove[idx]).then_some(note))
        .collect()
}

fn merge_slide_pair(left: &FlatNote, right: &FlatNote) -> Result<MergeResolution, MergeError> {
    let Some((l_ts, l_end, _l_x, _l_y, l_areas)) = slide_track(left) else {
        return Ok(MergeResolution::None);
    };
    let Some((r_ts, r_end, _r_x, _r_y, r_areas)) = slide_track(right) else {
        return Ok(MergeResolution::None);
    };

    let left_sig = slide_signatures(left);
    let right_sig = slide_signatures(right);

    if r_areas.len() >= 2 {
        if let Some(start) = is_contiguous_subsequence_by(l_areas, r_areas, same_area) {
            if start > 0 && start + right_sig.len() < left_sig.len() {
                return Ok(MergeResolution::ReplaceOne(left.clone()));
            }
        }
    }
    if l_areas.len() >= 2 {
        if let Some(start) = is_contiguous_subsequence_by(r_areas, l_areas, same_area) {
            if start > 0 && start + left_sig.len() < right_sig.len() {
                return Ok(MergeResolution::ReplaceOne(right.clone()));
            }
        }
    }

    let overlap_lr = suffix_prefix_overlap_len_by(l_areas, r_areas, same_area);
    if overlap_lr >= 2 {
        let mut merged_areas = l_areas.clone();
        merged_areas.extend_from_slice(&r_areas[overlap_lr..]);
        return Ok(MergeResolution::ReplaceOne(normalized_slide_track(
            merged_areas,
        )));
    }

    let overlap_rl = suffix_prefix_overlap_len_by(r_areas, l_areas, same_area);
    if overlap_rl >= 2 {
        let mut merged_areas = r_areas.clone();
        merged_areas.extend_from_slice(&l_areas[overlap_rl..]);
        return Ok(MergeResolution::ReplaceOne(normalized_slide_track(
            merged_areas,
        )));
    }

    let same_tail =
        left_sig.last() == right_sig.last() && (l_end - r_end).abs() <= MERGE_TIME_THRESHOLD;
    if same_tail {
        let overlap = common_suffix_area_len(l_areas, r_areas);
        if overlap >= 2 {
            if let Some(trimmed_right) = trim_slide_suffix(right, overlap) {
                return Ok(MergeResolution::ReplaceTwo(left.clone(), trimmed_right));
            }
            if let Some(trimmed_left) = trim_slide_suffix(left, overlap) {
                return Ok(MergeResolution::ReplaceTwo(trimmed_left, right.clone()));
            }
        }
        return Ok(MergeResolution::None);
    }

    let same_head =
        left_sig.first() == right_sig.first() && (l_ts - r_ts).abs() <= MERGE_TIME_THRESHOLD;
    if same_head {
        let overlap = common_prefix_area_len(l_areas, r_areas);
        if overlap >= 2 {
            if let Some(trimmed_right) = trim_slide_prefix(right, overlap) {
                return Ok(MergeResolution::ReplaceTwo(left.clone(), trimmed_right));
            }
            if let Some(trimmed_left) = trim_slide_prefix(left, overlap) {
                return Ok(MergeResolution::ReplaceTwo(trimmed_left, right.clone()));
            }
        }
        return Ok(MergeResolution::None);
    }

    Ok(MergeResolution::None)
}

fn merge_slide_tracks(notes: Vec<FlatNote>) -> Result<Vec<FlatNote>, MergeError> {
    let mut notes = notes;
    loop {
        let mut changed = false;
        'outer: for i in 0..notes.len() {
            for j in (i + 1)..notes.len() {
                let Some(_) = slide_track(&notes[i]) else {
                    continue;
                };
                let Some(_) = slide_track(&notes[j]) else {
                    continue;
                };
                match merge_slide_pair(&notes[i], &notes[j])? {
                    MergeResolution::None => {}
                    MergeResolution::ReplaceOne(merged) => {
                        notes[i] = merged;
                        notes.remove(j);
                        changed = true;
                        break 'outer;
                    }
                    MergeResolution::ReplaceTwo(left_new, right_new) => {
                        notes[i] = left_new;
                        notes[j] = right_new;
                        changed = true;
                        break 'outer;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    Ok(notes)
}

pub fn merge_chart(notes: Vec<FlatNote>) -> Result<Vec<FlatNote>, MergeError> {
    let notes = delete_tap_and_touch_covered_by_slides(notes);
    let notes = merge_slide_tracks(notes)?;
    Ok(merge_touch_groups(notes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(ts: f64, sensor: &str) -> SlideJudgeArea {
        SlideJudgeArea {
            ts,
            exit_ts: ts + 0.1,
            sensors: vec![sensor.into()],
            x: 0.0,
            y: 0.0,
        }
    }

    fn slide(ts: f64, end_ts: f64, path: &[&str]) -> FlatNote {
        FlatNote::SlideTrack {
            ts,
            end_ts,
            x: 0.0,
            y: 0.0,
            segments: vec![],
            judge_areas: path
                .iter()
                .enumerate()
                .map(|(idx, sensor)| area(ts + idx as f64 * 0.1, sensor))
                .collect(),
        }
    }

    #[test]
    fn tap_on_slide_head_is_deleted() {
        let merged = merge_chart(vec![
            FlatNote::Tap {
                ts: 1.0,
                key: 0,
                x: 0.0,
                y: 0.0,
                is_star: false,
            },
            slide(1.0, 2.0, &["A1", "A2", "A3"]),
        ])
        .unwrap();
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn pure_tail_head_contact_does_not_merge() {
        let merged = merge_chart(vec![
            slide(1.0, 2.0, &["A1", "A2", "A3"]),
            slide(2.0, 3.0, &["A3", "A4", "A5"]),
        ])
        .unwrap();
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn contained_middle_slide_is_absorbed() {
        let merged = merge_chart(vec![
            slide(1.0, 3.0, &["A1", "A2", "A3", "A4"]),
            slide(1.1, 2.2, &["A2", "A3"]),
        ])
        .unwrap();
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn overlapping_middle_slides_merge() {
        let merged = merge_chart(vec![
            slide(1.0, 2.5, &["A1", "A2", "A3"]),
            slide(1.1, 3.0, &["A2", "A3", "A4"]),
        ])
        .unwrap();
        assert_eq!(merged.len(), 1);
        match &merged[0] {
            FlatNote::SlideTrack { judge_areas, .. } => {
                assert_eq!(judge_areas.len(), 4);
            }
            _ => panic!("expected slide"),
        }
    }
}
