//! The queue of tests that the processes of `--jobs` share: a folder in which testkit claims each
//! test before running it, and the order to take them in, the slowest of the last run first, so
//! that no long test is left for the end.

use std::path::{Path, PathBuf};

/// Empties the queue folder and writes `order.txt` in it from the last run's report.
pub fn prepare(run_dir: &Path) -> PathBuf {
    let dir = run_dir.join("test-jobs").join("queue");
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    let report = std::fs::read_to_string(run_dir.join("test-reports").join("report.txt"))
        .unwrap_or_default();
    let _ = std::fs::write(dir.join("order.txt"), longest_first(&report).join("\n"));
    dir
}

/// The names of the tests in a report, the slowest first.
fn longest_first(report: &str) -> Vec<&str> {
    let mut times: Vec<(&str, f32)> = report.lines().filter_map(timed).collect();
    times.sort_by(|a, b| b.1.total_cmp(&a.1));
    times.into_iter().map(|(name, _)| name).collect()
}

/// The name and seconds of a report's `PASS name (1.2 s)` or `FAIL name (1.2 s)` line.
fn timed(line: &str) -> Option<(&str, f32)> {
    let rest = line
        .strip_prefix("PASS ")
        .or_else(|| line.strip_prefix("FAIL "))?;
    let (name, secs) = rest.split_once(" (")?;
    Some((name, secs.strip_suffix(" s)")?.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slowest_tests_come_first() {
        let report = "testkit report: 2 passed, 1 failed, 9.0 s, 3 at once\n\n\
            PASS quick (1.0 s)\n    a note (2.0 s)\nFAIL slow (5.5 s)\n    FAIL got 1, want 2\nPASS middle (2.5 s)\n";
        assert_eq!(
            longest_first(report),
            ["slow", "middle", "quick"],
            "ordered by time, notes left out"
        );
    }

    #[test]
    fn crashed_processes_are_left_out() {
        let report = "\nFAIL process 1 ended without a report, see run/test-jobs/1/output.log\nPASS quick (1.0 s)\n";
        assert_eq!(longest_first(report), ["quick"], "a crash names no test");
    }

    #[test]
    fn no_report_gives_no_order() {
        assert!(
            longest_first("").is_empty(),
            "a first run has nothing to go by"
        );
    }
}
