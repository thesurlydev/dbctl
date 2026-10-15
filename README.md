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
- Supports macOS and Linux (x86_64 and ARM64)


## Installation

```bash
cargo install --path .
```

## Quick Start

See [QUICKSTART.md](QUICKSTART.md) for a comprehensive guide with examples of all major features.

Basic workflow:

```bash
dbctl install 16.3        # Install PostgreSQL
dbctl init mydev          # Create a cluster (interactive)
dbctl start               # Start the cluster
dbctl createdb myapp      # Create a database
dbctl use myapp           # Start interactive SQL REPL
```

### Scripting

Without a terminal (CI, scripts, automation), `dbctl init` never
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
```

`databases` and `users` with `--json` require a running cluster rather than
starting one. callers parse this output, and the tests pin it in
tests, so treat it as an interface.

## Directory Structure

All data is stored in `~/.dbctl/`:

```
~/.dbctl/
├── context.json       # Remembers last-used cluster
├── cache/
│   └── postgres/      # Downloaded binary archives, reused across installs
├── versions/
│   └── 16.3/          # PostgreSQL binaries
│       ├── bin/
│       ├── lib/
│       └── share/
└── clusters/
    └── dev/           # Cluster named "dev"
        ├── data/      # PostgreSQL data directory
        ├── logs/      # Log files
        └── metadata.json  # Cluster configuration
```

## Binary Cache

`dbctl install` caches downloaded PostgreSQL archives locally, keyed by version, OS, and architecture, so reinstalling a version (e.g. after `dbctl remove`) skips the download.

```bash
# Show cache location, file count, and total size
dbctl cache stats

# Remove all cached archives
dbctl cache clear
```

## Requirements

- Rust 1.75 or later
- macOS or Linux (x86_64 or ARM64)
- Internet connection for downloading PostgreSQL binaries

No external tools required for runtime - all archive extraction is handled natively in Rust.
