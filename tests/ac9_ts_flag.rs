use std::process::Command;
use tempfile::TempDir;

fn hold_guard_bin() -> std::path::PathBuf {
    env!("CARGO_BIN_EXE_hold-guard").into()
}

#[test]
fn ac9_ts_flag_makes_output_deterministic() {
    let tmp = TempDir::new().unwrap();
    let ts = "2026-06-15T12:34:56Z";

    let run = |extra: &[&str]| -> serde_json::Value {
        let mut args = vec![
            "check",
            "--hold", tmp.path().to_str().unwrap(),
            "--max-size", "1G",
            "--ts", ts,
        ];
        args.extend_from_slice(extra);
        let out = Command::new(hold_guard_bin())
            .args(&args)
            .output()
            .expect("failed");
        serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).expect("JSON")
    };

    let v1 = run(&[]);
    let v2 = run(&[]);
    assert_eq!(v1["ts"], ts);
    assert_eq!(v2["ts"], ts);
    assert_eq!(v1["ts"], v2["ts"]);
}
