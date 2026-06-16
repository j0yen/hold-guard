//! AC4: enforce --apply removes exactly the selected LRU units, reports
//! reclaimed_bytes > 0, brings hold below --low-water, appends ledger lines.

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
fn ac4_enforce_apply_removes_units_and_appends_ledger() {
    let dir = TempDir::new().unwrap();
    let deps = dir.path().join("debug").join("deps");
    std::fs::create_dir_all(&deps).unwrap();
    let ledger_path = dir.path().join("guard-ledger.jsonl");

    let now = SystemTime::now();
    let unit_size = 2 * 1024 * 1024; // 2MB each
    make_unit_with_mtime(&deps, "unit_old", unit_size, now - Duration::from_secs(3600));
    make_unit_with_mtime(&deps, "unit_new", unit_size, now - Duration::from_secs(10));

    // Total = 4M, cap = 3M, low-water = 2M → should evict unit_old (2MB).
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
        .arg("--apply")
        .arg("--ledger")
        .arg(&ledger_path)
        .output()
        .expect("failed to run hold-guard enforce");

    assert!(output.status.success(), "exit: {:?}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("bad JSON: {e}\noutput: {stdout}"));

    // reclaimed_bytes > 0
    let reclaimed = v["reclaimed_bytes"].as_u64().expect("reclaimed_bytes");
    assert!(reclaimed > 0, "expected reclaimed_bytes > 0, got {reclaimed}");

    // unit_old should be removed.
    assert!(!deps.join("unit_old").exists(), "unit_old should have been evicted");
    // unit_new should remain.
    assert!(deps.join("unit_new").exists(), "unit_new should remain");

    // Ledger should have lines.
    assert!(ledger_path.exists(), "ledger file should exist");
    let content = std::fs::read_to_string(&ledger_path).unwrap();
    let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
    assert!(!lines.is_empty(), "ledger should have at least one line");
}
