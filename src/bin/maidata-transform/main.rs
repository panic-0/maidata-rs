use maidata::insn::RawInsn;
use maidata::transform::{
    normalize::normalize_note,
    transform::{transform_note, Transformer},
    NormalizedNote,
};
use maidata::AssociatedBeatmapData;
use maidata::Level;
use std::collections::HashMap;
use std::ops::Deref;
use std::time::Instant;
use walkdir::WalkDir;

fn minimal(group: &[Vec<NormalizedNote>]) -> Vec<Vec<NormalizedNote>> {
    let mut result = group.to_owned();
    for rotation in 0..8 {
        for flip in [false, true] {
            result = result.min(
                group
                    .iter()
                    .map(|bundle| {
                        bundle
                            .iter()
                            .map(|x| transform_note(x, Transformer { rotation, flip }))
                            .collect()
                    })
                    .collect::<Vec<_>>(),
            );
        }
    }
    result
}

fn main() {
    let dir = std::env::args().nth(1).expect("usage: $0 <path/to/charts>");

    let start = Instant::now();

    const K: usize = 1;

    let mut groups = Vec::new();
    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !e.file_type().is_dir())
    {
        if entry.file_name() == "maidata.txt" {
            let charts = parse_maidata(entry.path(), |diff| match diff.level() {
                Some(Level::Normal(level)) => (11..=13).contains(&level),
                Some(Level::Plus(level)) => (11..=13).contains(&level),
                Some(Level::Char(_)) => false,
                None => false,
            });
            for chart in charts.iter() {
                if chart.len() >= K {
                    for i in 0..chart.len() - K + 1 {
                        let group = chart[i..i + K].to_vec();
                        groups.push(minimal(&group));
                    }
                } else {
                    println!("Warning!");
                }
            }
        }
    }

    println!("{} groups", groups.len());

    let mut group_map = HashMap::with_capacity(groups.len());
    for group in groups.iter() {
        let count = group_map.entry(group).or_insert(0);
        *count += 1;
    }

    let mut result: Vec<_> = group_map.iter().map(|(&k, &v)| (v, k.clone())).collect();
    result.sort_by(|&(k1, _), &(k2, _)| k2.cmp(&k1));
    result.retain(|(_, group)| {
        group.iter().any(|bundle| {
            bundle
                .iter()
                .any(|note| matches!(note, NormalizedNote::Slide(_)))
        })
    });
    for (k, v) in result.iter().take(2000) {
        print!("{k}: ");
        for bundle in v.iter() {
            for (i, note) in bundle.iter().enumerate() {
                if i > 0 {
                    print!("/");
                }
                print!("{note}");
            }
            print!(",");
        }
        println!();
    }

    let duration = start.elapsed();
    println!("Time: {duration:?}");
}

// TODO: Note Bundle
fn parse_maidata<P: AsRef<std::path::Path>, F>(path: P, f: F) -> Vec<Vec<Vec<NormalizedNote>>>
where
    F: Fn(&AssociatedBeatmapData) -> bool,
{
    let content = maidata::app::read_file(path);
    let (maidata, state) = maidata::container::lex_maidata(&content);
    maidata::app::print_state_messages(&state);

    maidata
        .iter_difficulties()
        .filter_map(|diff| {
            if !f(&diff) {
                return None;
            }
            let notes: Vec<_> = diff
                .iter_insns()
                .filter_map(|insn| match insn.deref() {
                    RawInsn::Notes(insn) => {
                        let mut result: Vec<_> = insn
                            .deref()
                            .iter()
                            .map(|x| normalize_note(x.deref()).unwrap())
                            .collect();
                        result.sort();
                        Some(result)
                    }
                    _ => None,
                })
                .collect();
            match notes.is_empty() {
                true => None,
                false => Some(notes),
            }
        })
        .collect()
}
