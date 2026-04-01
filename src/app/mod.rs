use std::path::Path;

use crate::diag::State;

/// Read a file as a UTF-8 string.
pub fn read_file<P: AsRef<Path>>(path: P) -> String {
    let content = std::fs::read(path.as_ref())
        .unwrap_or_else(|_| panic!("reading file {:?} failed", path.as_ref()));
    String::from_utf8(content).expect("decoding file content as utf-8 failed")
}

/// Print warnings and errors from parser state to stderr.
pub fn print_state_messages(state: &State) {
    for warning in &state.warnings {
        eprintln!("Warning: {warning}");
    }
    for error in &state.errors {
        eprintln!("Error: {error}");
    }
}
