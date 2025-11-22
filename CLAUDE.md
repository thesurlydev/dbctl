# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`dbctl` is a PostgreSQL version and database manager written in Rust. It allows users to install multiple PostgreSQL versions, manage database clusters, and perform database operations - all without requiring root access or system-wide installations.

**Design Philosophy**: Optimized for local development with "magic" defaults - minimal typing, smart auto-selection, and interactive prompts when needed.

## Architecture

The codebase is organized into four main modules:

- `src/main.rs` - CLI entry point with flat command structure and auto-selection logic
- `src/context.rs` - Manages last-used cluster context for smart defaults
- `src/server.rs` - PostgreSQL server installation and cluster management
- `src/postgres.rs` - Database operations (create/drop/list databases, shell, exec, backup/restore, user management, privilege management, info)

### Key Design Decisions

**Directory Structure**: All PostgreSQL installations and clusters are stored in `~/.dbctl/`:
- `~/.dbctl/versions/{version}/` - PostgreSQL binary installations
- `~/.dbctl/clusters/{name}/` - Database clusters with metadata, data, and logs
- `~/.dbctl/context.json` - Stores last-used cluster for smart defaults

**Cluster Metadata**: Each cluster has a `metadata.json` file storing version, port, username, and directory paths. This is loaded by `load_cluster_metadata()` in `src/server.rs`.

**Installation Strategy**: Downloads prebuilt binaries from EnterpriseDB for both macOS (universal binaries) and Linux (architecture-specific). macOS uses .zip files (extracted with the `zip` crate), Linux uses .tar.gz files (extracted with the `tar` and `flate2` crates). All extraction is done in-memory using pure Rust implementations, requiring no external tools.

**Smart Defaults**: Commands automatically select clusters based on:
1. Explicit `--cluster` flag
2. Auto-selection if only one cluster exists
3. Last-used cluster from `~/.dbctl/context.json`
4. Error with helpful suggestions if ambiguous

**Auto-Start**: Database commands automatically start stopped clusters using `ensure_cluster_running()`.

**Interactive Mode**: Missing arguments trigger interactive prompts (using `dialoguer` crate).

**Colored Output**: Status and error messages use colored text (using `colored` crate) for better readability.

## Command Structure

### Flat Commands (Primary Interface)

**Installation & Clusters:**
- `install <version>` - Install PostgreSQL
- `init <name>` - Create cluster (interactive if args omitted)
- `start [cluster]` - Start cluster (auto-selects)
- `stop [cluster]` - Stop cluster
- `restart [cluster]` - Restart cluster
- `reload [cluster]` - Reload config
- `status [cluster]` - Show status (all if omitted)
- `list` / `ls` - List all clusters and databases
- `remove <cluster>` / `rm` - Remove cluster (with confirmation)
- `logs [cluster]` - View logs

**Databases:**
- `createdb <name>` - Create database (auto-selects cluster, auto-starts)
- `dropdb <name>` - Drop database (with confirmation)
- `databases` - List all databases
- `shell [database]` - Open psql (auto-selects cluster, auto-starts)
- `exec --sql "..." -d <db>` - Execute SQL
- `backup <db> -o <file>` - Backup
- `restore <db> -i <file>` - Restore

**Users:**
- `createuser <name>` - Create user (prompts for password)
- `users` - List users

### Backward Compatibility

The nested command structure (`server`/`db` subcommands) is still supported but hidden from help.

## Common Commands

### Using Justfile (Recommended)

The project includes a Justfile for common tasks:

```bash
just         # List all available commands
just build   # Build the project
just test    # Run all tests
just fmt     # Format code
just clippy  # Run linter
just check   # Run format and lint checks
just install # Install to ~/.cargo/bin
```

### Using Cargo Directly

```bash
cargo build                                              # Build the project
cargo run -- <command>                                   # Run with arguments
cargo build --release                                    # Build optimized binary
cargo test                                               # Run all tests
cargo test --test integration_tests -- --test-threads=1  # Run integration tests
cargo test <test_name>                                   # Run a specific test
cargo clippy                                             # Run linter
cargo fmt                                                # Format code
```

**Integration Tests** (located in `tests/integration_tests.rs`): Comprehensive suite covering all commands including installation, cluster management, database operations, and error handling. Tests run sequentially to avoid port conflicts.

### Example Usage (New Commands)

```bash
# No arguments - show status of all clusters
cargo run

# Install and initialize
cargo run -- install 16.3
cargo run -- init mydev  # Interactive prompts for version, port, etc.

# Start cluster (auto-selected if only one)
cargo run -- start

# Create database (auto-starts cluster if stopped)
cargo run -- createdb myapp

# Open shell (auto-selects cluster, auto-starts)
cargo run -- shell myapp

# Create user (prompts for password)
cargo run -- createuser alice

# List everything
cargo run -- ls

# Multiple clusters - uses last-used or shows options
cargo run -- start dev
cargo run -- createdb testdb  # Uses 'dev' cluster from context

# Explicit cluster specification
cargo run -- createdb proddb -c production
```

## Development Notes

- The project uses tokio for async runtime, required for tokio-postgres
- Database operations connect to the `postgres` maintenance database by default
- Clusters are configured with `trust` authentication for local connections in `pg_hba.conf`
- Port configuration is written to `postgresql.conf` during cluster initialization
- The `ClusterMetadata` struct is the central configuration object loaded by both server and database operations
- **Symlink handling**: The ZIP extraction code in `src/server.rs` properly handles symlinks in macOS archives by detecting them via `file.is_symlink()` and creating actual symlinks instead of extracting them as text files. This is critical for PostgreSQL's shared library dependencies (libicu*, libpq, etc.)
- **Command execution**: The `shell`, `exec`, `backup`, and `restore` commands use PostgreSQL's own binaries (psql, pg_dump) from the cluster's version directory via `get_version_dir()`
- **User management**: User creation and management operations connect to the `postgres` database and use SQL commands directly via tokio-postgres
- **Privilege management**: The `grant`, `revoke`, and `privileges` commands provide comprehensive privilege management for databases, schemas, and tables
- **Logs**: The `logs` command reads from the cluster's logs directory and uses `tail -f` for follow mode
- **Cluster control**: `start`, `stop`, `restart`, and `reload` commands use `pg_ctl` from the cluster's version directory
- **Context management**: The `Context` struct in `src/context.rs` manages the last-used cluster, stored in `~/.dbctl/context.json`, enabling smart auto-selection
- **Auto-start**: `ensure_cluster_running()` in `src/server.rs` automatically starts stopped clusters before database operations
- **Interactive prompts**: Using the `dialoguer` crate for password prompts, confirmations, and missing arguments
- **Colored output**: Using the `colored` crate for status symbols (●/○) and colored text in errors and status displays
- The primary use case for this tool is development and testing
- Best practices for CLI apps should be used when adding or updating features
- Always update @README.md and @CLAUDE.md after making changes to ensure they reflect the current code
- Always run all tests after each change to verify everything still works