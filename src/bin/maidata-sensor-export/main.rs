use maidata::container::lex_maidata;
use maidata::heatmap::{HeatmapEncoder, NUM_CHANNELS, NUM_SENSORS};
use maidata::materialize::MaterializationContext;
use maidata::Difficulty;
use ndarray::Array3;
use ndarray_npy::write_npy;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    // Mode 1: maidata.txt path + difficulty number
    // usage: maidata-sensor-export <maidata.txt> <difficulty_num> <output.npy>
    // difficulty_num: 1=Easy, 2=Basic, 3=Advanced, 4=Expert, 5=Master, 6=ReMaster, 7=Original
    if args.len() == 4 && std::path::Path::new(&args[1]).exists() {
        let maidata_path = &args[1];
        let diff_num: u8 = args[2].parse()?;
        let output_path = &args[3];

        let difficulty = match diff_num {
            1 => Difficulty::Easy,
            2 => Difficulty::Basic,
            3 => Difficulty::Advanced,
            4 => Difficulty::Expert,
            5 => Difficulty::Master,
            6 => Difficulty::ReMaster,
            7 => Difficulty::Original,
            _ => {
                eprintln!("Invalid difficulty number: {diff_num}. Use 1-7.");
                std::process::exit(1);
            }
        };

        let content = maidata::app::read_file(std::path::Path::new(maidata_path));
        let (maidata, state) = lex_maidata(&content);
        maidata::app::print_state_messages(&state);

        let diff_view = maidata
            .iter_difficulties()
            .find(|d| d.difficulty() == difficulty)
            .ok_or_else(|| format!("Difficulty {diff_num} not found in maidata file"))?;

        let offset = diff_view.offset().unwrap_or(0.0);
        let mut mcx = MaterializationContext::with_offset(offset);
        let sp_notes = mcx.materialize_insns(diff_view.iter_insns());
        let notes: Vec<_> = sp_notes.into_iter().map(|sp| sp.into_inner()).collect();

        if notes.is_empty() {
            eprintln!("Error: no materialized notes");
            std::process::exit(1);
        }

        let encoder = HeatmapEncoder::new();
        let frames_f32 = encoder.encode(&notes);
        let (t, _s, _c) = frames_f32.dim();

        let mut data = Vec::with_capacity(t * NUM_SENSORS * NUM_CHANNELS);
        for fi in 0..t {
            for si in 0..NUM_SENSORS {
                for ch in 0..NUM_CHANNELS {
                    let v = frames_f32[[fi, si, ch]];
                    data.push((v * 255.0).min(255.0) as u8);
                }
            }
        }
        let frames_u8 = Array3::from_shape_vec((t, NUM_SENSORS, NUM_CHANNELS), data).unwrap();

        eprintln!(
            "Encoded: [{} x 33 x 5] ({:.1}s, offset={:.1}s)",
            t,
            t as f64 * encoder.frame_dt(),
            offset
        );

        write_npy(output_path, &frames_u8)?;
        return Ok(());
    }

    // Mode 2: raw chart text (legacy, no offset)
    // usage: maidata-sensor-export <chart_text_or_filepath> <output.npy>
    if args.len() != 3 {
        eprintln!("usage:");
        eprintln!("  {} <maidata.txt> <difficulty_num> <output.npy>", args[0]);
        eprintln!("  {} <chart_text_or_file> <output.npy>", args[0]);
        std::process::exit(1);
    }

    let input = &args[1];
    let output_path = &args[2];

    let chart_text = if input == "-" {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        buf
    } else if std::path::Path::new(input).exists() {
        std::fs::read_to_string(input)?
    } else {
        input.clone()
    };

    let (insns, state) = maidata::container::parse_maidata_insns(&chart_text);
    maidata::app::print_state_messages(&state);

    if insns.is_empty() {
        eprintln!("Error: no instructions parsed from input");
        std::process::exit(1);
    }

    let mut mcx = MaterializationContext::with_offset(0.0);
    let sp_notes = mcx.materialize_insns(&insns);
    let notes: Vec<_> = sp_notes.into_iter().map(|sp| sp.into_inner()).collect();

    if notes.is_empty() {
        eprintln!("Error: no materialized notes");
        std::process::exit(1);
    }

    let encoder = HeatmapEncoder::new();
    let frames_f32 = encoder.encode(&notes);

    let (t, _s, _c) = frames_f32.dim();
    let mut data = Vec::with_capacity(t * NUM_SENSORS * NUM_CHANNELS);
    for fi in 0..t {
        for si in 0..NUM_SENSORS {
            for ch in 0..NUM_CHANNELS {
                let v = frames_f32[[fi, si, ch]];
                data.push((v * 255.0).min(255.0) as u8);
            }
        }
    }
    let frames_u8 = Array3::from_shape_vec((t, NUM_SENSORS, NUM_CHANNELS), data).unwrap();

    eprintln!(
        "Encoded: [{} x 33 x 5] ({:.1}s)",
        t,
        t as f64 * encoder.frame_dt()
    );

    write_npy(output_path, &frames_u8)?;
    Ok(())
}
