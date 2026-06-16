//! AC2: check --max-size <N> against a fixture hold under the cap reports
//! over_cap: false, selects nothing, reclaims nothing (read-only).

use std::process::Command;
use tempfile::TempDir;

fn binary() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop();
    if p.ends_with("deps") {
        p.pop();
    }
    p.join("hold-guard")
}

fn make_fixture_hold(dir: &TempDir, file_sizes: &[(&str, usize)]) {
    // Create a minimal hold structure: debug/deps/<unit>/file
    let deps = dir.path().join("debug").join("deps");
    std::fs::create_dir_all(&deps).unwrap();
    for (name, size) in file_sizes {
        let unit_dir = deps.join(name);
        std::fs::create_dir_all(&unit_dir).unwrap();
        std::fs::write(unit_dir.join("artifact"), vec![0u8; *size]).unwrap();
    }
}

#[test]
fn ac2_under_cap_reports_false_no_eviction() {
    let dir = TempDir::new().unwrap();
    // Create 1MB of fixtures; use a 100M cap — well under.
    make_fixture_hold(&dir, &[("unit_a", 512 * 1024), ("unit_b", 512 * 1024)]);

    let output = Command::new(binary())
        .arg("--hold")
        .arg(dir.path())
        .arg("check")
        .arg("--max-size")
        .arg("100M")
        .output()
        .expect("failed to run hold-guard check");

    assert!(output.status.success(), "exit: {:?}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("bad JSON: {e}\noutput: {stdout}"));

    assert_eq!(v["over_cap"], serde_json::Value::Bool(false), "expected over_cap=false, got {v}");
    let selected = v["selected_units"].as_array().expect("selected_units should be array");
    assert!(selected.is_empty(), "expected no selected units, got {selected:?}");

    // Verify no files were removed.
    let unit_a = dir.path().join("debug").join("deps").join("unit_a").join("artifact");
    assert!(unit_a.exists(), "unit_a was removed unexpectedly");
}
