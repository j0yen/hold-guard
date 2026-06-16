//! AC3: check --max-size <N> against a fixture hold over the cap reports
//! over_cap: true and lists LRU units oldest-access first until projected
//! size is below --low-water.

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
    // Set mtime on the file.
    let ft = filetime::FileTime::from_system_time(mtime);
    filetime::set_file_mtime(&artifact, ft).unwrap();
}

#[test]
fn ac3_over_cap_reports_true_and_lists_lru_units() {
    let dir = TempDir::new().unwrap();
    let deps = dir.path().join("debug").join("deps");
    std::fs::create_dir_all(&deps).unwrap();

    let now = SystemTime::now();
    let old = now - Duration::from_secs(3600); // 1hr old
    let newer = now - Duration::from_secs(60);  // 1min old
    let newest = now - Duration::from_secs(10); // 10s old

    // Each unit is 2MB; 3 units = 6MB total.
    // Cap = 4M, low-water = 3M. Should select oldest until under 3M.
    let unit_size = 2 * 1024 * 1024;
    make_unit_with_mtime(&deps, "unit_old", unit_size, old);
    make_unit_with_mtime(&deps, "unit_newer", unit_size, newer);
    make_unit_with_mtime(&deps, "unit_newest", unit_size, newest);

    let output = Command::new(binary())
        .arg("--hold")
        .arg(dir.path())
        .arg("check")
        .arg("--max-size")
        .arg("4M")
        .arg("--low-water")
        .arg("3M")
        .output()
        .expect("failed to run hold-guard check");

    assert!(output.status.success(), "exit: {:?}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("bad JSON: {e}\noutput: {stdout}"));

    assert_eq!(v["over_cap"], serde_json::Value::Bool(true), "expected over_cap=true");
    let selected = v["selected_units"].as_array().expect("selected_units must be array");
    assert!(!selected.is_empty(), "expected some units selected");

    // The oldest unit should appear first.
    let first = selected[0].as_str().expect("unit should be string");
    assert!(first.contains("unit_old"), "expected unit_old first, got: {first}");

    // Projected size should be below low-water (3M = 3145728).
    let projected = v["projected_bytes"].as_u64().expect("projected_bytes must be u64");
    assert!(projected <= 3 * 1024 * 1024, "projected {projected} should be <= 3M");

    // No files should have been removed (check is read-only).
    assert!(deps.join("unit_old").join("artifact").exists(), "unit_old was removed unexpectedly");
}
