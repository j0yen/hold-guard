//! Eviction policy: parse size strings, compute low-water, select LRU units.

use crate::measure::Unit;
use anyhow::{bail, Result};

/// Parse a human-readable size string into bytes.
///
/// Supported suffixes: `B`, `K`/`KB`/`KiB`, `M`/`MB`/`MiB`, `G`/`GB`/`GiB`, `T`/`TB`/`TiB`.
/// No suffix means bytes.
///
/// # Errors
///
/// Returns an error if the string is unparseable.
pub fn parse_size(s: &str) -> Result<u64> {
    let s = s.trim();
    // Try bare integer first.
    if let Ok(n) = s.parse::<u64>() {
        return Ok(n);
    }
    // Find where the numeric part ends.
    let split_pos = s.find(|c: char| c.is_alphabetic()).unwrap_or(s.len());
    let (num_str, suffix) = s.split_at(split_pos);
    let num: u64 = num_str.trim().parse().map_err(|_| anyhow::anyhow!("invalid size: {s}"))?;
    let multiplier: u64 = match suffix.trim().to_uppercase().as_str() {
        "" | "B" => 1,
        "K" | "KB" | "KIB" => 1024,
        "M" | "MB" | "MIB" => 1024 * 1024,
        "G" | "GB" | "GIB" => 1024 * 1024 * 1024,
        "T" | "TB" | "TIB" => 1024_u64 * 1024 * 1024 * 1024,
        other => bail!("unknown size suffix: {other}"),
    };
    Ok(num.saturating_mul(multiplier))
}

/// Resolve the low-water threshold.
///
/// If `low_water` is `None`, default to 80% of `cap`.
///
/// # Errors
///
/// Returns an error if the low_water string is unparseable.
pub fn resolve_low_water(low_water: Option<&str>, cap: u64) -> Result<u64> {
    match low_water {
        Some(s) => parse_size(s),
        None => Ok((cap as f64 * 0.8) as u64),
    }
}

/// Select the oldest (LRU) non-locked units to evict until the projected hold
/// size drops below `low_water`.
///
/// `units` must be sorted oldest-first (as returned by `measure::enumerate_units`).
/// Locked units (`.cargo-lock` present) are always skipped.
///
/// Returns only the units that would be evicted, in eviction order.
#[must_use]
pub fn select_lru(units: &[Unit], total_bytes: u64, low_water: u64) -> Vec<Unit> {
    if total_bytes <= low_water {
        return vec![];
    }
    let mut remaining = total_bytes;
    let mut selected = Vec::new();
    for unit in units {
        if remaining <= low_water {
            break;
        }
        if unit.is_locked {
            continue;
        }
        remaining = remaining.saturating_sub(unit.bytes);
        selected.push(unit.clone());
    }
    selected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_size_bare_bytes() {
        assert_eq!(parse_size("1024").unwrap(), 1024);
    }

    #[test]
    fn parse_size_mb() {
        assert_eq!(parse_size("10M").unwrap(), 10 * 1024 * 1024);
    }

    #[test]
    fn parse_size_gb() {
        assert_eq!(parse_size("60G").unwrap(), 60 * 1024 * 1024 * 1024);
    }

    #[test]
    fn parse_size_mib() {
        assert_eq!(parse_size("10MiB").unwrap(), 10 * 1024 * 1024);
    }

    #[test]
    fn parse_size_invalid() {
        assert!(parse_size("foobar").is_err());
    }

    #[test]
    fn resolve_low_water_default() {
        let cap = 100;
        assert_eq!(resolve_low_water(None, cap).unwrap(), 80);
    }

    #[test]
    fn resolve_low_water_explicit() {
        assert_eq!(resolve_low_water(Some("50M"), 100 * 1024 * 1024).unwrap(), 50 * 1024 * 1024);
    }
}
