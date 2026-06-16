use std::process::Command;
use tempfile::TempDir;

fn hold_guard_bin() -> std::path::PathBuf {
    env!("CARGO_BIN_EXE_hold-guard").into()
}

fn make_unit(dir: &std::path::Path, profile: &str, cat: &str, name: &str, size: usize) {
    let p = dir.join(profile).join(cat).join(name);
    std::fs::create_dir_all(&p).unwrap();
    std::fs::write(p.join("artifact"), vec![0u8; size]).unwrap();
}

#[test]
fn ac3_check_over_cap_lists_lru_units() {
    let tmp = TempDir::new().unwrap();
    // total ~3KB; cap 1KB; low-water 500B
    make_unit(tmp.path(), "debug", "deps", "crate_old-aaa", 1200);
    make_unit(tmp.path(), "debug", "deps", "crate_mid-bbb", 1200);
    make_unit(tmp.path(), "debug", "deps", "crate_new-ccc", 1200);

    let out = Command::new(hold_guard_bin())
        .args([
            "check",
            "--hold", tmp.path().to_str().unwrap(),
            "--max-size", "1000",
            "--low-water", "500",
            "--ts", "2026-01-01T00:00:00Z",
        ])
        .output()
        .expect("failed");

    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("expected JSON");
    assert_eq!(v["over_cap"], true);
    assert_eq!(v["dry_run"], true);
    // At least one unit should be listed
    let selected = v["units_selected"].as_array().unwrap();
    assert!(!selected.is_empty(), "expected at least one unit to evict");
}
