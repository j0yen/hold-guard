//! AC8: The ledger is append-only across runs: line count only grows.

use std::process::Command;
use std::time::{SystemTime, Duration};
use tempfile::TempDir;

fn binary() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop();
    if p.ends_with("deps") {
        p.pop();
    }
    p.join("hold-guard")
}

fn make_unit_with_mtime(deps: &std::path::Path, name: &str, size: usize, mtime: SystemTime) {
    let unit_dir = deps.join(name);
    std::fs::create_dir_all(&unit_dir).unwrap();
    let artifact = unit_dir.join("artifact");
    std::fs::write(&artifact, vec![0u8; size]).unwrap();
    let ft = filetime::FileTime::from_system_time(mtime);
    filetime::set_file_mtime(&artifact, ft).unwrap();
}

fn count_ledger_lines(path: &std::path::Path) -> usize {
    if !path.exists() {
        return 0;
    }
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .count()
}

#[test]
fn ac8_ledger_append_only_line_count_grows() {
    let dir = TempDir::new().unwrap();
    let deps = dir.path().join("debug").join("deps");
    std::fs::create_dir_all(&deps).unwrap();
    let ledger_path = dir.path().join("guard-ledger.jsonl");

    let now = SystemTime::now();
    let unit_size = 2 * 1024 * 1024;

    // First run: create 2 units, evict oldest.
    make_unit_with_mtime(&deps, "unit_a", unit_size, now - Duration::from_secs(7200));
    make_unit_with_mtime(&deps, "unit_b", unit_size, now - Duration::from_secs(60));

    let run1 = Command::new(binary())
        .arg("--hold").arg(dir.path())
        .arg("--ts").arg("2026-01-01T00:00:00Z")
        .arg("enforce")
        .arg("--max-size").arg("3M")
        .arg("--low-water").arg("2M")
        .arg("--apply")
        .arg("--ledger").arg(&ledger_path)
        .output()
        .expect("run1 failed");
    assert!(run1.status.success());

    let lines_after_run1 = count_ledger_lines(&ledger_path);
    assert!(lines_after_run1 > 0, "ledger should have lines after first run");

    // Second run: add a new unit and evict it.
    make_unit_with_mtime(&deps, "unit_c", unit_size, now - Duration::from_secs(3600));

    let run2 = Command::new(binary())
        .arg("--hold").arg(dir.path())
        .arg("--ts").arg("2026-01-02T00:00:00Z")
        .arg("enforce")
        .arg("--max-size").arg("3M")
        .arg("--low-water").arg("2M")
        .arg("--apply")
        .arg("--ledger").arg(&ledger_path)
        .output()
        .expect("run2 failed");
    assert!(run2.status.success());

    let lines_after_run2 = count_ledger_lines(&ledger_path);
    assert!(
        lines_after_run2 > lines_after_run1,
        "ledger line count should grow after second run: before={lines_after_run1} after={lines_after_run2}"
    );
}
