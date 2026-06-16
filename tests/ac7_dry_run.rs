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
fn ac7_enforce_without_apply_is_dry_run() {
    let tmp = TempDir::new().unwrap();
    let ledger = tmp.path().join("ledger.jsonl");

    make_unit(tmp.path(), "deps", "crate_a-aaa", 2000);
    make_unit(tmp.path(), "deps", "crate_b-bbb", 2000);

    let out = Command::new(hold_guard_bin())
        .args([
            "enforce",
            "--hold", tmp.path().to_str().unwrap(),
            "--max-size", "1000",
            "--low-water", "500",
            // No --apply
            "--ts", "2026-01-01T00:00:00Z",
            "--ledger", ledger.to_str().unwrap(),
        ])
        .output()
        .expect("failed");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("JSON");
    assert_eq!(v["dry_run"], true);
    assert_eq!(v["reclaimed_bytes"], 0, "dry-run must not reclaim anything");

    // Ledger must NOT exist (or be empty) — no apply
    if ledger.exists() {
        let content = std::fs::read_to_string(&ledger).unwrap();
        assert!(content.trim().is_empty(), "ledger should be empty for dry-run");
    }

    // Files must still exist
    let unit_path = tmp.path().join("debug").join("deps").join("crate_a-aaa");
    assert!(unit_path.exists(), "unit was deleted in dry-run!");
}
