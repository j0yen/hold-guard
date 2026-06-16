use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod evict;
mod event;
mod ledger;
mod measure;
mod policy;

#[derive(Parser)]
#[command(name = "hold-guard", about = "Cap the shared Cargo hold with LRU eviction")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Dry-run: report whether the hold is over cap and what would be evicted
    Check {
        /// Path to hold target dir (default: ~/wintermute/.hold/target)
        #[arg(long)]
        hold: Option<PathBuf>,
        /// Max size cap (e.g. 60G, 60GB, 60000000000)
        #[arg(long)]
        max_size: String,
        /// Low-water eviction target (default: 75% of max-size)
        #[arg(long)]
        low_water: Option<String>,
        /// Override timestamp (RFC3339) for deterministic output
        #[arg(long)]
        ts: Option<String>,
    },
    /// Enforce the cap; with --apply actually removes LRU units
    Enforce {
        /// Path to hold target dir
        #[arg(long)]
        hold: Option<PathBuf>,
        /// Max size cap
        #[arg(long)]
        max_size: String,
        /// Low-water eviction target (default: 75% of max-size)
        #[arg(long)]
        low_water: Option<String>,
        /// Actually remove files (default is dry-run)
        #[arg(long)]
        apply: bool,
        /// Override timestamp (RFC3339)
        #[arg(long)]
        ts: Option<String>,
        /// Ledger path (default: ~/wintermute/.hold/guard-ledger.jsonl)
        #[arg(long)]
        ledger: Option<PathBuf>,
    },
    /// Show hold size vs cap, ledger tail
    Status {
        /// Path to hold target dir
        #[arg(long)]
        hold: Option<PathBuf>,
        /// Max size cap
        #[arg(long)]
        max_size: Option<String>,
        /// Ledger path
        #[arg(long)]
        ledger: Option<PathBuf>,
    },
}

fn default_hold() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(home).join("wintermute/.hold/target")
}

fn default_ledger() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(home).join("wintermute/.hold/guard-ledger.jsonl")
}

fn main() -> Result<()> {
    sigpipe::reset();
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { hold, max_size, low_water, ts } => {
            let hold_path = hold.unwrap_or_else(default_hold);
            let cap = parse_size(&max_size)?;
            let low = low_water.map(|s| parse_size(&s)).transpose()?.unwrap_or(cap * 3 / 4);
            let ts_str = ts.unwrap_or_else(chrono_now);

            let units = measure::enumerate_units(&hold_path)?;
            let total: u64 = units.iter().map(|u| u.bytes).sum();
            let over_cap = total > cap;

            let selected = if over_cap {
                policy::select_evict_units(&units, total, cap, low)
            } else {
                vec![]
            };

            let reclaimed: u64 = selected.iter().map(|u| u.bytes).sum();

            let evt = event::Event {
                kind: "hold_guard_check".to_string(),
                severity: if over_cap { "warn".to_string() } else { "info".to_string() },
                hold_bytes: total,
                cap_bytes: cap,
                reclaimed_bytes: reclaimed,
                ts: ts_str,
                over_cap,
                dry_run: true,
                units_selected: selected.iter().map(|u| u.path.display().to_string()).collect(),
            };

            println!("{}", serde_json::to_string_pretty(&evt)?);
        }

        Commands::Enforce { hold, max_size, low_water, apply, ts, ledger } => {
            let hold_path = hold.unwrap_or_else(default_hold);
            let cap = parse_size(&max_size)?;
            let low = low_water.map(|s| parse_size(&s)).transpose()?.unwrap_or(cap * 3 / 4);
            let ts_str = ts.unwrap_or_else(chrono_now);
            let ledger_path = ledger.unwrap_or_else(default_ledger);

            let units = measure::enumerate_units(&hold_path)?;
            let total: u64 = units.iter().map(|u| u.bytes).sum();
            let over_cap = total > cap;

            let selected = if over_cap {
                policy::select_evict_units(&units, total, cap, low)
            } else {
                vec![]
            };

            let reclaimed = if apply {
                evict::remove_units(&selected)?
            } else {
                0
            };

            let evt = event::Event {
                kind: "hold_guard_enforce".to_string(),
                severity: if over_cap { "warn".to_string() } else { "info".to_string() },
                hold_bytes: total,
                cap_bytes: cap,
                reclaimed_bytes: reclaimed,
                ts: ts_str.clone(),
                over_cap,
                dry_run: !apply,
                units_selected: selected.iter().map(|u| u.path.display().to_string()).collect(),
            };

            if apply && !selected.is_empty() {
                ledger::append(&ledger_path, &evt)?;
            }

            println!("{}", serde_json::to_string_pretty(&evt)?);
        }

        Commands::Status { hold, max_size, ledger } => {
            let hold_path = hold.unwrap_or_else(default_hold);
            let ledger_path = ledger.unwrap_or_else(default_ledger);

            let total = measure::total_size(&hold_path).unwrap_or(0);
            let cap_str = max_size.as_deref().unwrap_or("(not set)");

            println!("hold_path: {}", hold_path.display());
            println!("hold_bytes: {total}");
            println!("cap: {cap_str}");

            if ledger_path.exists() {
                let tail = ledger::tail(&ledger_path, 5)?;
                println!("ledger_tail:");
                for line in tail {
                    println!("  {line}");
                }
            } else {
                println!("ledger: (none)");
            }
        }
    }

    Ok(())
}

fn parse_size(s: &str) -> Result<u64> {
    let s = s.trim();
    // Try numeric first
    if let Ok(n) = s.parse::<u64>() {
        return Ok(n);
    }
    // Parse suffixes: G/GB/GiB, M/MB/MiB, K/KB/KiB, T/TB/TiB
    let (num_part, suffix) = split_size(s)?;
    let base: u64 = num_part.parse().map_err(|_| anyhow::anyhow!("invalid size: {s}"))?;
    let multiplier = match suffix.to_uppercase().as_str() {
        "K" | "KB" => 1_000u64,
        "KIB" => 1_024,
        "M" | "MB" => 1_000_000,
        "MIB" => 1_048_576,
        "G" | "GB" => 1_000_000_000,
        "GIB" => 1_073_741_824,
        "T" | "TB" => 1_000_000_000_000,
        "TIB" => 1_099_511_627_776,
        "" => 1,
        other => anyhow::bail!("unknown size suffix: {other}"),
    };
    Ok(base * multiplier)
}

fn split_size(s: &str) -> Result<(&str, &str)> {
    let idx = s.find(|c: char| c.is_alphabetic())
        .ok_or_else(|| anyhow::anyhow!("no suffix found in size: {s}"))?;
    Ok((&s[..idx], &s[idx..]))
}

fn chrono_now() -> String {
    // Simple RFC3339 via system time — no chrono dep
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Format as YYYY-MM-DDTHH:MM:SSZ (UTC)
    let s = secs;
    let sec = s % 60;
    let min = (s / 60) % 60;
    let hour = (s / 3600) % 24;
    let days = s / 86400;
    // Gregorian calendar calculation
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    // Days since 1970-01-01
    let mut year = 1970u64;
    loop {
        let leap = is_leap(year);
        let days_in_year = if leap { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let month_days = [31u64, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u64;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}
