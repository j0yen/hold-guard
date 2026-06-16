//! AC6: A fingerprint unit with a simulated held lock (.cargo-lock) is never
//! selected for eviction even when it is the LRU candidate.

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
fn ac6_locked_unit_is_not_evicted() {
    let dir = TempDir::new().unwrap();
    let deps = dir.path().join("debug").join("deps");
    std::fs::create_dir_all(&deps).unwrap();
    let ledger_path = dir.path().join("guard-ledger.jsonl");

    let now = SystemTime::now();
    let unit_size = 3 * 1024 * 1024; // 3MB each; 6MB total

    // unit_locked is the oldest but has a .cargo-lock file.
    make_unit_with_mtime(&deps, "unit_locked", unit_size, now - Duration::from_secs(7200));
    std::fs::write(deps.join("unit_locked").join(".cargo-lock"), b"locked").unwrap();

    // unit_newer is less old but not locked.
    make_unit_with_mtime(&deps, "unit_newer", unit_size, now - Duration::from_secs(60));

    // Cap = 4M, low-water = 3M; must evict something but not unit_locked.
    let output = Command::new(binary())
        .arg("--hold")
        .arg(dir.path())
        .arg("--ts")
        .arg("2026-01-01T00:00:00Z")
        .arg("enforce")
        .arg("--max-size")
        .arg("4M")
        .arg("--low-water")
        .arg("3M")
        .arg("--apply")
        .arg("--ledger")
        .arg(&ledger_path)
        .output()
        .expect("failed to run hold-guard enforce");

    assert!(output.status.success(), "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr));

    // unit_locked must survive even though it is the LRU candidate.
    assert!(
        deps.join("unit_locked").exists(),
        "unit_locked was evicted despite holding a .cargo-lock"
    );
}
