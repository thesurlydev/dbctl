# dbctl-gui MVP Feature Roadmap

## Current Features (Implemented)

- Cluster management (start, stop, restart, remove)
- Cluster creation with version selection
- PostgreSQL installation
- Database CRUD operations
- User management
- SQL query execution
- Log viewing
- Backup/restore

---

## Phase 1: Core Improvements

### Table Browser (Implemented)
- [x] Visual table explorer with schema tree view
- [x] View table structure (columns, types, constraints)
- [x] Browse table data with pagination
- [x] Edit data inline (INSERT, UPDATE, DELETE)
- [x] Export table data to CSV/JSON

### Query Enhancements (Implemented)
- [x] Query history with recall (50 queries, stored in ~/.dbctl/query_history.json)
- [x] Save/load queries to files (~/.dbctl/queries/*.sql)
- [x] Multiple query tabs (up to 10)
- [x] Syntax highlighting (keywords, types, strings, numbers, comments)
- [x] Query execution plan visualization (EXPLAIN/EXPLAIN ANALYZE buttons)
- [x] Keyboard shortcuts (Cmd+Enter to execute, Cmd+S to save, Cmd+Up/Down for history)

### Connection Management
- Connection string builder
- Save connection profiles
- Connect to remote PostgreSQL servers (not just local clusters)
- SSL/TLS configuration

---

## Phase 2: Developer Experience

### Schema Management
- Visual schema designer
- Create/alter tables via UI
- Index management (create, drop, view usage stats)
- Foreign key visualization
- Generate migration SQL from schema changes

### Data Import/Export
- Import CSV/JSON to tables
- Export query results
- pg_dump format selection (plain, custom, directory)
- Scheduled backups

### Monitoring Dashboard
- Active connections display
- Query performance stats (pg_stat_statements)
- Table size analytics
- Index usage statistics
- Vacuum/analyze status
- Replication lag (if applicable)

---

## Phase 3: Advanced Features

### Extension Manager
- Browse available extensions
- Install/uninstall extensions
- Extension-specific UI (PostGIS map view, pgvector similarity search)

### Role & Permission Manager
- Visual role hierarchy
- Grant/revoke permissions via UI
- Row-level security policy editor
- Audit role permissions

### Query Builder
- Visual query builder for non-SQL users
- JOIN visualization
- Drag-and-drop table selection

### Data Generation
- Generate test data for tables
- Faker integration for realistic data
- Seed scripts management

---

## Phase 4: Team & Workflow

### Project Workspaces
- Save cluster + database + query sets as projects
- Share project configurations

### SQL Formatting
- Auto-format SQL queries
- Configurable formatting rules

### Diff Tool
- Compare schemas between databases
- Compare data between tables
- Generate sync scripts

### Notifications
- Cluster start/stop notifications
- Query completion alerts
- Error alerts

---

## Technical Improvements

### Performance
- Lazy loading for large datasets
- Query result streaming
- Background refresh for stats

### UI/UX
- Dark/light theme toggle
- Configurable keyboard shortcuts
- Resizable panels with memory
- Full-text search across all objects

### Platform
- System tray integration
- Auto-start clusters on app launch
- Native file dialogs for backup/restore paths
- Drag-and-drop SQL file execution

---

## Out of Scope for MVP

- Multi-user collaboration
- Cloud database support (RDS, Cloud SQL, etc.)
- BI/charting features
- Scheduled job execution
- Full pgAdmin feature parity
