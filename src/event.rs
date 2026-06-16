//! Build and emit structured SLO events matching ballast-guard's event schema.
//!
//! Schema fields: `kind`, `severity`, `hold_bytes`, `cap_bytes`,
//! `reclaimed_bytes`, `ts`.

use serde::{Deserialize, Serialize};

/// Structured SLO event — matches ballast-guard's event schema.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SloEvent {
    /// Event kind identifier.
    pub(crate) kind: String,
    /// Severity: "ok" | "warn" | "critical".
    pub(crate) severity: String,
    /// Hold size in bytes at event time.
    pub(crate) hold_bytes: u64,
    /// Configured cap in bytes.
    pub(crate) cap_bytes: u64,
    /// Bytes reclaimed by this enforcement (0 in dry-run).
    pub(crate) reclaimed_bytes: u64,
    /// RFC 3339 timestamp.
    pub(crate) ts: String,
    /// Whether --apply was given (true) or dry-run (false).
    pub(crate) applied: bool,
}

/// Build a structured SLO event.
///
/// `ts` is used verbatim if non-empty; otherwise the current UTC time is used.
#[must_use]
pub(crate) fn build_event(
    hold_bytes: u64,
    cap_bytes: u64,
    reclaimed_bytes: u64,
    applied: bool,
    ts: &str,
) -> SloEvent {
    let ts = if ts.is_empty() {
        chrono::Utc::now().to_rfc3339()
    } else {
        ts.to_owned()
    };
    let severity = if hold_bytes > cap_bytes {
        "critical"
    } else if hold_bytes > (cap_bytes as f64 * 0.9) as u64 {
        "warn"
    } else {
        "ok"
    };
    SloEvent {
        kind: "hold_guard.enforcement".to_owned(),
        severity: severity.to_owned(),
        hold_bytes,
        cap_bytes,
        reclaimed_bytes,
        ts,
        applied,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_fields_present() {
        let ev = build_event(100, 200, 50, true, "2026-01-01T00:00:00Z");
        assert_eq!(ev.kind, "hold_guard.enforcement");
        assert_eq!(ev.hold_bytes, 100);
        assert_eq!(ev.cap_bytes, 200);
        assert_eq!(ev.reclaimed_bytes, 50);
        assert_eq!(ev.ts, "2026-01-01T00:00:00Z");
        assert!(ev.applied);
    }

    #[test]
    fn event_severity_critical_when_over_cap() {
        let ev = build_event(300, 200, 0, false, "2026-01-01T00:00:00Z");
        assert_eq!(ev.severity, "critical");
    }

    #[test]
    fn event_severity_ok_when_under_cap() {
        let ev = build_event(100, 200, 0, false, "2026-01-01T00:00:00Z");
        assert_eq!(ev.severity, "ok");
    }
}
