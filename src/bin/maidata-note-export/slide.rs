use crate::sensor::{centroid_of_sensors, left_neighbor, right_neighbor};
use crate::types::{FlatNote, SlideJudgeArea, SlideSegment, SlideTrack};
use maidata::judge::slide_data_getter::SLIDE_DATA_GETTER;
use maidata::materialize::MaterializedSlideTrack;
use maidata::transform::{
    NormalizedSlideSegment, NormalizedSlideSegmentParams, NormalizedSlideSegmentShape,
    NormalizedSlideTrack,
};

pub fn original_slide_track(track: &MaterializedSlideTrack) -> SlideTrack {
    let segments: Vec<SlideSegment> = track
        .segments
        .iter()
        .map(|seg| {
            let shape_str = match seg.shape {
                NormalizedSlideSegmentShape::Straight => "straight",
                NormalizedSlideSegmentShape::CircleL => "circle_l",
                NormalizedSlideSegmentShape::CircleR => "circle_r",
                NormalizedSlideSegmentShape::CurveL => "curve_l",
                NormalizedSlideSegmentShape::CurveR => "curve_r",
                NormalizedSlideSegmentShape::ThunderL => "thunder_l",
                NormalizedSlideSegmentShape::ThunderR => "thunder_r",
                NormalizedSlideSegmentShape::Corner => "corner",
                NormalizedSlideSegmentShape::BendL => "bend_l",
                NormalizedSlideSegmentShape::BendR => "bend_r",
                NormalizedSlideSegmentShape::SkipL => "skip_l",
                NormalizedSlideSegmentShape::SkipR => "skip_r",
                NormalizedSlideSegmentShape::Fan => "fan",
            };
            SlideSegment {
                shape: shape_str.to_string(),
                start: seg.start.index(),
                destination: seg.destination.index(),
            }
        })
        .collect();

    SlideTrack {
        start_ts: track.start_ts,
        end_ts: track.start_ts + track.dur,
        segments,
    }
}

pub fn expand_slide_track(track: &MaterializedSlideTrack) -> Option<FlatNote> {
    let mut judge_areas = Vec::new();

    let has_fan = track
        .segments
        .iter()
        .any(|s| s.shape == NormalizedSlideSegmentShape::Fan);

    if has_fan {
        for segment in &track.segments {
            if segment.shape == NormalizedSlideSegmentShape::Fan {
                let left = left_neighbor(segment.destination);
                let right = right_neighbor(segment.destination);

                for dest in [left, right] {
                    let norm = NormalizedSlideSegment::new(
                        NormalizedSlideSegmentShape::Straight,
                        NormalizedSlideSegmentParams {
                            start: segment.start,
                            destination: dest,
                        },
                    );
                    if let Some(slide_data) = SLIDE_DATA_GETTER.get_by_segment(&norm) {
                        expand_segment_notes(track, &slide_data, &mut judge_areas);
                    }
                }
            } else {
                let norm = NormalizedSlideSegment::new(
                    segment.shape,
                    NormalizedSlideSegmentParams {
                        start: segment.start,
                        destination: segment.destination,
                    },
                );
                if let Some(slide_data) = SLIDE_DATA_GETTER.get_by_segment(&norm) {
                    expand_segment_notes(track, &slide_data, &mut judge_areas);
                }
            }
        }
    } else {
        let segments: Vec<_> = track
            .segments
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
            .collect();
        let norm_track = NormalizedSlideTrack { segments };

        if let Some(slide_data) = SLIDE_DATA_GETTER.get(&norm_track) {
            expand_segment_notes(track, &slide_data, &mut judge_areas);
        }
    }

    if judge_areas.is_empty() {
        return None;
    }

    let segments = original_slide_track(track).segments;

    let first = &judge_areas[0];
    Some(FlatNote::SlideTrack {
        ts: track.start_ts,
        end_ts: track.start_ts + track.dur,
        x: first.x,
        y: first.y,
        segments,
        judge_areas,
    })
}

fn expand_segment_notes(
    track: &MaterializedSlideTrack,
    slide_data: &maidata::judge::slide_data_getter::SlideData,
    judge_areas: &mut Vec<SlideJudgeArea>,
) {
    let total_dist = slide_data.total_distance();
    if total_dist <= 0.0 {
        return;
    }
    let mut cumul = 0.0;
    for ha in slide_data.iter() {
        let enter_ts = track.start_ts + (cumul / total_dist) * track.dur;
        let ha_dur = ha.push_distance / total_dist * track.dur;
        let exit_ts = enter_ts + ha_dur;

        let pos = centroid_of_sensors(&ha.hit_points);
        judge_areas.push(SlideJudgeArea {
            ts: enter_ts,
            exit_ts,
            sensors: ha.hit_points.iter().map(|s| format!("{s}")).collect(),
            x: pos.0,
            y: pos.1,
        });

        cumul += ha.push_distance + ha.release_distance;
    }
}
