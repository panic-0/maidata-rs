use maidata::heatmap::{HeatmapEncoder, NUM_CHANNELS, NUM_SENSORS};
use maidata::materialize::{
    MaterializationContext, MaterializedHold, MaterializedSlideSegment, MaterializedSlideTrack,
    MaterializedTap, MaterializedTouch, MaterializedTouchHold, Note,
};
use maidata::transform::transform::{Transformable, Transformer};
use ndarray::Array3;
use ndarray_npy::write_npy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const MIRROR: Transformer = Transformer {
    rotation: 0,
    flip: true,
};

fn mirror_notes(notes: &[Note]) -> Vec<Note> {
    notes
        .iter()
        .map(|note| match note {
            Note::Bpm(b) => Note::Bpm(*b),
            Note::Tap(p) => Note::Tap(MaterializedTap {
                key: p.key.transform(MIRROR),
                ..*p
            }),
            Note::Touch(p) => Note::Touch(MaterializedTouch {
                sensor: p.sensor.transform(MIRROR),
                ..*p
            }),
            Note::Hold(p) => Note::Hold(MaterializedHold {
                key: p.key.transform(MIRROR),
                ..*p
            }),
            Note::TouchHold(p) => Note::TouchHold(MaterializedTouchHold {
                sensor: p.sensor.transform(MIRROR),
                ..*p
            }),
            Note::SlideTrack(p) => Note::SlideTrack(MaterializedSlideTrack {
                segments: p
                    .segments
                    .iter()
                    .map(|s| MaterializedSlideSegment {
                        start: s.start.transform(MIRROR),
                        destination: s.destination.transform(MIRROR),
                        shape: maidata::transform::NormalizedSlideSegment::new(
                            s.shape,
                            maidata::transform::NormalizedSlideSegmentParams {
                                start: s.start,
                                destination: s.destination,
                            },
                        )
                        .transform(MIRROR)
                        .shape(),
                    })
                    .collect(),
                ..p.clone()
            }),
        })
        .collect()
}

fn mirror_song_id(song_id: &str, offset: u64) -> String {
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

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--mirror" => {
                mirror_offset = Some(
                    args.get(i + 1)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(10_000_000),
                )
            }
            _ if chart_root.is_empty() => chart_root = &args[i],
            _ if output_dir.is_empty() => output_dir = &args[i],
            _ => limit = Some(args[i].parse()?),
        }
        i += 1;
    }

    // --mirror with explicit offset: already parsed above

    if chart_root.is_empty() || output_dir.is_empty() {
        eprintln!(
            "usage: {} [--mirror [offset]] <chart_root> <output_dir> [limit]",
            args[0]
        );
        eprintln!("  --mirror [offset]  append mirrored charts (default offset: 10000000)");
        std::process::exit(1);
    }
    std::fs::create_dir_all(output_dir)?;

    let mirror_offset = mirror_offset.unwrap_or(0);

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

            // Skip charts below level 10
            let min_level: u8 = match diff_view.level() {
                Some(maidata::Level::Normal(lv)) | Some(maidata::Level::Plus(lv)) => lv,
                _ => 0,
            };
            if min_level < 10 {
                continue;
            }

            let offset = diff_view.offset().unwrap_or(0.0);

            let mut mcx = MaterializationContext::with_offset(offset);
            let sp_notes = mcx.materialize_insns(diff_view.iter_insns());
            let notes: Vec<_> = sp_notes.into_iter().map(|sp| sp.into_inner()).collect();

            if notes.is_empty() {
                continue;
            }

            let frames_f32 = encoder.encode(&notes);
            let total_frames = frames_f32.dim().0;

            // Keep only non-empty frames, quantize to u8
            let (dense_u8, frame_offsets) = compact_frames(&frames_f32);
            let n = dense_u8.dim().0;
            if n == 0 {
                continue;
            }

            let filename = format!("{}_{}.npy", song_id, diff_discriminant(diff));
            let out_path = PathBuf::from(output_dir).join(&filename);
            write_npy(&out_path, &dense_u8)?;

            manifest.push(ManifestEntry {
                song_id: song_id.clone(),
                difficulty: format!("{diff:?}"),
                chart_constant: cc,
                file: filename,
                total_frames,
                frame_dt: encoder.frame_dt(),
                frame_offsets: frame_offsets.clone(),
            });

            eprintln!(
                "  {}: {} [{diff:?}] → {n}/{total_frames} frames",
                song_id,
                maidata.title(),
            );

            // Mirror augmentation
            if mirror_offset > 0 {
                let mirrored_notes = mirror_notes(&notes);
                let mirrored_frames = encoder.encode(&mirrored_notes);
                let (mirrored_u8, _) = compact_frames(&mirrored_frames);
                let mirrored_total = mirrored_frames.dim().0;
                let n_m = mirrored_u8.dim().0;
                if n_m == 0 {
                    continue;
                }
                let mirror_id = mirror_song_id(&song_id, mirror_offset);
                let mirror_file = format!("{}_{}.npy", mirror_id, diff_discriminant(diff));
                let mirror_path = PathBuf::from(output_dir).join(&mirror_file);
                write_npy(&mirror_path, &mirrored_u8)?;

                manifest.push(ManifestEntry {
                    song_id: mirror_id,
                    difficulty: format!("{diff:?}"),
                    chart_constant: cc,
                    file: mirror_file,
                    total_frames: mirrored_total,
                    frame_dt: encoder.frame_dt(),
                    frame_offsets: frame_offsets.clone(),
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

/// Filter out all-zero frames, quantize to u8 (×255, clamp 255).
/// Returns dense Array3<u8> [N, 33, 5] and original frame indices.
fn compact_frames(frames: &Array3<f32>) -> (Array3<u8>, Vec<u32>) {
    let (t, _sensors, _ch) = frames.dim();

    let mut indices = Vec::new();
    let mut data = Vec::new();

    for fi in 0..t {
        // Check if frame has any non-zero value
        let mut has = false;
        for si in 0..NUM_SENSORS {
            for ch in 0..NUM_CHANNELS {
                if frames[[fi, si, ch]] != 0.0 {
                    has = true;
                    break;
                }
            }
            if has {
                break;
            }
        }

        if !has {
            continue;
        }

        indices.push(fi as u32);
        for si in 0..NUM_SENSORS {
            for ch in 0..NUM_CHANNELS {
                let v = frames[[fi, si, ch]];
                data.push((v * 255.0).min(255.0) as u8);
            }
        }
    }

    let n = indices.len();
    let arr = Array3::from_shape_vec((n, NUM_SENSORS, NUM_CHANNELS), data).unwrap();
    (arr, indices)
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
        let mirrored = encoder.encode(&mirror_notes(&notes));

        // Original: sensor 0 has tap; mirrored: sensor 7 has tap
        assert!(original[[0, 0, CH_TAP_INSTANT]] > 0.0);
        assert_eq!(original[[0, 7, CH_TAP_INSTANT]], 0.0);
        assert!(mirrored[[0, 7, CH_TAP_INSTANT]] > 0.0);
        assert_eq!(mirrored[[0, 0, CH_TAP_INSTANT]], 0.0);
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
        let mirrored = encoder.encode(&mirror_notes(&notes));

        assert!(original[[0, 26, CH_TOUCH_INSTANT]] > 0.0);
        assert!(mirrored[[0, 32, CH_TOUCH_INSTANT]] > 0.0);
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
        let mirrored = encoder.encode(&mirror_notes(&notes));

        // Mirrored hold head (tap) on sensor 7
        assert!(mirrored[[0, 7, CH_TAP_INSTANT]] > 0.0, "mirrored hold head");
        // Mirrored hold body on sensor 7
        assert!(mirrored[[0, 7, CH_HOLD]] > 0.0, "mirrored hold body");
    }
}
