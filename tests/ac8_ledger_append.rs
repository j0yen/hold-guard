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
fn ac8_ledger_is_append_only() {
    let tmp = TempDir::new().unwrap();
    let ledger = tmp.path().join("ledger.jsonl");

    // Run enforce --apply twice; ledger line count must only grow
    for i in 0..2u32 {
        // Recreate units each time so there's something to evict
        make_unit(tmp.path(), "deps", &format!("crate_{i}a-111"), 2000);
        make_unit(tmp.path(), "deps", &format!("crate_{i}b-222"), 2000);

        Command::new(hold_guard_bin())
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
    }

    let content = std::fs::read_to_string(&ledger).unwrap_or_default();
    let line_count = content.lines().count();
    // Both runs should have appended; at least 2 lines (one per run that found something to evict)
    assert!(line_count >= 2, "expected at least 2 ledger lines, got {line_count}");
}
