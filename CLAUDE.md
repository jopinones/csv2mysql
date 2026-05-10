# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**csv2mysql** is a Rust TUI/CLI tool for loading CSV files into MySQL databases, built for the SUPEREDUC team. It handles automatic schema detection, pre-load validation, dry-run mode, batch insertion, and full audit logging.

## Build & Run Commands

```bash
# Build
cargo build                        # Debug build (all crates)
cargo build --release              # Release build

# Run
./target/debug/csv2mysql tui       # Launch interactive TUI
./target/debug/csv2mysql run --file data.csv --config configs/rol_cobro.toml
./target/debug/csv2mysql validate --file data.csv
./target/debug/csv2mysql init-db --database-url "mysql://user:pass@host/db"

# Tests
cargo test                         # All tests
cargo test -p csv2mysql-core       # Core crate only
cargo test retry                   # Tests matching "retry"

# Code quality
cargo clippy -- -D warnings
cargo fmt
cargo fmt --check
```

## Workspace Architecture

Three-crate workspace with clean separation:

- **`crates/core/`** — Pure business logic library (no UI). All domain types, parsing, validation, loading, retry, and audit logging live here. Depends on: sqlx, csv, encoding_rs, tokio.
- **`crates/tui/`** — Interactive terminal UI using ratatui + crossterm. Depends on core. Entry: `src/main.rs` initializes crossterm terminal, runs event loop with 35/65 layout split.
- **`crates/cli/`** — Thin clap-based command dispatcher. Subcommands delegate to core. Entry: `src/main.rs`.

## Core Module Responsibilities

| Module | Responsibility |
|---|---|
| `types.rs` | All data structures: `SqlType`, `ColumnDef`, `TableSchema`, `DataRow`, `LoadStats`, `ExecutionMode`, `ProcessLog` |
| `config.rs` | TOML config loading (`AppConfig`, `DatabaseConfig`, `DatasetConfig`) |
| `parser.rs` | CSV parsing: encoding detection (UTF-8 / Windows-1252), delimiter sniffing, SHA-256 file hashing |
| `schema.rs` | Type inference from CSV column samples → SQL types (INT, BIGINT, DECIMAL, DATE, DATETIME, VARCHAR, TEXT) |
| `validator.rs` | Pre-load validation: type checking, NULL checks, length checks, duplicate detection |
| `loader.rs` | Batch MySQL insertion with transactions and progress tracking |
| `retry.rs` | Exponential backoff retry (max 5 retries, 100ms–5s delays); `is_retryable()` on errors |
| `proc_log.rs` | Audit trail: inserts/updates `proceso_log` and `proceso_log_detalle` tables |
| `error.rs` | `Csv2MysqlError` enum with 11+ variants; `is_retryable()` method |

## Implementation Status

The framework is established but integration between TUI/CLI and core is incomplete. Key TODOs:

- `tui/src/app.rs`: `execute()`, `dry_run()`, `validate()` methods are stubs — need to wire mpsc channels and call core async functions
- `cli/src/main.rs`: `run`, `validate`, `init-db` subcommands are scaffolded but not functional
- No UI rendering for validation errors yet
- No async progress event streaming from core → TUI yet

## Configuration

Two-layer config system:
1. **Database config** (`configs/default.toml`) — connection, pool settings
2. **Dataset config** (e.g., `configs/rol_cobro.toml`) — per-file column mapping, SQL types, nullability, date formats

`DATABASE_URL` env var (in `.env`, git-ignored) overrides config file for sqlx.

## Database

Run `migrations/0001_proceso_log.sql` once to create the audit tables (`proceso_log`, `proceso_log_detalle`). This is what `init-db` subcommand should do.

sqlx uses compile-time query checking — `DATABASE_URL` must be set at build time for `query!` macros, or use `query_as!` with `SQLX_OFFLINE=true` and a prepared query cache.

## TUI Keyboard Shortcuts

`↑↓` navigate list, `→` enter directory, `←` go back, `E` execute load, `D` dry-run, `V` validate, `q` quit.
