//! A queue of tests shared with the other processes of a run (see the game's `--jobs`): a folder
//! in which each process claims a test before running it, so every test runs once, in whichever
//! process is free first.

use std::path::Path;

use crate::runner::TestDef;

/// The tests in the order of the folder's `order.txt`, the slowest of the last run first; tests it
/// doesn't name (new ones) go before all of them.
pub fn in_order(mut tests: Vec<TestDef>, dir: &Path) -> Vec<TestDef> {
    let order = std::fs::read_to_string(dir.join("order.txt")).unwrap_or_default();
    let rank = |name: &str| order.lines().position(|n| n == name).map_or(0, |i| i + 1);
    tests.sort_by_cached_key(|t| rank(&t.name));
    tests
}

/// Whether this process runs `name`: only the first to ask does. A folder that can't be written
/// runs every test in every process rather than none.
pub fn claim(dir: &Path, name: &str) -> bool {
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join(name));
    !matches!(file, Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists)
}
