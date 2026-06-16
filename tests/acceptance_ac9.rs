//! AC9: --ts <rfc3339> makes event and ledger timestamps deterministic.

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
fn ac9_ts_flag_makes_timestamps_deterministic() {
    let dir = TempDir::new().unwrap();
    let deps = dir.path().join("debug").join("deps");
    std::fs::create_dir_all(&deps).unwrap();
    let ledger_path = dir.path().join("guard-ledger.jsonl");

    let now = SystemTime::now();
    let unit_size = 2 * 1024 * 1024;
    make_unit_with_mtime(&deps, "unit_old", unit_size, now - Duration::from_secs(3600));
    make_unit_with_mtime(&deps, "unit_new", unit_size, now - Duration::from_secs(10));

    let fixed_ts = "2026-01-01T00:00:00Z";

    let output = Command::new(binary())
        .arg("--hold").arg(dir.path())
        .arg("--ts").arg(fixed_ts)
        .arg("enforce")
        .arg("--max-size").arg("3M")
        .arg("--low-water").arg("2M")
        .arg("--apply")
        .arg("--ledger").arg(&ledger_path)
        .output()
        .expect("failed to run hold-guard enforce");

    assert!(output.status.success(), "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr));

    // Check event ts.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("bad JSON: {e}\noutput: {stdout}"));
    assert_eq!(v["ts"].as_str().unwrap(), fixed_ts, "event ts should match --ts flag");

    // Check ledger ts.
    assert!(ledger_path.exists(), "ledger should exist");
    let content = std::fs::read_to_string(&ledger_path).unwrap();
    let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
    assert!(!lines.is_empty(), "ledger should have lines");
    for line in &lines {
        let lv: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("bad ledger JSON: {e}\nline: {line}"));
        assert_eq!(lv["ts"].as_str().unwrap(), fixed_ts,
            "ledger ts should match --ts flag");
    }
}
