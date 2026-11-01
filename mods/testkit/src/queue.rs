//! A queue of tests shared with the other processes of a run (see the game's `--jobs`): a folder
//! in which each process claims a test before running it, so every test runs once, in whichever
//! process is free first.

use std::path::Path;

/// Whether this process runs `name`: only the first to ask does. A folder that can't be written
/// runs every test in every process rather than none.
pub fn claim(dir: &Path, name: &str) -> bool {
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join(name));
    !matches!(file, Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists)
}
