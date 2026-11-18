# dbctl

A PostgreSQL version and database manager. Install and manage multiple PostgreSQL versions and database clusters without requiring root access or system-wide installations.

Designed for local development with smart defaults - minimal typing, auto-selection, and helpful prompts.

## Features

- Install multiple PostgreSQL versions side-by-side
- Local binary cache speeds up repeat installs and reinstalls
- Create and manage isolated database clusters
- Start, stop, restart, and reload cluster configurations
- Monitor cluster status and view logs
- Create and drop databases within clusters
- Database backup and restore operations
- User management (create, list, drop users)
- Smart cluster auto-selection (no need to specify cluster each time)
- Auto-start stopped clusters when accessing them
- Interactive prompts for missing information
- Colored output for better readability
- No root access required - all data stored in `~/.dbctl/`
- Downloads are verified: every PostgreSQL archive is checked against a pinned
  SHA-256 before it is unpacked
- Supports macOS and Linux (x86_64 and ARM64). On macOS dbctl downloads
  PostgreSQL; on Linux it uses the distribution's packages
  (`apt-get install postgresql-16` from apt.postgresql.org), since no Linux
  binaries are published for current versions


## Installation

Requires Rust 1.98 or later.

```bash
cargo install --git https://github.com/thesurlydev/dbctl dbctl
```

Or from a checkout: `cargo install --path crates/dbctl` (`mise run install`).

## Quick Start

See [QUICKSTART.md](QUICKSTART.md) for a comprehensive guide with examples of all major features.

Basic workflow:

```bash
dbctl install 16          # Install PostgreSQL 16 (the latest pinned 16.x)
dbctl init mydev          # Create a cluster (interactive)
dbctl start               # Start the cluster
dbctl createdb myapp      # Create a database
dbctl use myapp           # Start interactive SQL REPL
```

### Authentication

`dbctl init` takes `--auth trust` (the default) or `--auth password`:

- **trust**: every local connection is trusted. Convenient on a laptop, where
  nothing untrusted runs, but any user on the machine can connect over
  127.0.0.1 as the superuser, so use password mode on a shared machine. The
  unix socket is owner-only (0700) in both modes.
- **password**: TCP logins need a scram-sha-256 password, and the unix socket
  is owner-only (0700), so only the OS user running PostgreSQL can bypass
  passwords. What servers use: create the superuser without a password, give
  each application its own role and database, and connect applications over
  TCP with that role's password.

```bash
dbctl init main --version 16 --port 5432 --username postgres --auth password
printf '%s\n' "$APP_PASSWORD" | dbctl create-user app --password-stdin -c main
dbctl create-db app --owner app -c main
```

dbctl itself always connects through the cluster's unix socket, so it works
under either mode without a password.

### Running under systemd

`dbctl run <cluster>` runs PostgreSQL in the foreground, replacing the dbctl
process, so a service manager supervises the database directly. Stop it with
SIGINT (PostgreSQL's fast shutdown):

```ini
[Service]
User=postgres
ExecStart=/usr/local/bin/dbctl run main
KillSignal=SIGINT
KillMode=mixed
Restart=always
```

### Scripting

Without a terminal (CI, scripts, automation over SSH), `dbctl init` never
prompts: pass the cluster name (and `--version` when several are installed),
and it leaves the cluster stopped for an explicit `dbctl start`. `start` on a
running cluster succeeds without doing anything.

Machine-readable output, one JSON value on stdout:

```bash
dbctl versions --json                 # {"available": [...], "installed": [...]}
dbctl list --json                     # [{"name","version","port","username","status"}]
dbctl status <name> --json            # one cluster, same shape as list
dbctl databases --cluster <c> --json  # ["app", "postgres"]
dbctl users --cluster <c> --json      # [{"name","superuser","create_db","create_role"}]
dbctl extension list -c <c> --json              # ["hstore", "vector", ...] available to install
dbctl extension list -c <c> -d <db> --json      # ["plpgsql", "vector"] created in <db>
```

Secrets and backups:

```bash
printf '%s\n' "$PW" | dbctl create-user app --password-stdin -c main  # not in ps
dbctl dumpall -c main -o /backups/main.sql   # every database and role, written atomically
```

`databases`, `users`, and `extension list --database` with `--json` require a
running cluster rather than starting one. These shapes are pinned in tests and
are a stable interface: a change to one is a breaking change.

## Development

```bash
mise run gate               # build, test, clippy -D warnings, fmt --check
mise run test-integration   # CLI tests that install PostgreSQL (slow)
```

`crates/dbctl/tests/cli.rs` runs the real binary with HOME pointed at a
scratch directory and stdin closed, so it never touches your `~/.dbctl` and
catches anything that would wait on a prompt. The PostgreSQL tests reuse your
`~/.dbctl/cache/postgres` archives (read-only) when present.

## Directory Structure

All data is stored in `~/.dbctl/`:

```
~/.dbctl/
├── context.json       # Remembers last-used cluster
├── cache/
│   └── postgres/      # Downloaded binary archives, reused across installs
├── versions/
│   └── 16/            # PostgreSQL binaries
│       ├── bin/
│       ├── lib/
│       └── share/
└── clusters/
    └── dev/           # Cluster named "dev"
        ├── data/      # PostgreSQL data directory
        ├── logs/      # Log files
        └── metadata.json  # Cluster configuration
```

## Verified Downloads

On macOS, dbctl installs PostgreSQL from EnterpriseDB's binary archives, and
only the releases listed in `AVAILABLE_VERSIONS` (`crates/dbctl/src/server.rs`),
each pinned to the SHA-256 of its archive. Name a release by its major version
(`dbctl install 16`) or in full (`dbctl install 16.14`); `dbctl versions` lists
them. A download is hashed as it streams and kept only when it matches, and a
cached archive is checked again before every install. The pgvector source that
`dbctl extension install pgvector` builds is pinned to a commit the same way.

## Binary Cache

`dbctl install` caches downloaded PostgreSQL archives locally, keyed by version, OS, and architecture, so reinstalling a version skips the download.

```bash
# Show cache location, file count, and total size
dbctl cache stats

# Remove all cached archives
dbctl cache clear
```

## Requirements

- Rust 1.98 or later to build
- macOS or Linux (x86_64 or ARM64)
- macOS: an internet connection for downloading PostgreSQL binaries
- Linux: PostgreSQL installed from your distribution's packages

No external tools required for runtime - all archive extraction is handled natively in Rust.

## Security

See [SECURITY.md](SECURITY.md) for how to report a vulnerability and what dbctl
does and does not protect against.

## License

Apache-2.0. See [LICENSE](LICENSE).
