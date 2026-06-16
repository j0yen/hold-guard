use std::process::Command;
use tempfile::TempDir;

fn hold_guard_bin() -> std::path::PathBuf {
    env!("CARGO_BIN_EXE_hold-guard").into()
}

fn make_fixture(dir: &std::path::Path, profile: &str, cat: &str, name: &str, size: usize) {
    let p = dir.join(profile).join(cat).join(name);
    std::fs::create_dir_all(&p).unwrap();
    std::fs::write(p.join("artifact"), vec![0u8; size]).unwrap();
}

#[test]
fn ac2_check_under_cap_reports_false() {
    let tmp = TempDir::new().unwrap();
    // create a tiny fixture: 1KB
    make_fixture(tmp.path(), "debug", "deps", "crate_abc-deadbeef", 512);
    make_fixture(tmp.path(), "debug", "deps", "crate_xyz-cafebabe", 512);

    let out = Command::new(hold_guard_bin())
        .args([
            "check",
            "--hold", tmp.path().to_str().unwrap(),
            "--max-size", "1000000",  // 1MB cap, hold is ~1KB
            "--ts", "2026-01-01T00:00:00Z",
        ])
        .output()
        .expect("failed to run hold-guard");

    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("expected JSON output");
    assert_eq!(v["over_cap"], false);
    assert_eq!(v["reclaimed_bytes"], 0);
    assert_eq!(v["dry_run"], true);
    // units_selected must be empty
    assert_eq!(v["units_selected"].as_array().unwrap().len(), 0);
}
