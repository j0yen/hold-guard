//! hold-guard — cap and LRU-evict a shared cargo hold.
//!
//! Subcommands:
//! - `check`   — dry-run: measure hold, report over/under cap, list would-evict units.
//! - `enforce` — evict LRU units until hold is below `--low-water`; dry-run unless `--apply`.
//! - `status`  — hold size vs cap, last enforcement, ledger tail.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod event;
mod evict;
mod ledger;
mod measure;
mod policy;

#[derive(Debug, Parser)]
#[command(
    name = "hold-guard",
    about = "Cap and LRU-evict a shared cargo hold so it can't regrow to 214G",
    version
)]
struct Cli {
    /// Path to the cargo target hold directory (default: ~/wintermute/.hold/target)
    #[arg(long, global = true)]
    hold: Option<PathBuf>,

    /// RFC 3339 timestamp to use for events/ledger (default: current time).
    /// Useful for deterministic tests.
    #[arg(long, global = true)]
    ts: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Dry-run: measure hold, report over/under cap, list units that would be evicted.
    Check {
        /// Maximum allowed hold size (e.g. 60G, 10G, 500M).
        #[arg(long)]
        max_size: String,

        /// Low-water target (default: 80% of max-size).
        #[arg(long)]
        low_water: Option<String>,

        /// Path to the ledger file (default: <hold>/../guard-ledger.jsonl).
        #[arg(long)]
        ledger: Option<PathBuf>,
    },
    /// Enforce the cap: evict LRU units until hold is below --low-water.
    Enforce {
        /// Maximum allowed hold size (e.g. 60G, 10G, 500M).
        #[arg(long)]
        max_size: String,

        /// Low-water target (default: 80% of max-size).
        #[arg(long)]
        low_water: Option<String>,

        /// Actually remove files. Without this flag, enforcement is a dry-run.
        #[arg(long)]
        apply: bool,

        /// Path to the ledger file (default: <hold>/../guard-ledger.jsonl).
        #[arg(long)]
        ledger: Option<PathBuf>,
    },
    /// Show hold size vs cap, last enforcement, and ledger tail.
    Status {
        /// Maximum allowed hold size for comparison (optional).
        #[arg(long)]
        max_size: Option<String>,

        /// Path to the ledger file (default: <hold>/../guard-ledger.jsonl).
        #[arg(long)]
        ledger: Option<PathBuf>,
    },
}

fn default_hold() -> Result<PathBuf> {
    let home = std::env::var("HOME").map_err(|_| anyhow::anyhow!("HOME not set"))?;
    Ok(PathBuf::from(home).join("wintermute").join(".hold").join("target"))
}

fn resolve_hold(cli_hold: Option<PathBuf>) -> Result<PathBuf> {
    cli_hold.map_or_else(default_hold, Ok)
}

fn default_ledger(hold: &PathBuf) -> PathBuf {
    hold.parent()
        .unwrap_or(hold)
        .join("guard-ledger.jsonl")
}

fn resolve_ledger(cli_ledger: Option<PathBuf>, hold: &PathBuf) -> PathBuf {
    cli_ledger.unwrap_or_else(|| default_ledger(hold))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Check { max_size, low_water, ledger } => {
            let hold = resolve_hold(cli.hold)?;
            let ledger = resolve_ledger(ledger, &hold);
            let cap = policy::parse_size(&max_size)?;
            let lw = policy::resolve_low_water(low_water.as_deref(), cap)?;
            let units = measure::enumerate_units(&hold)?;
            let total_bytes: u64 = units.iter().map(|u| u.bytes).sum();
            let over_cap = total_bytes > cap;
            let selected = if over_cap {
                policy::select_lru(&units, total_bytes, lw)
            } else {
                vec![]
            };
            let projected = total_bytes.saturating_sub(selected.iter().map(|u| u.bytes).sum::<u64>());
            let result = serde_json::json!({
                "over_cap": over_cap,
                "hold_bytes": total_bytes,
                "cap_bytes": cap,
                "low_water_bytes": lw,
                "projected_bytes": projected,
                "selected_units": selected.iter().map(|u| u.path.display().to_string()).collect::<Vec<_>>(),
                "ledger": ledger.display().to_string(),
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Enforce { max_size, low_water, apply, ledger } => {
            let hold = resolve_hold(cli.hold)?;
            let ledger_path = resolve_ledger(ledger, &hold);
            let cap = policy::parse_size(&max_size)?;
            let lw = policy::resolve_low_water(low_water.as_deref(), cap)?;
            let ts = cli.ts.as_deref().unwrap_or("");
            let units = measure::enumerate_units(&hold)?;
            let total_bytes: u64 = units.iter().map(|u| u.bytes).sum();
            let selected = policy::select_lru(&units, total_bytes, lw);
            let reclaimed_bytes = selected.iter().map(|u| u.bytes).sum::<u64>();
            if apply {
                evict::remove_units(&selected)?;
                ledger::append(&ledger_path, &selected, reclaimed_bytes, ts)?;
            }
            let ev = event::build_event(total_bytes, cap, reclaimed_bytes, apply, ts);
            println!("{}", serde_json::to_string_pretty(&ev)?);
        }
        Command::Status { max_size, ledger } => {
            let hold = resolve_hold(cli.hold)?;
            let ledger_path = resolve_ledger(ledger, &hold);
            let units = measure::enumerate_units(&hold)?;
            let total_bytes: u64 = units.iter().map(|u| u.bytes).sum();
            let cap_bytes = max_size.as_deref().map(policy::parse_size).transpose()?;
            let last_line = ledger::tail(&ledger_path, 5)?;
            let result = serde_json::json!({
                "hold_bytes": total_bytes,
                "hold_human": humansize::format_size(total_bytes, humansize::BINARY),
                "cap_bytes": cap_bytes,
                "unit_count": units.len(),
                "ledger": ledger_path.display().to_string(),
                "ledger_tail": last_line,
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }

    Ok(())
}
