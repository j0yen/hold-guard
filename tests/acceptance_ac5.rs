//! AC5: The emitted event validates against ballast-guard's event schema.
//! Required fields: kind, severity, hold_bytes, cap_bytes, reclaimed_bytes, ts.

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
fn ac5_event_schema_fields_present_and_typed() {
    let dir = TempDir::new().unwrap();
    let deps = dir.path().join("debug").join("deps");
    std::fs::create_dir_all(&deps).unwrap();
    let ledger_path = dir.path().join("guard-ledger.jsonl");

    let now = SystemTime::now();
    let unit_size = 2 * 1024 * 1024;
    make_unit_with_mtime(&deps, "unit_old", unit_size, now - Duration::from_secs(3600));
    make_unit_with_mtime(&deps, "unit_new", unit_size, now - Duration::from_secs(10));

    let output = Command::new(binary())
        .arg("--hold")
        .arg(dir.path())
        .arg("--ts")
        .arg("2026-06-16T00:00:00Z")
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

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("bad JSON: {e}\noutput: {stdout}"));

    // Validate schema fields.
    assert!(v["kind"].is_string(), "kind must be string");
    assert!(v["severity"].is_string(), "severity must be string");
    assert!(v["hold_bytes"].is_u64(), "hold_bytes must be integer");
    assert!(v["cap_bytes"].is_u64(), "cap_bytes must be integer");
    assert!(v["reclaimed_bytes"].is_u64(), "reclaimed_bytes must be integer");
    assert!(v["ts"].is_string(), "ts must be string");

    // Validate values.
    assert!(!v["kind"].as_str().unwrap().is_empty(), "kind must not be empty");
    let severity = v["severity"].as_str().unwrap();
    assert!(
        ["ok", "warn", "critical"].contains(&severity),
        "severity must be ok/warn/critical, got {severity}"
    );
}
