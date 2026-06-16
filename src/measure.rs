use anyhow::Result;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct Unit {
    /// The fingerprint subdirectory (evictable unit)
    pub path: PathBuf,
    /// Total bytes in this unit
    pub bytes: u64,
    /// Last access/mtime (whichever is later)
    pub last_used: SystemTime,
    /// Whether the unit appears to be held by an active build
    pub held: bool,
}

/// Enumerate evictable units under a hold target directory.
/// Evictable units are subdirs of `deps/`, `.fingerprint/`, `incremental/`
/// under any profile directory (e.g. target/debug, target/release, target/x86_64-*/debug, etc.)
pub fn enumerate_units(hold: &Path) -> Result<Vec<Unit>> {
    if !hold.exists() {
        return Ok(vec![]);
    }

    let mut units = Vec::new();

    // Walk one level to find profile dirs (debug, release, etc.)
    // Profile dirs live directly under hold, or under target-triple dirs
    let evictable_parents = ["deps", ".fingerprint", "incremental"];

    for entry in WalkDir::new(hold).max_depth(4).min_depth(1) {
        let entry = entry?;
        if !entry.file_type().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if !evictable_parents.contains(&name.as_ref()) {
            continue;
        }
        // Each child of this dir is an evictable unit
        let parent_path = entry.path();
        if let Ok(rd) = std::fs::read_dir(parent_path) {
            for child in rd.flatten() {
                if child.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let unit_path = child.path();
                    let (bytes, last_used) = dir_stats(&unit_path);
                    let held = is_held(&unit_path);
                    units.push(Unit {
                        path: unit_path,
                        bytes,
                        last_used,
                        held,
                    });
                }
            }
        }
    }

    Ok(units)
}

/// Return total bytes under a directory (non-recursive units already summed at unit level).
pub fn total_size(hold: &Path) -> Result<u64> {
    if !hold.exists() {
        return Ok(0);
    }
    let mut total = 0u64;
    for entry in WalkDir::new(hold) {
        let entry = entry?;
        if entry.file_type().is_file() {
            total += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    Ok(total)
}

fn dir_stats(path: &Path) -> (u64, SystemTime) {
    let mut bytes = 0u64;
    let mut latest = SystemTime::UNIX_EPOCH;
    for entry in WalkDir::new(path) {
        let Ok(e) = entry else { continue };
        if e.file_type().is_file() {
            if let Ok(meta) = e.metadata() {
                bytes += meta.len();
                let t = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                if t > latest {
                    latest = t;
                }
            }
        }
    }
    (bytes, latest)
}

/// Check if this unit appears held by an active build.
/// Heuristic: presence of a `.cargo-lock` file inside the unit dir.
fn is_held(path: &Path) -> bool {
    path.join(".cargo-lock").exists()
}
