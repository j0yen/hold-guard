use std::process::Command;
use tempfile::TempDir;

fn hold_guard_bin() -> std::path::PathBuf {
    env!("CARGO_BIN_EXE_hold-guard").into()
}

fn make_unit_with_lock(dir: &std::path::Path, cat: &str, name: &str, size: usize) {
    let p = dir.join("debug").join(cat).join(name);
    std::fs::create_dir_all(&p).unwrap();
    std::fs::write(p.join("artifact"), vec![0u8; size]).unwrap();
    // Create the lock file that signals an active build holds this unit
    std::fs::write(p.join(".cargo-lock"), b"").unwrap();
}

fn make_unit(dir: &std::path::Path, cat: &str, name: &str, size: usize) {
    let p = dir.join("debug").join(cat).join(name);
    std::fs::create_dir_all(&p).unwrap();
    std::fs::write(p.join("artifact"), vec![0u8; size]).unwrap();
}

#[test]
fn ac6_held_unit_not_selected() {
    let tmp = TempDir::new().unwrap();
    // total 3000; cap 1000; all 3 units needed to evict
    // but "crate_locked" has a .cargo-lock — must not be selected
    make_unit_with_lock(tmp.path(), "deps", "crate_locked-aaa", 1000);
    make_unit(tmp.path(), "deps", "crate_free1-bbb", 1000);
    make_unit(tmp.path(), "deps", "crate_free2-ccc", 1000);

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
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("JSON");

    let selected = v["units_selected"].as_array().unwrap();
    for s in selected {
        assert!(
            !s.as_str().unwrap().contains("crate_locked"),
            "held unit should not be selected: {s}"
        );
    }
}
