# AGENTS.md

Guidance for AI coding agents working in this repository.

## Project Overview

dbctl is a PostgreSQL version and database manager written in Rust. It installs and manages multiple PostgreSQL versions and database clusters without root access or system-wide installation. All state lives in `~/.dbctl/`.

The primary audience is local development: smart defaults, auto-selection of clusters when unambiguous, interactive prompts for missing arguments, and colored output. It is also driven by programs: automation runs it over SSH on servers, with no terminal, and parses its `--json` output.

Default branch is `master`; remotes are `origin` (GitHub) and `forgejo`.

## Repository Layout

```
/
├── Cargo.toml              # Workspace root (edition 2024, resolver 3)
├── crates/
│   └── dbctl/              # The only crate: both library and binary
│       ├── Cargo.toml
│       ├── tests/
│       │   └── cli.rs      # E2E tests of the real binary, each with a scratch HOME
│       └── src/
│           ├── main.rs     # CLI entry point (clap derive, tokio async main)
│           ├── lib.rs      # Library root; re-exports public API
│           ├── server.rs   # PostgreSQL install/uninstall, cluster lifecycle (start/stop/init/status/logs)
│           ├── postgres.rs # Database operations (createdb/dropdb/users/SQL exec/backup/restore/migrations)
│           ├── connection.rs # Remote connection profiles, SSL modes, connect/test
│           ├── extension.rs # Build/install/enable PostgreSQL extensions via PGXS (e.g. pgvector)
│           ├── cache.rs    # Local binary cache for downloaded PostgreSQL archives
│           └── context.rs  # Remembers last-used cluster (~/.dbctl/context.json)
├── QUICKSTART.md           # User-facing walkthrough of all major features
└── mise.toml               # Task runner config (dev/test/install tasks)
```

## Build, Test, and Lint Commands

```bash
# Build (debug)
cargo build
# or: mise run dev

# Fast tests (no PostgreSQL needed)
cargo test
# or: mise run test

# CLI tests that install PostgreSQL (and build pgvector)
cargo test --test cli -- --ignored
# or: mise run test-integration

# Everything that must pass before committing: build, test, clippy -D warnings, fmt --check
# (`check` is an alias). CI on Forgejo runs this same task.
mise run gate

# Install the binary to ~/.cargo/bin
cargo install --path crates/dbctl
# or: mise run install

```

The CLI tests run the built binary with `HOME` pointed at a scratch directory, so they never touch your `~/.dbctl/`, and with stdin closed, the way automation runs dbctl: no command may wait on a prompt. Each PostgreSQL test gets its own cluster and port, so they run in parallel. They install PostgreSQL once per run, reusing your local download cache when it has the archive.

## Architecture and Conventions

- Single crate with both a `lib.rs` (library exposing all logic) and `main.rs` (thin CLI layer). CLI commands in `main.rs` use clap's derive API; subcommand enums like `ServerAction`, `DbAction`, `CacheAction`, `ExtensionAction` live in their respective modules so they can be reused.
- Async runtime is tokio (`#[tokio::main]`). Database connections use `tokio-postgres` with `rustls` for TLS.
- Error handling uses `anyhow` (`Result`, `bail!`) throughout.
- On macOS, PostgreSQL binaries are downloaded from EnterpriseDB build archives, cached in `~/.dbctl/cache/postgres/` keyed by `{version}-{os}-{arch}`, and installed to `~/.dbctl/versions/<version>/`. On Linux, EnterpriseDB publishes no binaries for current versions, so `install` links `~/.dbctl/versions/<version>` to the distribution's package (`/usr/lib/postgresql/<major>`, from apt.postgresql.org). Clusters live in `~/.dbctl/clusters/<name>/` with a `metadata.json` describing each cluster.
- `AVAILABLE_VERSIONS` in `crates/dbctl/src/server.rs` is the source of truth for supported PostgreSQL major versions and their latest minor releases.
- Cluster management shells out to the installed PostgreSQL binaries (`initdb`, `pg_ctl`, `psql`, `pg_dump`, etc.); extension builds shell out to PGXS (`make` with `PG_CONFIG` pointing at the managed install).
- Several commands support a `--json` flag for machine-readable output (see the README's Scripting section). Callers parse it and the tests pin the shapes, so treat it as a public interface: changing a shape is a breaking change. Preserve this pattern when adding new list/status-style commands.
- The CLI favors smart defaults: cluster name arguments are optional when only one cluster exists, and the human-readable commands auto-start a stopped cluster when they access it. JSON reads do not: they report the cluster as it is, failing on a stopped one. Keep this UX when adding commands.
- `Server` and `Db` subcommands in `main.rs` are hidden backward-compatibility shims; new functionality goes in the top-level commands.

## Code Style

- Keep modules focused: server/cluster lifecycle in `server.rs`, database-level operations in `postgres.rs`, connection profiles in `connection.rs`, etc.
- Use `colored` for user-facing terminal output and `dialoguer` for interactive prompts; use `indicatif` for progress bars on long-running operations (downloads, installs).
- Serialize state with `serde`/`serde_json` (see `ClusterMetadata`, `Context`, `ConnectionProfile`).
- Never require root or write outside `~/.dbctl/` (the VPS cache path `/data/cache/postgres/` in `cache.rs` is the one documented exception).

## Testing Guidance

- Tests are E2E by design: they invoke the built CLI and verify behavior a real user would see. When fixing a bug, reproduce it through the CLI first, then add or update an integration test covering it.
- Use the `Cluster` guard in `tests/cli.rs` (it removes its cluster when dropped, even on failure) and give each test its own port. Tests that need PostgreSQL are `#[ignore]`d so `cargo test` stays fast.

## Docs

- README.md and QUICKSTART.md document user-facing behavior; update them when commands, flags, or directory layouts change.
