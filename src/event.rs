use serde::{Deserialize, Serialize};

/// Structured SLO event — same schema as ballast-guard uses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub kind: String,
    pub severity: String,
    pub hold_bytes: u64,
    pub cap_bytes: u64,
    pub reclaimed_bytes: u64,
    pub ts: String,
    pub over_cap: bool,
    pub dry_run: bool,
    pub units_selected: Vec<String>,
}
