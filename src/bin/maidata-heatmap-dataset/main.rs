use maidata::heatmap::encode::compact_quantized_frames;
use maidata::heatmap::HeatmapEncoder;
use maidata::materialize::{
    MaterializationContext, MaterializedHold, MaterializedSlideSegment, MaterializedSlideTrack,
    MaterializedTap, MaterializedTouch, MaterializedTouchHold, Note,
};
use maidata::transform::transform::{Transformable, Transformer};
use ndarray_npy::write_npy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const MIRROR: Transformer = Transformer {
    rotation: 0,
    flip: true,
    vertical_flip: false,
};

const VERTICAL_FLIP: Transformer = Transformer {
    rotation: 0,
    flip: false,
    vertical_flip: true,
};

const MIRROR_VERTICAL_FLIP: Transformer = Transformer {
    rotation: 0,
    flip: true,
    vertical_flip: true,
};

fn transform_notes(notes: &[Note], transformer: Transformer) -> Vec<Note> {
    notes
        .iter()
        .map(|note| match note {
            Note::Bpm(b) => Note::Bpm(*b),
            Note::Tap(p) => Note::Tap(MaterializedTap {
                key: p.key.transform(transformer),
                ..*p
            }),
            Note::Touch(p) => Note::Touch(MaterializedTouch {
                sensor: p.sensor.transform(transformer),
                ..*p
            }),
            Note::Hold(p) => Note::Hold(MaterializedHold {
                key: p.key.transform(transformer),
                ..*p
            }),
            Note::TouchHold(p) => Note::TouchHold(MaterializedTouchHold {
                sensor: p.sensor.transform(transformer),
                ..*p
            }),
            Note::SlideTrack(p) => Note::SlideTrack(MaterializedSlideTrack {
                segments: p
                    .segments
                    .iter()
                    .map(|s| MaterializedSlideSegment {
                        start: s.start.transform(transformer),
                        destination: s.destination.transform(transformer),
                        shape: maidata::transform::NormalizedSlideSegment::new(
                            s.shape,
                            maidata::transform::NormalizedSlideSegmentParams {
                                start: s.start,
                                destination: s.destination,
                            },
                        )
                        .transform(transformer)
                        .shape(),
                    })
                    .collect(),
                ..p.clone()
            }),
        })
        .collect()
}

fn offset_song_id(song_id: &str, offset: u64) -> String {
    let numeric: String = song_id.split('_').next().unwrap_or(song_id).to_string();
    let rest = &song_id[numeric.len()..];
    if let Ok(id) = numeric.parse::<u64>() {
        format!("{}{}", id + offset, rest)
    } else {
        format!("{offset}_{song_id}")
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut chart_root = "";
    let mut output_dir = "";
    let mut limit: Option<usize> = None;
    let mut mirror_offset: Option<u64> = None;
    let mut vertical_flip_offset: Option<u64> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--mirror" => {
                let (offset, consumed) = parse_optional_offset(&args, i, 10_000_000);
                mirror_offset = Some(offset);
                i += consumed;
            }
            "--vertical-flip" | "--flip-vertical" | "--flip-y" => {
                let (offset, consumed) = parse_optional_offset(&args, i, 20_000_000);
                vertical_flip_offset = Some(offset);
                i += consumed;
            }
            _ if chart_root.is_empty() => chart_root = &args[i],
            _ if output_dir.is_empty() => output_dir = &args[i],
            _ => limit = Some(args[i].parse()?),
        }
        i += 1;
    }

    if chart_root.is_empty() || output_dir.is_empty() {
        eprintln!(
            "usage: {} [--mirror [offset]] [--vertical-flip [offset]] <chart_root> <output_dir> [limit]",
            args[0]
        );
        eprintln!("  --mirror [offset]         append mirrored charts (default offset: 10000000)");
        eprintln!("  --vertical-flip [offset]  append vertically flipped charts (default offset: 20000000)");
        std::process::exit(1);
    }
    std::fs::create_dir_all(output_dir)?;

    let mut variants = Vec::new();
    if let Some(offset) = mirror_offset.filter(|&offset| offset > 0) {
        variants.push(AugmentationVariant {
            transformer: MIRROR,
            offset,
        });
    }
    if let Some(offset) = vertical_flip_offset.filter(|&offset| offset > 0) {
        variants.push(AugmentationVariant {
            transformer: VERTICAL_FLIP,
            offset,
        });
    }
    if let (Some(mirror_offset), Some(vertical_flip_offset)) = (mirror_offset, vertical_flip_offset)
    {
        if mirror_offset > 0 && vertical_flip_offset > 0 {
            variants.push(AugmentationVariant {
                transformer: MIRROR_VERTICAL_FLIP,
                offset: mirror_offset + vertical_flip_offset,
            });
        }
    }

    eprintln!("Fetching chart constants from diving-fish...");
    let label_map = fetch_labels()?;

    let maidata_files: Vec<PathBuf> = WalkDir::new(chart_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !e.file_type().is_dir() && e.file_name() == "maidata.txt")
        .map(|e| e.into_path())
        .collect();
    eprintln!("Found {} maidata files", maidata_files.len());

    let encoder = HeatmapEncoder::new();
    let mut manifest: Vec<ManifestEntry> = Vec::new();
    let mut songs_processed = 0usize;

    for path in &maidata_files {
        let song_id = extract_song_id(path, chart_root);

        // Skip utage (宴会场) songs: ID >= 100000
        let numeric_id: String = song_id.split('_').next().unwrap_or(&song_id).to_string();
        if let Ok(id) = numeric_id.parse::<u64>() {
            if id >= 100000 {
                continue;
            }
        }

        if limit == Some(songs_processed) {
            break;
        }

        let content = maidata::app::read_file(path);
        let (maidata, state) = maidata::container::lex_maidata(&content);
        maidata::app::print_state_messages(&state);

        for diff_view in maidata.iter_difficulties() {
            let diff = diff_view.difficulty();
            let cc = label_map
                .get(&numeric_id)
                .and_then(|m| m.get(&(diff as u8)))
                .copied();

            // // Skip charts below level 10
            // let min_level: u8 = match diff_view.level() {
            //     Some(maidata::Level::Normal(lv)) | Some(maidata::Level::Plus(lv)) => lv,
            //     _ => 0,
            // };
            // if min_level < 10 {
            //     continue;
            // }

            let offset = diff_view.offset().unwrap_or(0.0);

            let mut mcx = MaterializationContext::with_offset(offset);
            let sp_notes = mcx.materialize_insns(diff_view.iter_insns());
            let notes: Vec<_> = sp_notes.into_iter().map(|sp| sp.into_inner()).collect();

            if notes.is_empty() {
                continue;
            }

            let sample = encode_sample(&encoder, &notes)?;
            if sample.n == 0 {
                continue;
            }

            let filename = format!("{}_{}.npy", song_id, diff_discriminant(diff));
            let out_path = PathBuf::from(output_dir).join(&filename);
            write_npy(&out_path, &sample.dense_u8)?;

            manifest.push(ManifestEntry {
                song_id: song_id.clone(),
                difficulty: format!("{diff:?}"),
                chart_constant: cc,
                file: filename,
                total_frames: sample.total_frames,
                frame_dt: encoder.frame_dt(),
                frame_offsets: sample.frame_offsets,
            });

            eprintln!(
                "  {}: {} [{diff:?}] → {n}/{total_frames} frames",
                song_id,
                maidata.title(),
                n = sample.n,
                total_frames = sample.total_frames,
            );

            for variant in &variants {
                let augmented_notes = transform_notes(&notes, variant.transformer);
                let augmented_sample = encode_sample(&encoder, &augmented_notes)?;
                if augmented_sample.n == 0 {
                    continue;
                }
                let augmented_id = offset_song_id(&song_id, variant.offset);
                let augmented_file = format!("{}_{}.npy", augmented_id, diff_discriminant(diff));
                let augmented_path = PathBuf::from(output_dir).join(&augmented_file);
                write_npy(&augmented_path, &augmented_sample.dense_u8)?;

                manifest.push(ManifestEntry {
                    song_id: augmented_id,
                    difficulty: format!("{diff:?}"),
                    chart_constant: cc,
                    file: augmented_file,
                    total_frames: augmented_sample.total_frames,
                    frame_dt: encoder.frame_dt(),
                    frame_offsets: augmented_sample.frame_offsets,
                });
            }
        }

        songs_processed += 1;
    }

    let manifest_path = PathBuf::from(output_dir).join("manifest.json");
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(&manifest_path, manifest_json)?;

    eprintln!("Exported {} samples", manifest.len());
    Ok(())
}

fn parse_optional_offset(args: &[String], flag_index: usize, default: u64) -> (u64, usize) {
    match args.get(flag_index + 1).and_then(|s| s.parse().ok()) {
        Some(offset) => (offset, 1),
        None => (default, 0),
    }
}

struct AugmentationVariant {
    transformer: Transformer,
    offset: u64,
}

struct EncodedSample {
    dense_u8: ndarray::Array2<u8>,
    total_frames: usize,
    frame_offsets: Vec<u32>,
    n: usize,
}

fn encode_sample(
    encoder: &HeatmapEncoder,
    notes: &[Note],
) -> Result<EncodedSample, Box<dyn std::error::Error>> {
    let frames_f32 = encoder.encode(notes);
    let total_frames = frames_f32.dim().0;
    let (dense_u8, frame_offsets) = compact_quantized_frames(&frames_f32)?;
    let n = dense_u8.dim().0;
    Ok(EncodedSample {
        dense_u8,
        total_frames,
        frame_offsets,
        n,
    })
}

fn diff_discriminant(d: maidata::Difficulty) -> &'static str {
    use maidata::Difficulty::*;
    match d {
        Easy => "Easy",
        Basic => "Basic",
        Advanced => "Advanced",
        Expert => "Expert",
        Master => "Master",
        ReMaster => "ReMaster",
        Original => "Original",
    }
}

fn extract_song_id(maidata_path: &Path, _chart_root: &str) -> String {
    maidata_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

// --- Label data from diving-fish API ---

#[derive(Deserialize)]
struct ApiSong {
    id: String,
    ds: Vec<f64>,
}

fn fetch_labels() -> Result<HashMap<String, HashMap<u8, f64>>, Box<dyn std::error::Error>> {
    let resp: Vec<ApiSong> = ureq::get("https://www.diving-fish.com/api/maimaidxprober/music_data")
        .call()?
        .into_json()?;

    let mut map = HashMap::new();
    for song in resp {
        let mut diff_map = HashMap::new();

        let diff_ids: Vec<u8> = match song.ds.len() {
            5 => vec![2, 3, 4, 5, 6],
            4 => vec![2, 3, 4, 5],
            _ => continue,
        };

        for (i, &d) in diff_ids.iter().enumerate() {
            if let Some(&cc) = song.ds.get(i) {
                diff_map.insert(d, cc);
            }
        }
        map.insert(song.id, diff_map);
    }
    Ok(map)
}

#[derive(Serialize)]
struct ManifestEntry {
    song_id: String,
    difficulty: String,
    chart_constant: Option<f64>,
    file: String,
    total_frames: usize,
    frame_dt: f64,
    frame_offsets: Vec<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use maidata::heatmap::encode::*;
    use maidata::insn::{Key, TouchSensor};
    use maidata::materialize::MaterializedTapShape;

    #[test]
    fn test_mirror_touch_sensor_e1_e2() {
        let e1 = TouchSensor::new('E', Some(0)).unwrap();
        let e2 = TouchSensor::new('E', Some(1)).unwrap();
        let e8 = TouchSensor::new('E', Some(7)).unwrap();
        assert_eq!(e1.transform(MIRROR), e1, "E1 mirror should be E1");
        assert_eq!(e2.transform(MIRROR), e8, "E2 mirror should be E8");
    }

    #[test]
    fn test_vertical_flip_tap_and_ab_mapping() {
        assert_eq!(Key::new(0).unwrap().transform(VERTICAL_FLIP).index(), 3);
        assert_eq!(Key::new(1).unwrap().transform(VERTICAL_FLIP).index(), 2);
        assert_eq!(Key::new(4).unwrap().transform(VERTICAL_FLIP).index(), 7);
        assert_eq!(Key::new(5).unwrap().transform(VERTICAL_FLIP).index(), 6);

        let a1 = TouchSensor::new('A', Some(0)).unwrap();
        let a4 = TouchSensor::new('A', Some(3)).unwrap();
        let b6 = TouchSensor::new('B', Some(5)).unwrap();
        let b7 = TouchSensor::new('B', Some(6)).unwrap();
        assert_eq!(a1.transform(VERTICAL_FLIP), a4);
        assert_eq!(b6.transform(VERTICAL_FLIP), b7);
    }

    #[test]
    fn test_vertical_flip_de_mapping() {
        let d1 = TouchSensor::new('D', Some(0)).unwrap();
        let d5 = TouchSensor::new('D', Some(4)).unwrap();
        let e3 = TouchSensor::new('E', Some(2)).unwrap();
        let e6 = TouchSensor::new('E', Some(5)).unwrap();
        let e8 = TouchSensor::new('E', Some(7)).unwrap();
        assert_eq!(d1.transform(VERTICAL_FLIP), d5);
        assert_eq!(e3.transform(VERTICAL_FLIP), e3);
        assert_eq!(e6.transform(VERTICAL_FLIP), e8);
    }

    #[test]
    fn test_mirror_tap_sensor_index() {
        // Tap on key 0 (A1, sensor 0) should mirror to key 7 (A8, sensor 7)
        let encoder = HeatmapEncoder::new();
        let notes = vec![Note::Tap(MaterializedTap {
            ts: 0.0,
            key: Key::new(0).unwrap(),
            shape: MaterializedTapShape::Ring,
            is_break: false,
            is_ex: false,
            is_each: false,
        })];
        let original = encoder.encode(&notes);
        let mirrored = encoder.encode(&transform_notes(&notes, MIRROR));

        // Original: key 0 has tap; mirrored: key 7 has tap
        assert!(original[[0, TAP_FEATURE_OFFSET]] > 0.0);
        assert_eq!(original[[0, TAP_FEATURE_OFFSET + 7]], 0.0);
        assert!(mirrored[[0, TAP_FEATURE_OFFSET + 7]] > 0.0);
        assert_eq!(mirrored[[0, TAP_FEATURE_OFFSET]], 0.0);
    }

    #[test]
    fn test_mirror_touch_sensor_index() {
        // Touch on E2 (sensor 26) should mirror to E8 (sensor 32)
        let encoder = HeatmapEncoder::new();
        let notes = vec![Note::Touch(MaterializedTouch {
            ts: 0.0,
            sensor: TouchSensor::new('E', Some(1)).unwrap(),
            is_each: false,
        })];
        let original = encoder.encode(&notes);
        let mirrored = encoder.encode(&transform_notes(&notes, MIRROR));

        assert!(original[[0, TOUCH_FEATURE_OFFSET + 26]] > 0.0);
        assert!(mirrored[[0, TOUCH_FEATURE_OFFSET + 32]] > 0.0);
    }

    #[test]
    fn test_mirror_hold_includes_tap_head() {
        // Hold on key 0 should mirror to key 7, both tap head and hold body
        let encoder = HeatmapEncoder::new();
        let notes = vec![Note::Hold(MaterializedHold {
            ts: 0.0,
            dur: 0.3,
            key: Key::new(0).unwrap(),
            is_break: false,
            is_ex: false,
            is_each: false,
        })];
        let mirrored = encoder.encode(&transform_notes(&notes, MIRROR));

        // Mirrored hold head (tap) on sensor 7
        assert!(
            mirrored[[0, TAP_FEATURE_OFFSET + 7]] > 0.0,
            "mirrored hold head"
        );
        // Mirrored hold body occupies one hand.
        assert!(
            mirrored[[0, HOLD_FEATURE_OFFSET]] > 0.0,
            "mirrored hold body"
        );
    }
}
