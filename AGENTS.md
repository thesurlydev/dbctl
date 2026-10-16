# AGENTS.md

Guidance for AI coding agents working in this repository.

## Project Overview

dbctl is a PostgreSQL version and database manager written in Rust. It installs and manages multiple PostgreSQL versions and database clusters without root access or system-wide installation. All state lives in `~/.dbctl/`.

The primary audience is local development: smart defaults, auto-selection of clusters when unambiguous, interactive prompts for missing arguments, and colored output.

## Repository Layout

```
/
├── Cargo.toml              # Workspace root (edition 2024, resolver 3)
├── crates/
│   └── dbctl/              # The only crate: both library and binary
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs     # CLI entry point (clap derive, tokio async main)
│           ├── lib.rs      # Library root; re-exports public API
│           ├── server.rs   # PostgreSQL install/uninstall, cluster lifecycle (start/stop/init/status/logs)
│           ├── postgres.rs # Database operations (createdb/dropdb/users/SQL exec/backup/restore/migrations)
│           ├── connection.rs # Remote connection profiles, SSL modes, connect/test
│           ├── extension.rs # Build/install/enable PostgreSQL extensions via PGXS (e.g. pgvector)
│           ├── cache.rs    # Local binary cache for downloaded PostgreSQL archives
│           └── context.rs  # Remembers last-used cluster (~/.dbctl/context.json)
├── tests/
│   └── integration_tests.rs # E2E tests that run the actual CLI via `cargo run`
├── QUICKSTART.md           # User-facing walkthrough of all major features
└── mise.toml               # Task runner config (dev/test/install tasks)
```

## Build, Test, and Lint Commands

```bash
# Build (debug)
cargo build
# or: mise run dev

# Run all tests (single-threaded; tests share ~/.dbctl state)
cargo test -- --test-threads=1
# or: mise run test

# Install the binary to ~/.cargo/bin
cargo install --path crates/dbctl
# or: mise run install

# Format and lint (run before considering work complete)
cargo fmt
cargo clippy --all-targets -- -D warnings
```

Important: integration tests MUST be run single-threaded (`--test-threads=1`). They operate against the real `~/.dbctl/` directory and will interfere with each other if run in parallel. The integration tests shell out to `cargo run -- ...` and perform real installs/cluster operations, so they are slow and require network access for PostgreSQL binary downloads (subsequent runs hit the local binary cache).

## Architecture and Conventions

- Single crate with both a `lib.rs` (library exposing all logic) and `main.rs` (thin CLI layer). CLI commands in `main.rs` use clap's derive API; subcommand enums like `ServerAction`, `DbAction`, `CacheAction`, `ExtensionAction` live in their respective modules so they can be reused.
- Async runtime is tokio (`#[tokio::main]`). Database connections use `tokio-postgres` with `rustls` for TLS.
- Error handling uses `anyhow` (`Result`, `bail!`) throughout.
- PostgreSQL binaries are downloaded from EnterpriseDB build archives, cached in `~/.dbctl/cache/postgres/` keyed by `{version}-{os}-{arch}`, and installed to `~/.dbctl/versions/<version>/`. Clusters live in `~/.dbctl/clusters/<name>/` with a `metadata.json` describing each cluster.
- `AVAILABLE_VERSIONS` in `crates/dbctl/src/server.rs` is the source of truth for supported PostgreSQL major versions and their latest minor releases.
- Cluster management shells out to the installed PostgreSQL binaries (`initdb`, `pg_ctl`, `psql`, `pg_dump`, etc.); extension builds shell out to PGXS (`make` with `PG_CONFIG` pointing at the managed install).
- Several commands support a `--json` flag for machine-readable output. Preserve this pattern when adding new list/status-style commands.
- The CLI favors smart defaults: cluster name arguments are optional when only one cluster exists, and stopped clusters are auto-started when accessed. Keep this UX when adding commands.
- `Server` and `Db` subcommands in `main.rs` are hidden backward-compatibility shims; new functionality goes in the top-level commands.

## Code Style

- Rust edition 2024 (workspace-wide). Do not downgrade.
- Keep modules focused: server/cluster lifecycle in `server.rs`, database-level operations in `postgres.rs`, connection profiles in `connection.rs`, etc.
- Use `colored` for user-facing terminal output and `dialoguer` for interactive prompts; use `indicatif` for progress bars on long-running operations (downloads, installs).
- Serialize state with `serde`/`serde_json` (see `ClusterMetadata`, `Context`, `ConnectionProfile`).
- Never require root or write outside `~/.dbctl/` (the VPS cache path `/data/cache/postgres/` in `cache.rs` is the one documented exception).
- No emojis in source, output, or commit messages. No em dashes; use a plain hyphen for ranges.

## Testing Guidance

- Tests are E2E by design: they invoke the built CLI and verify behavior a real user would see. When fixing a bug, reproduce it through the CLI first, then add or update an integration test covering it.
- Use the existing `cleanup_test_cluster` / `cleanup_test_version` helpers so tests do not leak state into the user's `~/.dbctl/`.
- Be careful when running tests on a machine where `~/.dbctl/` contains real user data; the tests share that directory.

## Commit and PR Guidance

- Do not commit changes unless explicitly asked.
- Do not add yourself as co-author in commit messages.
- `CHANGELOG.md` (if present) is auto-generated; never edit it manually.
- README.md and QUICKSTART.md document user-facing behavior; update them when commands, flags, or directory layouts change.
