//! Enumerate evictable units in a cargo hold directory.
//!
//! Evictable units are per-fingerprint subdirectories under:
//! - `<hold>/debug/deps/`
//! - `<hold>/debug/.fingerprint/`
//! - `<hold>/debug/incremental/`
//! - `<hold>/release/deps/`
//! - `<hold>/release/.fingerprint/`
//! - `<hold>/release/incremental/`
//!
//! Each subdirectory is one unit, sized as the sum of all files within.
//! The unit's "last access" time is the most recent mtime across its contents.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

/// A single evictable unit (per-fingerprint subdirectory).
#[derive(Debug, Clone)]
pub struct Unit {
    /// Absolute path to the unit directory.
    pub path: PathBuf,
    /// Total bytes of all files within the unit.
    pub bytes: u64,
    /// Most recent mtime across all files (used for LRU ordering).
    pub last_used: SystemTime,
    /// True if a `.cargo-lock` file exists in this unit directory.
    pub is_locked: bool,
}

/// Enumerate all evictable units under a hold directory.
///
/// Returns units sorted oldest-first (lowest `last_used` first) so
/// the caller can slice the front for LRU eviction.
///
/// # Errors
///
/// Returns an error if the hold directory cannot be read.
pub fn enumerate_units(hold: &Path) -> Result<Vec<Unit>> {
    let mut units = Vec::new();

    // Subdirs within each profile that contain per-fingerprint units.
    let evictable_subdirs = ["deps", ".fingerprint", "incremental"];
    let profiles = ["debug", "release"];

    for profile in profiles {
        for subdir in evictable_subdirs {
            let parent = hold.join(profile).join(subdir);
            if !parent.is_dir() {
                continue;
            }
            // Each direct child of parent is one evictable unit.
            let rd = std::fs::read_dir(&parent)
                .with_context(|| format!("read_dir {}", parent.display()))?;
            for entry in rd {
                let entry = entry.with_context(|| format!("entry in {}", parent.display()))?;
                let path = entry.path();
                if path.is_dir() {
                    let (bytes, last_used) = size_and_mtime(&path)?;
                    let is_locked = path.join(".cargo-lock").exists();
                    units.push(Unit { path, bytes, last_used, is_locked });
                } else {
                    // Flat files directly under deps/ etc. are also artifacts.
                    // Treat each as a single-file unit.
                    let meta = entry.metadata()
                        .with_context(|| format!("metadata for {}", path.display()))?;
                    let bytes = meta.len();
                    let last_used = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                    let is_locked = false; // flat files can't hold .cargo-lock
                    units.push(Unit { path, bytes, last_used, is_locked });
                }
            }
        }
    }

    // Sort oldest-first so policy::select_lru can take from the front.
    units.sort_by_key(|u| u.last_used);

    Ok(units)
}

/// Recursively sum file sizes and find the most recent mtime within a directory.
fn size_and_mtime(dir: &Path) -> Result<(u64, SystemTime)> {
    let mut total = 0u64;
    let mut newest = SystemTime::UNIX_EPOCH;
    for entry in WalkDir::new(dir).follow_links(false) {
        let entry = entry.with_context(|| format!("walk {}", dir.display()))?;
        if entry.file_type().is_file() {
            let meta = entry.metadata()
                .with_context(|| format!("metadata for {}", entry.path().display()))?;
            total += meta.len();
            let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            if mtime > newest {
                newest = mtime;
            }
        }
    }
    Ok((total, newest))
}
