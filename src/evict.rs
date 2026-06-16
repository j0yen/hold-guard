use crate::measure::Unit;
use anyhow::Result;

/// Remove the selected units from the filesystem.
/// Returns total bytes reclaimed.
pub fn remove_units(units: &[Unit]) -> Result<u64> {
    let mut reclaimed = 0u64;
    for unit in units {
        if unit.path.exists() {
            reclaimed += unit.bytes;
            std::fs::remove_dir_all(&unit.path)?;
        }
    }
    Ok(reclaimed)
}
