use crate::event::Event;
use anyhow::Result;
use std::io::{BufRead, Write};
use std::path::Path;

/// Append an event to the ledger (append-only JSONL).
pub fn append(ledger_path: &Path, evt: &Event) -> Result<()> {
    if let Some(parent) = ledger_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(ledger_path)?;
    let line = serde_json::to_string(evt)?;
    writeln!(file, "{line}")?;
    Ok(())
}

/// Return the last `n` lines of the ledger.
pub fn tail(ledger_path: &Path, n: usize) -> Result<Vec<String>> {
    let file = std::fs::File::open(ledger_path)?;
    let reader = std::io::BufReader::new(file);
    let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
    let start = lines.len().saturating_sub(n);
    Ok(lines[start..].to_vec())
}
