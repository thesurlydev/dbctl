# dbctl

A PostgreSQL version and database manager for developers. Install and manage multiple PostgreSQL versions and database clusters without requiring root access or system-wide installations.

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

### GUI Features (dbctl-gui)

- Visual cluster management with start/stop controls
- Database and user management interfaces
- **SQL Query Tab** with:
  - Multiple query tabs (up to 10)
  - Syntax highlighting (keywords, types, strings, numbers, comments)
  - Query history with recall (Cmd+Up/Down to navigate)
  - Save/load queries to files (~/.dbctl/queries/)
  - EXPLAIN and EXPLAIN ANALYZE buttons
  - Keyboard shortcuts (Cmd+Enter to execute, Cmd+S to save)
- **Table Browser** with:
  - Schema tree navigation with collapsible schemas
  - Table structure viewer (columns, constraints, indexes)
  - Paginated data browsing with configurable page size
  - Insert and delete rows (tables with primary keys)
  - Export data to CSV/JSON (copied to clipboard)
- **Command Palette** (Cmd+K):
  - Fuzzy search across 45+ commands
  - Quick access to all app functionality
  - Keyboard shortcuts displayed for each command
  - Categories: Cluster, Database, Table, Schema, SQL, Users, Connections, Navigation

## Installation

Using just (recommended):

```bash
just install              # Install to ~/.cargo/bin
just install-system       # Install to /usr/local/bin (requires sudo)
```

Or using cargo directly:

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
sudo cp target/release/dbctl /usr/local/bin/
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

## Command Overview

### No arguments - Show status

Just run `dbctl` to see all your clusters and their status:

```bash
dbctl
```

Shows a colored overview of all clusters with running status, version, and ports.

### Smart Auto-Selection

Most commands don't require specifying a cluster:
- If you have only one cluster, it's automatically selected
- If you have multiple clusters, the last-used cluster is remembered
- Override anytime with `--cluster` or `-c`

```bash
# These all work without specifying cluster (if you have one cluster):
dbctl start
dbctl createdb myapp
dbctl shell myapp
dbctl users

# With multiple clusters, specify once and it's remembered:
dbctl start dev
dbctl createdb myapp  # Uses 'dev' automatically
```

## Usage

### Installation & Cluster Management

#### Install PostgreSQL

```bash
dbctl install <VERSION>
```

Example: `dbctl install 16.3`

#### List Available Versions

```bash
dbctl versions
```

Shows all PostgreSQL versions available to install, marking which are already installed.

#### Initialize a Cluster (Interactive)

```bash
dbctl init <NAME>
```

Prompts for version, port, and username if not provided:

```bash
dbctl init mydev
# Prompts:
# - PostgreSQL version: [shows installed versions]
# - Port: [5432]
# - Superuser name: [postgres]
# - Start now? [Y/n]
```

Or specify everything explicitly:

```bash
dbctl init production --version 16.3 --port 5433 --username admin
```

#### Start / Stop / Restart Cluster

```bash
dbctl start [cluster]
dbctl stop [cluster]
dbctl restart [cluster]
```

Cluster name is optional - auto-selected if only one exists.

#### Reload Configuration

```bash
dbctl reload [cluster]
```

Reloads PostgreSQL configuration without restarting.

#### Check Status

```bash
dbctl status          # Shows all clusters
dbctl status mydev    # Shows specific cluster
```

#### List Everything

```bash
dbctl ls              # Lists all clusters
dbctl list            # Same as ls
```

#### Remove a Cluster

```bash
dbctl remove <cluster>
dbctl rm <cluster>         # Short alias
```

Prompts for confirmation before deleting data. Use `--force` to skip.

#### View Logs

```bash
dbctl logs [cluster] [--lines 50] [--follow]
dbctl logs mydev -n 100 -f
```

### Database Operations

#### Create / Drop Database

```bash
dbctl createdb <name> [-c cluster]
dbctl dropdb <name> [-c cluster]
```

Examples:
```bash
dbctl createdb myapp           # Auto-selects cluster, auto-starts if stopped
dbctl createdb proddb -c prod  # Explicit cluster
dbctl dropdb olddb             # Prompts for confirmation
dbctl dropdb testdb --force    # Skip confirmation
```

#### List Databases

```bash
dbctl databases [-c cluster]
```

Without a cluster argument, shows databases for all clusters.

#### Interactive SQL REPL

```bash
dbctl use <database> [-c cluster]
```

Opens a built-in SQL REPL connected to the specified database. Supports multi-line
statements, command history, and psql-style backslash commands (`\dt`, `\d table`, `\l`, `\q`).

Examples:
```bash
dbctl use myapp                # Start REPL for myapp database
dbctl use myapp -c production  # Explicit cluster
```

#### Open Interactive Shell (psql)

```bash
dbctl shell [database] [-c cluster]
```

Opens the native `psql` shell (requires PostgreSQL binaries).

Examples:
```bash
dbctl shell myapp              # Opens psql to myapp database
dbctl shell                    # Opens psql to postgres database
dbctl shell myapp -c dev       # Explicit cluster
```

#### Execute SQL

```bash
dbctl exec --sql "<SQL>" -d <database> [-c cluster]
dbctl exec --file <file.sql> -d <database> [-c cluster]
```

Examples:
```bash
dbctl exec --sql "SELECT * FROM users" -d myapp
dbctl exec --file schema.sql -d myapp
```

#### Backup / Restore Database

```bash
dbctl backup <database> -o <file> [-c cluster]
dbctl restore <database> -i <file> [-c cluster]
```

Examples:
```bash
dbctl backup myapp -o backup.sql
dbctl restore myapp -i backup.sql
```

### User Management

#### Create User

```bash
dbctl createuser <username> [-p password] [--superuser] [-c cluster]
```

If password is not provided, prompts securely:
```bash
dbctl createuser alice           # Prompts for password
dbctl createuser bob --superuser # Create superuser
```

#### List Users

```bash
dbctl users [-c cluster]
```

#### Note on Privileges

For advanced privilege management (grant/revoke), use the nested commands:
```bash
dbctl db grant --cluster dev --database myapp --user alice --on database --privileges CONNECT
dbctl db privileges --cluster dev --database myapp --user alice
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
- [just](https://github.com/casey/just) (optional, for running common tasks)

No external tools required for runtime - all archive extraction is handled natively in Rust.

Install just:
```bash
cargo install just
```

## Development

The project includes a Justfile for common development tasks:

```bash
just                     # List all available commands
just build               # Build in debug mode
just build-release       # Build in release mode
just test                # Run all tests
just test-integration    # Run integration tests
just test-one <name>     # Run a specific test
just clippy              # Run linter
just fmt                 # Format code
just ci                  # Run all CI checks
just clean               # Clean build artifacts
just stats               # Show project statistics
```

## Testing

The project includes a comprehensive integration test suite that verifies all commands work correctly.

Run all tests:

```bash
just test-integration
# or
cargo test --test integration_tests -- --test-threads=1
```

Run a specific test:

```bash
just test-one test_server_install_and_list
# or
cargo test --test integration_tests test_server_install_and_list
```

The test suite covers:
- Server installation and version management
- Cluster initialization, start, stop, restart, and removal
- Database creation and deletion
- Error handling for invalid operations
- Custom configurations (ports, usernames)

Note: Tests run sequentially (`--test-threads=1`) to avoid port conflicts and may take several minutes to complete as they download PostgreSQL binaries and perform actual database operations.

## Migration from Old Commands

The old nested command structure still works but is hidden from help:

```bash
# Old style (still works)
dbctl server install --version 16.3
dbctl server init --version 16.3 --name dev --port 5432
dbctl server start --name dev
dbctl db create --cluster dev --database myapp

# New style (recommended)
dbctl install 16.3
dbctl init dev --version 16.3 --port 5432
dbctl start           # Auto-selects
dbctl createdb myapp  # Auto-selects, auto-starts
```

## Tips

- **First time**: Run `dbctl` with no arguments to see what's available
- **Multiple clusters**: The tool remembers your last-used cluster, so you don't need to specify it each time
- **Auto-start**: Database commands automatically start stopped clusters - no need to remember to start first
- **Interactive**: When in doubt, omit arguments and the tool will prompt you
- **Confirmation**: Destructive operations (drop, remove) require confirmation unless you use `--force`

## License

Apache 2.0
