//! Evict (remove) selected units from the hold.

use crate::measure::Unit;
use anyhow::{Context, Result};

/// Remove all units in `selected` from disk.
///
/// Each unit is either a directory (removed recursively) or a single file.
/// Errors accumulate and are returned at the end if any removal fails.
///
/// # Errors
///
/// Returns an error if any unit cannot be removed.
pub(crate) fn remove_units(selected: &[Unit]) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();
    for unit in selected {
        let result = if unit.path.is_dir() {
            std::fs::remove_dir_all(&unit.path)
                .with_context(|| format!("remove_dir_all {}", unit.path.display()))
        } else {
            std::fs::remove_file(&unit.path)
                .with_context(|| format!("remove_file {}", unit.path.display()))
        };
        if let Err(e) = result {
            errors.push(format!("{e:#}"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("eviction errors:\n{}", errors.join("\n")))
    }
}
