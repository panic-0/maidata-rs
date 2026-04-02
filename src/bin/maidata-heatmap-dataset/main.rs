use maidata::heatmap::{HeatmapEncoder, NUM_CHANNELS, NUM_SENSORS};
use maidata::materialize::MaterializationContext;
use ndarray::Array3;
use ndarray_npy::write_npy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if !(3..=4).contains(&args.len()) {
        eprintln!("usage: {} <chart_root> <output_dir> [limit]", args[0]);
        std::process::exit(1);
    }
    let chart_root = &args[1];
    let output_dir = &args[2];
    let limit: Option<usize> = args.get(3).map(|s| s.parse()).transpose()?;
    std::fs::create_dir_all(output_dir)?;

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
                .get(&song_id)
                .and_then(|m| m.get(&(diff as u8)))
                .copied();

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
                frame_offsets,
            });

            eprintln!(
                "  {}: {} [{diff:?}] → {n}/{total_frames} frames",
                song_id,
                maidata.title(),
            );
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
