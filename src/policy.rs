use crate::measure::Unit;

/// Select LRU units to evict until projected size drops below low_water.
/// Never selects held units.
/// Returns units sorted oldest-access first (LRU order).
pub fn select_evict_units(
    units: &[Unit],
    total: u64,
    _cap: u64,
    low_water: u64,
) -> Vec<Unit> {
    if total <= low_water {
        return vec![];
    }

    // Sort by last_used ascending (oldest first = LRU)
    let mut candidates: Vec<&Unit> = units.iter().filter(|u| !u.held).collect();
    candidates.sort_by_key(|u| u.last_used);

    let mut selected = Vec::new();
    let mut remaining = total;

    for unit in candidates {
        if remaining <= low_water {
            break;
        }
        selected.push(unit.clone());
        remaining = remaining.saturating_sub(unit.bytes);
    }

    selected
}
