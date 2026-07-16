# dbctl

A PostgreSQL version and database manager. Install and manage multiple PostgreSQL versions and database clusters without requiring root access or system-wide installations.

Designed for local development with smart defaults - minimal typing, auto-selection, and helpful prompts.

## Features

- Install multiple PostgreSQL versions side-by-side
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

## Directory Structure

All data is stored in `~/.dbctl/`:

```
~/.dbctl/
├── context.json       # Remembers last-used cluster
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

## Requirements

- Rust 1.75 or later
- macOS or Linux (x86_64 or ARM64)
- Internet connection for downloading PostgreSQL binaries

No external tools required for runtime - all archive extraction is handled natively in Rust.
