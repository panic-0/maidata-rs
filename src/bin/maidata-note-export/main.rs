mod filter;
mod merge;
mod sensor;
mod slide;
mod types;

use maidata::container::lex_maidata;
use maidata::materialize::{MaterializationContext, MaterializedTapShape, Note};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use types::FlatNote;
use walkdir::WalkDir;

use filter::find_simultaneous_violation;
use merge::merge_chart;
use sensor::{key_position, sensor_position};
use slide::{expand_slide_track, original_slide_track};
use types::{note_time_range, round_json_values, ChartEntry};

struct Args {
    chart_root: String,
    output_path: String,
    raw: bool,
    limit: Option<usize>,
}

fn parse_args() -> Result<Args, String> {
    let mut positional = Vec::new();
    let mut raw = false;
    let mut limit = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--raw" => raw = true,
            "--limit" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--limit requires a chart count".to_string())?;
                limit = Some(parse_limit(&value)?);
            }
            "-h" | "--help" => return Err(
                "usage: maidata-note-export [--raw] [--limit <count>] <chart_root> <output_path>"
                    .to_string(),
            ),
            _ if arg.starts_with("--limit=") => {
                limit = Some(parse_limit(arg.trim_start_matches("--limit="))?);
            }
            _ if arg.starts_with('-') => return Err(format!("unknown option: {arg}")),
            _ => positional.push(arg),
        }
    }

    let chart_root = positional
        .first()
        .cloned()
        .unwrap_or_else(|| ".".to_string());
    let output_path = positional
        .get(1)
        .cloned()
        .unwrap_or_else(|| "notes.jsonl".to_string());

    Ok(Args {
        chart_root,
        output_path,
        raw,
        limit,
    })
}

fn parse_limit(value: &str) -> Result<usize, String> {
    value
        .parse()
        .map_err(|_| format!("invalid --limit value: {value}"))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args().map_err(|msg| {
        eprintln!("{msg}");
        std::io::Error::new(std::io::ErrorKind::InvalidInput, msg)
    })?;

    let label_map = load_labels()?;

    let maidata_files: Vec<PathBuf> = WalkDir::new(&args.chart_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !e.file_type().is_dir() && e.file_name() == "maidata.txt")
        .map(|e| e.into_path())
        .collect();
    eprintln!("Found {} maidata files", maidata_files.len());

    let out_file = std::fs::File::create(&args.output_path)?;
    let mut out = std::io::BufWriter::new(out_file);
    let mut exported = 0usize;
    let mut skipped_multi = 0usize;
    let mut skipped_merge = 0usize;

    'charts: for path in &maidata_files {
        if args.limit.is_some_and(|limit| exported >= limit) {
            break;
        }

        let song_id = extract_song_id(path);

        let numeric_id: String = song_id.split('_').next().unwrap_or(&song_id).to_string();
        if let Ok(id) = numeric_id.parse::<u64>() {
            if id >= 100000 {
                continue;
            }
        }

        let content = maidata::app::read_file(path);
        // TODO
        let (maidata, _state) = lex_maidata(&content);
        // maidata::app::print_state_messages(&state);
        let title = maidata.title().to_string();

        for diff_view in maidata.iter_difficulties() {
            if args.limit.is_some_and(|limit| exported >= limit) {
                break 'charts;
            }

            let diff = diff_view.difficulty();

            let cc = label_map
                .get(&numeric_id)
                .and_then(|m| m.get(&(diff as u8)))
                .copied();

            let min_level: u8 = match diff_view.level() {
                Some(maidata::Level::Normal(lv)) | Some(maidata::Level::Plus(lv)) => lv,
                _ => 0,
            };
            if min_level < 11 {
                continue;
            }

            let offset = diff_view.offset().unwrap_or(0.0);
            let mut mcx = MaterializationContext::with_offset(offset);
            let sp_notes = mcx.materialize_insns(diff_view.iter_insns());
            let mat_notes: Vec<_> = sp_notes.into_iter().map(|sp| sp.into_inner()).collect();

            if mat_notes.is_empty() {
                continue;
            }

            let mut flat_notes: Vec<FlatNote> = Vec::new();
            let mut slides = Vec::new();
            for note in &mat_notes {
                match note {
                    Note::Bpm(_) => {}
                    Note::Tap(tap) => {
                        let is_star = tap.shape == MaterializedTapShape::Star;
                        let pos = key_position(tap.key);
                        flat_notes.push(FlatNote::Tap {
                            ts: tap.ts,
                            key: tap.key.index(),
                            x: pos.0,
                            y: pos.1,
                            is_star,
                        });
                    }
                    Note::Touch(touch) => {
                        let pos = sensor_position(&touch.sensor);
                        flat_notes.push(FlatNote::Touch {
                            ts: touch.ts,
                            sensor: format!("{}", touch.sensor),
                            x: pos.0,
                            y: pos.1,
                        });
                    }
                    Note::Hold(hold) => {
                        let pos = key_position(hold.key);
                        flat_notes.push(FlatNote::Hold {
                            ts: hold.ts,
                            dur: hold.dur,
                            key: hold.key.index(),
                            x: pos.0,
                            y: pos.1,
                        });
                    }
                    Note::TouchHold(th) => {
                        let pos = sensor_position(&th.sensor);
                        flat_notes.push(FlatNote::TouchHold {
                            ts: th.ts,
                            dur: th.dur,
                            sensor: format!("{}", th.sensor),
                            x: pos.0,
                            y: pos.1,
                        });
                    }
                    Note::SlideTrack(track) => {
                        slides.push(original_slide_track(track));
                        if let Some(expanded) = expand_slide_track(track) {
                            flat_notes.push(expanded);
                        }
                    }
                }
            }

            flat_notes.sort_by(|a, b| {
                note_time_range(a)
                    .0
                    .partial_cmp(&note_time_range(b).0)
                    .unwrap()
            });

            let raw_chart = if args.raw {
                Some(mat_notes.clone())
            } else {
                None
            };

            let merge_result = merge_chart(flat_notes);
            let flat_notes = match merge_result {
                Ok(result) => result,
                Err(err) => {
                    eprintln!(
                        "  NOTE WARNING: {song_id} [{diff:?}] cannot merge exported slide graph: {}",
                        err.message
                    );
                    skipped_merge += 1;
                    continue;
                }
            };

            if let Some(violation_ts) = find_simultaneous_violation(&flat_notes) {
                let mut active_notes: Vec<&FlatNote> = Vec::new();
                for note in &flat_notes {
                    let (ts, exit_ts) = note_time_range(note);
                    if ts <= violation_ts && exit_ts >= violation_ts {
                        active_notes.push(note);
                    }
                }
                // TODO: 存在 C判定区 touch/touchhold则不输出
                if active_notes.iter().all(|n| {
                    if let FlatNote::Touch { sensor, .. } | FlatNote::TouchHold { sensor, .. } = n {
                        !sensor.starts_with('C')
                    } else {
                        true
                    }
                }) {
                    eprintln!(
                        "  WARNING: {song_id} [{diff:?}] >2 simultaneous at t={violation_ts:.4} ({} notes overlap):",
                        active_notes.len()
                    );
                    for n in &active_notes {
                        let (ts, exit_ts) = note_time_range(n);
                        eprintln!("    {n:?}  [{ts:.4}..{exit_ts:.4}]");
                    }
                }
                skipped_multi += 1;
                continue;
            }

            let entry = ChartEntry {
                song_id: song_id.clone(),
                title: title.clone(),
                level: cc,
                difficulty: format!("{diff:?}"),
                notes: flat_notes,
                slides,
                raw: raw_chart,
            };
            let mut json_val = serde_json::to_value(&entry)?;
            round_json_values(&mut json_val);
            serde_json::to_writer(&mut out, &json_val)?;
            writeln!(out)?;
            exported += 1;
            if args.limit.is_some_and(|limit| exported >= limit) {
                break 'charts;
            }

            // TODO
            // eprintln!(
            //     "  {}: {} [{diff:?}] → {} notes",
            //     song_id,
            //     title,
            //     entry.notes.len()
            // );
        }
    }

    drop(out);
    eprintln!(
        "Exported {exported} charts, skipped {skipped_multi} (too many simultaneous notes), {skipped_merge} (unsupported slide merge)"
    );
    Ok(())
}

fn extract_song_id(maidata_path: &std::path::Path) -> String {
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

fn load_labels() -> Result<HashMap<String, HashMap<u8, f64>>, Box<dyn std::error::Error>> {
    let cache_path = label_cache_path();
    if let Ok(cached) = fs::read_to_string(&cache_path) {
        match parse_api_songs(&cached) {
            Ok(songs) => {
                eprintln!(
                    "Loaded chart constants from cache: {}",
                    cache_path.display()
                );
                return Ok(label_map_from_api(songs));
            }
            Err(err) => {
                eprintln!(
                    "Ignoring invalid chart-constant cache {}: {err}",
                    cache_path.display()
                );
            }
        }
    }

    eprintln!("Fetching chart constants from diving-fish...");
    let json = ureq::get("https://www.diving-fish.com/api/maimaidxprober/music_data")
        .call()?
        .into_string()?;
    let songs = parse_api_songs(&json)?;
    if let Some(parent) = cache_path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            eprintln!(
                "WARNING: cannot create cache dir {}: {err}",
                parent.display()
            );
        }
    }
    if let Err(err) = fs::write(&cache_path, &json) {
        eprintln!(
            "WARNING: cannot write chart-constant cache {}: {err}",
            cache_path.display()
        );
    } else {
        eprintln!("Cached chart constants at {}", cache_path.display());
    }
    Ok(label_map_from_api(songs))
}

fn label_cache_path() -> PathBuf {
    if let Ok(path) = std::env::var("MAIDATA_NOTE_EXPORT_LABEL_CACHE") {
        return PathBuf::from(path);
    }
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        return PathBuf::from(local_app_data)
            .join("maidata-note-export")
            .join("diving-fish-music-data.json");
    }
    if let Ok(xdg_cache_home) = std::env::var("XDG_CACHE_HOME") {
        return PathBuf::from(xdg_cache_home)
            .join("maidata-note-export")
            .join("diving-fish-music-data.json");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".cache")
            .join("maidata-note-export")
            .join("diving-fish-music-data.json");
    }
    std::env::temp_dir()
        .join("maidata-note-export")
        .join("diving-fish-music-data.json")
}

fn parse_api_songs(json: &str) -> serde_json::Result<Vec<ApiSong>> {
    serde_json::from_str(json.trim_start_matches('\u{feff}'))
}

fn label_map_from_api(resp: Vec<ApiSong>) -> HashMap<String, HashMap<u8, f64>> {
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
    map
}
