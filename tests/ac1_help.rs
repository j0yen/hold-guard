use std::process::Command;

fn hold_guard_bin() -> std::path::PathBuf {
    env!("CARGO_BIN_EXE_hold-guard").into()
}

#[test]
fn ac1_help_exits_zero() {
    let out = Command::new(hold_guard_bin())
        .arg("--help")
        .output()
        .expect("failed to run hold-guard");
    assert!(out.status.success(), "expected exit 0 from --help");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("check"), "missing 'check' in help");
    assert!(stdout.contains("enforce"), "missing 'enforce' in help");
    assert!(stdout.contains("status"), "missing 'status' in help");
}
