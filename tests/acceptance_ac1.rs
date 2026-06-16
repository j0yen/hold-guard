//! AC1: hold-guard --help lists check, enforce, status subcommands and exits 0.

use std::process::Command;

fn binary() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop();
    if p.ends_with("deps") {
        p.pop();
    }
    p.join("hold-guard")
}

#[test]
fn ac1_help_lists_subcommands_exits_0() {
    let output = Command::new(binary())
        .arg("--help")
        .output()
        .expect("failed to run hold-guard --help");

    assert!(output.status.success(), "exit code was not 0: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    assert!(combined.contains("check"), "help output missing 'check': {combined}");
    assert!(combined.contains("enforce"), "help output missing 'enforce': {combined}");
    assert!(combined.contains("status"), "help output missing 'status': {combined}");
}
