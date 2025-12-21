# Quick Start Guide

This guide walks you through the main features of dbctl with practical examples.

## Initial Setup

### Install PostgreSQL

First, install a PostgreSQL version:

```bash
dbctl install 16.3
```

You can install multiple versions side-by-side:

```bash
dbctl install 15.4
dbctl install 17.0
```

### Create Your First Cluster

Create a cluster interactively (prompts for missing options):

```bash
dbctl init mydev
```

Or specify everything explicitly:

```bash
dbctl init mydev --version 16.3 --port 5432 --username postgres
```

### Check Status

View all clusters and their status:

```bash
dbctl
# or
dbctl status
```

## Working with Clusters

### Start, Stop, and Restart

```bash
# Start a cluster (auto-selected if only one exists)
dbctl start

# Stop it
dbctl stop

# Restart it
dbctl restart

# Reload configuration without restarting
dbctl reload
```

### View Logs

```bash
# View last 50 lines (default)
dbctl logs

# View last 100 lines
dbctl logs -n 100

# Follow logs in real-time
dbctl logs -f

# Combine options
dbctl logs mydev -n 200 -f
```

### Multiple Clusters

Create additional clusters on different ports:

```bash
dbctl init production --version 16.3 --port 5433 --username admin
dbctl init testing --version 15.4 --port 5434 --username postgres
```

List all clusters:

```bash
dbctl ls
```

Switch between clusters:

```bash
dbctl start production
dbctl createdb prodapp    # Uses 'production' cluster

dbctl start testing
dbctl createdb testapp    # Uses 'testing' cluster
```

## Database Operations

### Create and Drop Databases

```bash
# Create a database (auto-starts cluster if stopped)
dbctl createdb myapp

# Create in a specific cluster
dbctl createdb proddb -c production

# Drop a database (prompts for confirmation)
dbctl dropdb olddb

# Force drop without confirmation
dbctl dropdb testdb --force
```

### List Databases

```bash
# List databases in auto-selected cluster
dbctl databases

# List databases in specific cluster
dbctl databases -c production

# List all databases in all clusters
dbctl ls
```

### Interactive SQL Shell

```bash
# Open shell to a database
dbctl shell myapp

# Open shell to postgres database (default)
dbctl shell

# Explicit cluster
dbctl shell myapp -c production
```

### Execute SQL Commands

```bash
# Execute inline SQL
dbctl exec --sql "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT)" -d myapp

# Execute SQL from a file
dbctl exec --file schema.sql -d myapp

# Query data
dbctl exec --sql "SELECT * FROM users" -d myapp
```

### Backup and Restore

```bash
# Backup a database
dbctl backup myapp -o myapp_backup.sql

# Backup with specific cluster
dbctl backup proddb -o prod_backup.sql -c production

# Restore a database
dbctl restore myapp -i myapp_backup.sql

# Restore to different cluster
dbctl restore testapp -i prod_backup.sql -c testing
```

## User Management

### Create Users

```bash
# Create user (prompts for password)
dbctl createuser alice

# Create user with password
dbctl createuser bob -p secretpass

# Create superuser
dbctl createuser admin --superuser
```

### List Users

```bash
# List users in auto-selected cluster
dbctl users

# List users in specific cluster
dbctl users -c production
```

### Manage Privileges

Grant database privileges:

```bash
# Grant CONNECT privilege on database
dbctl db grant --cluster mydev --database myapp --user alice --on database --privileges CONNECT

# Grant CREATE privilege on database
dbctl db grant --cluster mydev --database myapp --user alice --on database --privileges CREATE
```

Grant schema privileges:

```bash
# Grant USAGE on schema
dbctl db grant --cluster mydev --database myapp --user alice --on schema --schema public --privileges USAGE

# Grant CREATE on schema
dbctl db grant --cluster mydev --database myapp --user alice --on schema --schema public --privileges CREATE
```

Grant table privileges:

```bash
# Grant SELECT on a specific table
dbctl db grant --cluster mydev --database myapp --user alice --on table --table users --privileges SELECT

# Grant multiple privileges
dbctl db grant --cluster mydev --database myapp --user alice --on table --table users --privileges "SELECT, INSERT, UPDATE"
```

View user privileges:

```bash
# View all privileges for a user
dbctl db privileges --cluster mydev --database myapp --user alice
```

Revoke privileges:

```bash
# Revoke table privileges
dbctl db revoke --cluster mydev --database myapp --user alice --on table --table users --privileges INSERT

# Revoke schema privileges
dbctl db revoke --cluster mydev --database myapp --user alice --on schema --schema public --privileges CREATE
```

## Complete Workflow Example

Here's a complete workflow from installation to working with databases:

```bash
# 1. Install PostgreSQL
dbctl install 16.3

# 2. Create and start a cluster
dbctl init mydev --version 16.3 --port 5432 --username postgres

# 3. Create a database
dbctl createdb myapp

# 4. Create a schema
dbctl exec --sql "
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT UNIQUE NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
)" -d myapp

# 5. Insert some data
dbctl exec --sql "
INSERT INTO users (name, email) VALUES
    ('Alice', 'alice@example.com'),
    ('Bob', 'bob@example.com')
" -d myapp

# 6. Query the data
dbctl exec --sql "SELECT * FROM users" -d myapp

# 7. Create a user with limited privileges
dbctl createuser readonly -p readonly123

# 8. Grant SELECT privilege
dbctl db grant --cluster mydev --database myapp --user readonly --on database --privileges CONNECT
dbctl db grant --cluster mydev --database myapp --user readonly --on schema --schema public --privileges USAGE
dbctl db grant --cluster mydev --database myapp --user readonly --on table --table users --privileges SELECT

# 9. Backup the database
dbctl backup myapp -o myapp_backup.sql

# 10. Open an interactive shell
dbctl shell myapp
```

## Cluster Management

### Remove a Cluster

```bash
# Remove with confirmation prompt
dbctl remove mydev

# Force remove without confirmation
dbctl rm mydev --force
```

### Working with Different PostgreSQL Versions

```bash
# Create clusters with different versions
dbctl init dev16 --version 16.3 --port 5432
dbctl init dev15 --version 15.4 --port 5433

# Switch between them
dbctl start dev16
dbctl createdb myapp    # Created in PostgreSQL 16.3

dbctl start dev15
dbctl createdb myapp    # Created in PostgreSQL 15.4
```

## Tips and Tricks

### Auto-Selection

When you have only one cluster, you never need to specify it:

```bash
dbctl start
dbctl createdb myapp
dbctl shell myapp
dbctl users
```

### Last-Used Cluster

With multiple clusters, dbctl remembers your last-used cluster:

```bash
dbctl start production     # Set production as active
dbctl createdb proddb      # Uses production
dbctl backup proddb -o backup.sql  # Uses production
dbctl shell proddb         # Uses production

# Switch to different cluster
dbctl start dev            # Set dev as active
dbctl createdb testdb      # Uses dev
```

### Override Auto-Selection

Use `-c` or `--cluster` to override:

```bash
# Even if 'dev' is last-used, explicitly use 'production'
dbctl createdb newdb -c production
dbctl shell myapp -c production
```

### Auto-Start

Database commands automatically start stopped clusters:

```bash
dbctl stop                 # Stop the cluster
dbctl createdb myapp       # Automatically starts it first
dbctl shell myapp          # Automatically starts if stopped
```

### Check Before You Act

Always check status before making changes:

```bash
dbctl                      # Shows all clusters
dbctl databases            # Shows all databases
dbctl users                # Shows all users
```

## Connecting External Tools

You can use any PostgreSQL client with your dbctl clusters:

```bash
# Start your cluster
dbctl start mydev

# Check the port and credentials
dbctl status mydev

# Connect with psql
psql -h localhost -p 5432 -U postgres -d myapp

# Connect with connection string
psql "postgresql://postgres@localhost:5432/myapp"

# Use in application configuration
DATABASE_URL=postgresql://postgres@localhost:5432/myapp
```

## Next Steps

- Check out the full command reference in the [README](README.md)
- Run `dbctl --help` for command-line help
- Run `dbctl <command> --help` for help on specific commands
