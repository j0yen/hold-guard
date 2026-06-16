//! Append-only ledger for eviction records.
//!
//! Each record is a JSON line (JSONL) appended to the ledger file.
//! The ledger is never truncated; it only grows.

use crate::measure::Unit;
use anyhow::{Context, Result};
use serde::Serialize;
use std::io::Write;
use std::path::Path;

/// One ledger line.
#[derive(Debug, Serialize)]
struct LedgerLine {
    ts: String,
    evicted_path: String,
    bytes_reclaimed: u64,
    total_reclaimed_bytes: u64,
}

/// Append eviction records to the ledger file.
///
/// Each evicted unit gets one line. The ledger is opened in append mode.
///
/// # Errors
///
/// Returns an error if the ledger cannot be opened or written.
pub fn append(ledger_path: &Path, evicted: &[Unit], total_reclaimed: u64, ts: &str) -> Result<()> {
    if evicted.is_empty() {
        return Ok(());
    }
    let ts = if ts.is_empty() {
        chrono::Utc::now().to_rfc3339()
    } else {
        ts.to_owned()
    };
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(ledger_path)
        .with_context(|| format!("open ledger {}", ledger_path.display()))?;
    for unit in evicted {
        let line = LedgerLine {
            ts: ts.clone(),
            evicted_path: unit.path.display().to_string(),
            bytes_reclaimed: unit.bytes,
            total_reclaimed_bytes: total_reclaimed,
        };
        let json = serde_json::to_string(&line)
            .context("serialize ledger line")?;
        writeln!(f, "{json}").context("write ledger line")?;
    }
    f.flush().context("flush ledger")?;
    Ok(())
}

/// Return the last `n` lines from the ledger file as a Vec of raw JSON strings.
///
/// Returns an empty vec if the ledger does not exist.
///
/// # Errors
///
/// Returns an error if the ledger cannot be read.
pub fn tail(ledger_path: &Path, n: usize) -> Result<Vec<String>> {
    if !ledger_path.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(ledger_path)
        .with_context(|| format!("read ledger {}", ledger_path.display()))?;
    let lines: Vec<String> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(ToOwned::to_owned)
        .collect();
    let start = lines.len().saturating_sub(n);
    Ok(lines[start..].to_vec())
}
