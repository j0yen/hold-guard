// Validate that emitted event has all required ballast-guard schema fields
use std::process::Command;
use tempfile::TempDir;

fn hold_guard_bin() -> std::path::PathBuf {
    env!("CARGO_BIN_EXE_hold-guard").into()
}

#[test]
fn ac5_event_has_required_fields() {
    let tmp = TempDir::new().unwrap();
    let out = Command::new(hold_guard_bin())
        .args([
            "check",
            "--hold", tmp.path().to_str().unwrap(),
            "--max-size", "1G",
            "--ts", "2026-01-01T00:00:00Z",
        ])
        .output()
        .expect("failed");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("JSON");

    // Required schema fields
    assert!(v.get("kind").is_some(), "missing kind");
    assert!(v.get("severity").is_some(), "missing severity");
    assert!(v.get("hold_bytes").is_some(), "missing hold_bytes");
    assert!(v.get("cap_bytes").is_some(), "missing cap_bytes");
    assert!(v.get("reclaimed_bytes").is_some(), "missing reclaimed_bytes");
    assert!(v.get("ts").is_some(), "missing ts");
    assert!(v.get("over_cap").is_some(), "missing over_cap");
    assert!(v.get("dry_run").is_some(), "missing dry_run");
    assert!(v.get("units_selected").is_some(), "missing units_selected");

    // ts matches what we passed
    assert_eq!(v["ts"], "2026-01-01T00:00:00Z");
}
