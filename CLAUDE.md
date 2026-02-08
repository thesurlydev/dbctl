# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`dbctl` is a PostgreSQL version and database manager written in Rust. It allows users to install multiple PostgreSQL versions, manage database clusters, and perform database operations - all without requiring root access or system-wide installations.

**Design Philosophy**: Optimized for local development with "magic" defaults - minimal typing, smart auto-selection, and interactive prompts when needed.

## Architecture

### Workspace Structure

This is a Cargo workspace with two crates:

```
crates/
  dbctl/        # CLI tool and shared library
  dbctl-gui/    # Native GUI application (eframe/egui)
```

### CLI Crate (`crates/dbctl`)

Both a library and a binary. The library is re-exported for use by `dbctl-gui`.

- `src/main.rs` - CLI entry point using clap with flat command structure and auto-selection logic
- `src/lib.rs` - Re-exports all public types and functions from submodules
- `src/server.rs` - PostgreSQL installation (binary downloads from EnterpriseDB), cluster lifecycle (`pg_ctl` start/stop/restart/reload), `ClusterMetadata`, `ensure_cluster_running()`
- `src/postgres.rs` - Database operations via `tokio-postgres`: CRUD databases, interactive SQL REPL (`run_repl` using `simple_query` + `rustyline`), shell/exec (spawns psql/pg_dump binaries), backup/restore, user and privilege management, table browsing, schema introspection
- `src/context.rs` - Last-used cluster tracking (`~/.dbctl/context.json`)
- `src/connection.rs` - Remote PostgreSQL connection profiles (`ConnectionProfile`, `ConnectionStore`, SSL support), persisted to `~/.dbctl/connections.json`

### GUI Crate (`crates/dbctl-gui`)

Native desktop app built with eframe/egui. Depends on the `dbctl` library crate.

- `src/main.rs` - App entry point, `DbctlApp` struct, top-level UI rendering, async result handling via `mpsc` channels
- `src/state.rs` - Shared types: `Tab` enum (Overview/Databases/Users/Logs/Sql/Tables/SchemaDesigner), `AsyncResult` enum, `ActiveConnection`
- `src/tabs/` - One module per tab (overview, databases, users, logs, sql, tables, schema_designer)
- `src/schema_designer/` - Visual schema editor: `mod.rs` (state), `canvas.rs` (rendering, crow's foot FK notation), `diff.rs` (SQL migration and DDL generation), `actions.rs` (undo/redo with timestamps), `models.rs` (data types), `templates.rs` (pre-built schema templates)
- `src/ai_chat/` - Claude API integration for natural language schema design: `mod.rs` (state, config), `claude_client.rs` (API calls, prompt engineering)
- `src/command_palette.rs` - Fuzzy-searchable command palette (Cmd+K)
- `src/theme.rs` - Dark/light/system theme support, persisted to `~/.dbctl/gui-config.json`
- `src/dialogs.rs` - Modal dialogs (settings, confirmations)
- `src/syntax.rs` - SQL syntax highlighting
- `src/sql_state.rs` - SQL tab state (multiple query tabs, history)
- `src/table_browser.rs` - Table browser state

### Key Design Decisions

**Runtime data directory**: All PostgreSQL installations and clusters are stored in `~/.dbctl/`:
- `~/.dbctl/versions/{version}/` - PostgreSQL binary installations
- `~/.dbctl/clusters/{name}/` - Database clusters with `metadata.json`, `data/`, and `logs/`
- `~/.dbctl/context.json` - Last-used cluster for smart defaults
- `~/.dbctl/connections.json` - Saved remote connection profiles
- `~/.dbctl/gui-config.json` - GUI theme preferences
- `~/.dbctl/ai-config.json` - AI assistant settings (model, API key via keyring)
- `~/.dbctl/repl_history` - SQL REPL command history
- `~/.dbctl/queries/` - Saved SQL queries
- `~/.dbctl/schema-layouts/` - Persisted schema designer table positions

**Cluster auto-selection priority**: (1) explicit `--cluster` flag, (2) auto-select if only one cluster exists, (3) last-used from context.json, (4) error with suggestions.

**Auto-start**: Database commands call `ensure_cluster_running()` to automatically start stopped clusters.

**Installation**: Downloads prebuilt PostgreSQL binaries from EnterpriseDB. macOS uses .zip (extracted with `zip` crate, with symlink handling for shared libraries), Linux uses .tar.gz (extracted with `tar` + `flate2` crates). All extraction is pure Rust, no external tools needed.

**GUI async pattern**: Database operations run on a tokio runtime, communicate results back to the egui render loop via `mpsc::Sender<AsyncResult>` / `mpsc::Receiver<AsyncResult>`.

**Dual connection model**: The GUI supports both local clusters (via `ClusterMetadata`) and remote connections (via `ConnectionProfile`). Functions with `_with_source` suffix accept a `ConnectionSource` enum to handle both.

## Build and Development Commands

### Using Justfile

```bash
just         # List all available commands
just build   # Build the entire workspace
just test    # Run all tests (--test-threads=1)
just fmt     # Format code
just clippy  # Run clippy with -D warnings
just check   # Run fmt + clippy
```

### Using Cargo Directly

```bash
cargo build                                              # Build entire workspace
cargo build -p dbctl                                     # Build CLI only
cargo build -p dbctl-gui                                 # Build GUI only
cargo run -p dbctl -- <command>                          # Run CLI with arguments
cargo run -p dbctl-gui                                   # Run GUI
cargo test -- --test-threads=1                           # Run all tests
cargo test --test integration_tests -- --test-threads=1  # Integration tests only
cargo test <test_name> -- --test-threads=1               # Run a specific test
cargo clippy --all-targets --all-features -- -D warnings # Lint
cargo fmt                                                # Format
```

**Integration tests** are at `tests/integration_tests.rs` (workspace root). They download real PostgreSQL binaries and run actual cluster/database operations. Tests MUST run with `--test-threads=1` to avoid port conflicts.

## CLI Command Structure

Flat commands are the primary interface. The nested `server`/`db` subcommand structure is still supported but hidden from help.

**Cluster lifecycle**: `install`, `versions`, `init`, `start`, `stop`, `restart`, `reload`, `status`, `list`/`ls`, `remove`/`rm`, `logs`

**Database operations**: `createdb`, `dropdb`, `databases`, `use`, `shell`, `exec`, `backup`, `restore`

**User management**: `createuser`, `users`

**Privilege management** (nested only): `db grant`, `db revoke`, `db privileges`

## Development Notes

- Uses tokio async runtime (required for tokio-postgres)
- Database operations connect to the `postgres` maintenance database by default
- Clusters use `trust` authentication in `pg_hba.conf` for local connections
- `use` command runs a built-in SQL REPL via `tokio-postgres::Client::simple_query()` and `rustyline` -- does not require PostgreSQL binaries, unlike `shell`/`exec`
- `shell`, `exec`, `backup`, `restore` spawn PostgreSQL binaries (psql, pg_dump) from the cluster's version directory via `get_version_dir()`
- Schema designer visual column order (`SchemaDesignerState.visual_column_order`) is separate from the actual database schema order
- Schema designer undo/redo uses `TimestampedAction` wrapping `SchemaAction` with `chrono::DateTime<Local>`, max 100 actions
- All GUI colors come from `Theme` struct fields, never hardcoded -- required for dark/light mode support
- Always update README.md, QUICKSTART.md, and CLAUDE.md after making changes to ensure they reflect the current code
- Always run all tests after each change to verify everything still works
