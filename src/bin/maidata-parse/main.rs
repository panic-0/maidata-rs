use std::time::Instant;
use walkdir::WalkDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::args().nth(1).expect("usage: $0 <path/to/charts>");

    let start = Instant::now();

    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !e.file_type().is_dir())
    {
        if entry.file_name() == "maidata.txt" {
            // println!("{:?}", entry.path());
            parse_maidata(entry.path())?;
        }
    }

    let duration = start.elapsed();
    println!("Time: {:?}", duration);

    Ok(())
}

fn parse_maidata<P: AsRef<std::path::Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
    let content = maidata::app::read_file(&path);
    let (_, state) = maidata::container::lex_maidata(&content);

    if state.has_messages() {
        println!("Path: {:?}", path.as_ref());
        maidata::app::print_state_messages(&state);
        println!();
    }

    // for diff in maidata.iter_difficulties() {
    //     println!("{} insns", diff.iter_insns().count());

    //     // for insn in diff.iter_insns() {
    //     //     println!("{:?}", insn);
    //     // }
    // }

    Ok(())
}
