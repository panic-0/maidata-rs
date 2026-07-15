use super::slide_data::{
    HitAreaData, BEND_L_DATA, CIRCLE_L_DATA, CORNER_DATA, CURVE_L_DATA, FAN_DATA, SKIP_L_DATA,
    STRAIGHT_DATA, THUNDER_L_DATA,
};
use crate::insn::{Key, TouchSensor};
use crate::transform;
use crate::transform::{NormalizedSlideSegment, NormalizedSlideSegmentShape, NormalizedSlideTrack};
use lazy_static::lazy_static;
use transform::transform::{Transformable, Transformer};

use enum_map::EnumMap;

#[derive(Clone, Debug)]
pub struct HitArea {
    pub hit_points: Vec<TouchSensor>,
    pub push_distance: f64,
    pub release_distance: f64,
}
// pub type SlideData = Vec<HitArea>;
#[derive(Clone, Debug)]
pub struct SlideData(Vec<HitArea>);

impl SlideData {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn into_path(self) -> Vec<Vec<TouchSensor>> {
        self.0
            .into_iter()
            .map(|hit_area| hit_area.hit_points)
            .collect()
    }

    pub fn total_distance(&self) -> f64 {
        self.0
            .iter()
            .map(|hit_area| hit_area.push_distance + hit_area.release_distance)
            .sum()
    }
}

impl Default for SlideData {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Vec<HitArea>> for SlideData {
    fn from(data: Vec<HitArea>) -> Self {
        Self(data)
    }
}

impl std::ops::Deref for SlideData {
    type Target = Vec<HitArea>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct SlideDataGetter {
    slide_data_list: EnumMap<NormalizedSlideSegmentShape, [[Option<SlideData>; 8]; 8]>,
}

impl SlideDataGetter {
    fn add_data(
        &mut self,
        shape: NormalizedSlideSegmentShape,
        start: u8,
        destination: u8,
        data: SlideData,
    ) {
        self.slide_data_list[shape][start as usize][destination as usize] = Some(data);
    }

    fn add_shape_data(
        &mut self,
        shape: NormalizedSlideSegmentShape,
        data: &[&[HitAreaData]; 8],
        flip: bool,
    ) {
        for (destination, hit_areas) in data.iter().enumerate() {
            if hit_areas.is_empty() {
                continue;
            }
            for rotation in 0..8 {
                let transformer = Transformer {
                    rotation,
                    flip,
                    vertical_flip: false,
                };
                let start = Key::new(0).unwrap().transform(transformer).index();
                let destination = Key::new(destination as u8)
                    .unwrap()
                    .transform(transformer)
                    .index();
                let data = hit_areas
                    .iter()
                    .map(|hit_area_data| {
                        let hit_points = hit_area_data
                            .hit_points
                            .iter()
                            .map(|sensor| sensor.transform(transformer))
                            .collect();
                        HitArea {
                            hit_points,
                            push_distance: hit_area_data.push_distance,
                            release_distance: hit_area_data.release_distance,
                        }
                    })
                    .collect::<Vec<_>>()
                    .into();
                self.add_data(shape, start, destination, data);
            }
        }
    }

    fn new() -> Self {
        let mut result = Self {
            slide_data_list: EnumMap::default(),
        };
        result.add_shape_data(NormalizedSlideSegmentShape::Straight, &STRAIGHT_DATA, false);
        result.add_shape_data(NormalizedSlideSegmentShape::CircleL, &CIRCLE_L_DATA, false);
        result.add_shape_data(NormalizedSlideSegmentShape::CircleR, &CIRCLE_L_DATA, true);
        result.add_shape_data(NormalizedSlideSegmentShape::CurveL, &CURVE_L_DATA, false);
        result.add_shape_data(NormalizedSlideSegmentShape::CurveR, &CURVE_L_DATA, true);
        result.add_shape_data(
            NormalizedSlideSegmentShape::ThunderL,
            &THUNDER_L_DATA,
            false,
        );
        result.add_shape_data(NormalizedSlideSegmentShape::ThunderR, &THUNDER_L_DATA, true);
        result.add_shape_data(NormalizedSlideSegmentShape::Corner, &CORNER_DATA, false);
        result.add_shape_data(NormalizedSlideSegmentShape::BendL, &BEND_L_DATA, false);
        result.add_shape_data(NormalizedSlideSegmentShape::BendR, &BEND_L_DATA, true);
        result.add_shape_data(NormalizedSlideSegmentShape::SkipL, &SKIP_L_DATA, false);
        result.add_shape_data(NormalizedSlideSegmentShape::SkipR, &SKIP_L_DATA, true);
        result.add_shape_data(NormalizedSlideSegmentShape::Fan, &FAN_DATA, false);
        result
    }

    pub fn get_by_segment(&self, segment: &NormalizedSlideSegment) -> Option<SlideData> {
        self.slide_data_list[segment.shape()][segment.params().start.index() as usize]
            [segment.params().destination.index() as usize]
            .clone()
    }

    // pay attention to Fan Slide
    pub fn get(&self, track: &NormalizedSlideTrack) -> Option<SlideData> {
        let mut result = SlideData::new();
        for segment in &track.segments {
            let mut data = self.get_by_segment(segment)?;
            assert!(!data.is_empty());
            if !result.is_empty() {
                assert!(result.last().unwrap().hit_points.len() == 1);
                let push_distance = result.0.pop().unwrap().push_distance;
                data.0.first_mut().unwrap().push_distance += push_distance;
            }
            result.0.extend(data.0);
        }
        Some(result)
    }

    pub fn get_path_by_segment(
        &self,
        segment: &NormalizedSlideSegment,
    ) -> Option<Vec<Vec<TouchSensor>>> {
        self.get_by_segment(segment).map(|data| data.into_path())
    }

    pub fn get_path(&self, track: &NormalizedSlideTrack) -> Option<Vec<Vec<TouchSensor>>> {
        self.get(track).map(|data| data.into_path())
    }
}

lazy_static! {
    pub static ref SLIDE_DATA_GETTER: SlideDataGetter = SlideDataGetter::new();
}
