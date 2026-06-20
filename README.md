# hold-guard

Caps the shared Cargo build hold and evicts least-recently-used artifacts so it can never silently regrow to fill the disk.

## Why it exists

A shared Cargo `target/` is a commons, and commons accrete. Every new dependency version leaves its compiled artifacts behind; nothing in Cargo's model ever takes them back out. On the wintermute fleet that hold grew to 214G and filled the disk to 97%.

`cargo clean` is the wrong tool — it drops everything, so the next build is cold. The artifacts you want to keep are exactly the ones you built most recently. hold-guard measures the hold against a size cap and, when it's over, evicts oldest-accessed artifacts first until the hold is back under a low-water mark. Hot crates survive; builds stay warm; the budget holds.

## Install

```sh
cargo install --path .
```

## Usage

Three subcommands. The hold path defaults to `~/wintermute/.hold/target`; the ledger to `~/wintermute/.hold/guard-ledger.jsonl`.

```sh
# Dry-run: is the hold over cap, and what would be evicted?
hold-guard check --max-size 60G

# Enforce the cap. Dry-run by default — pass --apply to actually remove.
hold-guard enforce --max-size 60G --apply

# Show current size vs cap and the tail of the ledger.
hold-guard status --max-size 60G
```

`check` is read-only and emits a JSON event reporting `over_cap`, current `hold_bytes`, the cap, and the units it would evict. `enforce` does the same but, with `--apply`, removes the selected units, records `reclaimed_bytes`, and appends a line to the ledger. Sizes accept `60G`, `60GB`, or raw bytes. `--low-water` sets the eviction target (default: 75% of the cap). `--ts <rfc3339>` pins timestamps for deterministic output.

## How it works

The eviction unit is the per-fingerprint subdirectory under `deps/`, `.fingerprint/`, and `incremental/`. Cargo's layout keeps these under stable names, and their mtime reflects last use — so LRU eviction by fingerprint directory is both safe and cheap, because Cargo simply rebuilds an evicted artifact the next time it needs it.

Two guarantees keep it safe to run unattended:

- **Dry-run by default.** Nothing is removed unless you pass `--apply`.
- **Locks are never evicted.** A unit with a `.cargo-lock` file is skipped even when it's the oldest candidate, so a build in flight is never touched.

Every applied eviction appends to an append-only JSONL ledger (line count only grows), and the emitted event matches ballast-guard's event schema, so one notifier can surface both whole-disk and hold-budget events.

## Where it fits

hold-guard is part of the `hold-*` family that manages the wintermute fleet's shared Cargo target:

- **hold-survey** — measures dependency duplication across the fleet
- **hold-migrate** — drains private `target/` dirs into the shared hold
- **hold-anchor** — deduplicates artifacts across machines
- **hold-guard** — bounds the hold's budget with LRU eviction (this repo)

It composes with ballast-guard (whole-disk fossils) rather than replacing it: ballast-guard watches the disk, hold-guard watches the hold.
