use std::process::Command;
use tempfile::TempDir;

fn hold_guard_bin() -> std::path::PathBuf {
    env!("CARGO_BIN_EXE_hold-guard").into()
}

fn make_unit(dir: &std::path::Path, cat: &str, name: &str, size: usize) {
    let p = dir.join("debug").join(cat).join(name);
    std::fs::create_dir_all(&p).unwrap();
    std::fs::write(p.join("artifact"), vec![0u8; size]).unwrap();
}

#[test]
fn ac4_enforce_apply_reclaims_and_appends_ledger() {
    let tmp = TempDir::new().unwrap();
    let ledger = tmp.path().join("ledger.jsonl");

    // total ~3000 bytes; cap 1000; low-water 500
    make_unit(tmp.path(), "deps", "crate_a-111", 1000);
    make_unit(tmp.path(), "deps", "crate_b-222", 1000);
    make_unit(tmp.path(), "deps", "crate_c-333", 1000);

    let out = Command::new(hold_guard_bin())
        .args([
            "enforce",
            "--hold", tmp.path().to_str().unwrap(),
            "--max-size", "1000",
            "--low-water", "500",
            "--apply",
            "--ts", "2026-01-01T00:00:00Z",
            "--ledger", ledger.to_str().unwrap(),
        ])
        .output()
        .expect("failed");

    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("JSON");
    assert!(v["reclaimed_bytes"].as_u64().unwrap() > 0, "expected bytes reclaimed");
    assert_eq!(v["dry_run"], false);

    // Ledger should exist and have lines
    assert!(ledger.exists(), "ledger not written");
    let content = std::fs::read_to_string(&ledger).unwrap();
    assert!(!content.trim().is_empty(), "ledger is empty");
}
