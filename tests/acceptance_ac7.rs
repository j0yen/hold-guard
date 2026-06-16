//! AC7: enforce is dry-run unless --apply is given: without --apply it
//! reclaims nothing and writes no ledger line.

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

#[test]
fn ac7_without_apply_dry_run_no_removal_no_ledger() {
    let dir = TempDir::new().unwrap();
    let deps = dir.path().join("debug").join("deps");
    std::fs::create_dir_all(&deps).unwrap();
    let ledger_path = dir.path().join("guard-ledger.jsonl");

    let now = SystemTime::now();
    let unit_size = 2 * 1024 * 1024;
    make_unit_with_mtime(&deps, "unit_old", unit_size, now - Duration::from_secs(3600));
    make_unit_with_mtime(&deps, "unit_new", unit_size, now - Duration::from_secs(10));

    // Enforce WITHOUT --apply.
    let output = Command::new(binary())
        .arg("--hold")
        .arg(dir.path())
        .arg("--ts")
        .arg("2026-01-01T00:00:00Z")
        .arg("enforce")
        .arg("--max-size")
        .arg("3M")
        .arg("--low-water")
        .arg("2M")
        .arg("--ledger")
        .arg(&ledger_path)
        // Note: no --apply flag
        .output()
        .expect("failed to run hold-guard enforce");

    assert!(output.status.success(), "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr));

    // Files must not be removed.
    assert!(deps.join("unit_old").exists(), "unit_old removed in dry-run");
    assert!(deps.join("unit_new").exists(), "unit_new removed in dry-run");

    // Ledger must not exist or be empty.
    if ledger_path.exists() {
        let content = std::fs::read_to_string(&ledger_path).unwrap();
        let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
        assert!(lines.is_empty(), "ledger should have no lines in dry-run, got {lines:?}");
    }
}
