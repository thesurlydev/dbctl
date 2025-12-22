# dbctl-gui MVP Feature Roadmap

## Current Features (Implemented)

### Core CLI
- [x] Cluster management (start, stop, restart, remove)
- [x] Cluster creation with version selection and auto-port detection
- [x] PostgreSQL installation (multiple versions)
- [x] Database CRUD operations
- [x] User management with privileges
- [x] SQL query execution
- [x] Log viewing
- [x] Backup/restore (pg_dump/psql)
- [x] Grant/revoke privileges on databases, schemas, tables

---

## Phase 1: Core Improvements (Implemented)

### Table Browser
- [x] Visual table explorer with schema tree view
- [x] Collapsible schema navigation
- [x] View table structure (columns, types, constraints, indexes)
- [x] Browse table data with pagination (configurable page size)
- [x] Insert new rows via dialog
- [x] Delete rows with confirmation
- [x] Update cells inline
- [x] Export table data to CSV/JSON

### Query Enhancements
- [x] Query history with recall (50 queries, stored in ~/.dbctl/query_history.json)
- [x] Save/load queries to files (~/.dbctl/queries/*.sql)
- [x] Multiple query tabs (up to 10)
- [x] Syntax highlighting (keywords, types, strings, numbers, comments)
- [x] Query execution plan visualization (EXPLAIN/EXPLAIN ANALYZE buttons)
- [x] Keyboard shortcuts (Cmd+Enter to execute, Cmd+S to save, Cmd+Up/Down for history)

### Connection Management
- [x] Connection string builder (auto-generated, copy to clipboard)
- [x] Save connection profiles (persisted to ~/.dbctl/connections.json)
- [x] Connect to remote PostgreSQL servers (add/edit/delete connections)
- [x] SSL/TLS configuration (disable, require modes)
- [x] Connection testing with version display

---

## Phase 2: Developer Experience (Implemented)

### Schema Designer
- [x] Visual canvas-based schema editor
- [x] Draggable table positioning
- [x] Create/alter/drop tables via UI
- [x] Add/edit/rename/drop columns
- [x] Index management (create, drop)
- [x] Foreign key visualization (bezier curves between tables)
- [x] Constraint management (PK, FK, UNIQUE)
- [x] Generate migration SQL from schema changes (live preview)
- [x] Apply migrations with confirmation dialog
- [x] Undo/redo support with action history
- [x] Keyboard shortcuts:
  - Cmd+N: New table
  - Cmd+S: Apply migration
  - Cmd+D: Duplicate table
  - F2: Rename table/column
  - Cmd+Z / Cmd+Shift+Z: Undo/Redo
  - Delete/Backspace: Delete selected
  - Escape: Cancel operation
- [x] Auto-layout for tables
- [x] Visual column reordering (drag-and-drop)
- [x] Semantic default column ordering (PK first, FK second, timestamps last)
- [x] Formatted column types (varchar(N), numeric(p,s))

### Data Export (Partial)
- [x] Export table data to CSV
- [x] Export table data to JSON
- [x] Backup database to SQL file
- [x] Restore database from SQL file
- [ ] Import CSV/JSON to tables
- [ ] Export query results
- [ ] pg_dump format selection (plain, custom, directory)
- [ ] Scheduled backups

### Monitoring (Basic)
- [x] Cluster statistics (database count, user count, table count)
- [x] Table row count estimates
- [x] Table size estimates
- [ ] Active connections display
- [ ] Query performance stats (pg_stat_statements)
- [ ] Index usage statistics
- [ ] Vacuum/analyze status

---

## Phase 3: Advanced Features (Not Started)

### Extension Manager
- [ ] Browse available extensions
- [ ] Install/uninstall extensions
- [ ] Extension-specific UI (PostGIS map view, pgvector similarity search)

### Role & Permission Manager
- [ ] Visual role hierarchy
- [ ] Grant/revoke permissions via UI
- [ ] Row-level security policy editor
- [ ] Audit role permissions

### Query Builder
- [ ] Visual query builder for non-SQL users
- [ ] JOIN visualization
- [ ] Drag-and-drop table selection

### Data Generation
- [ ] Generate test data for tables
- [ ] Faker integration for realistic data
- [ ] Seed scripts management

---

## Phase 4: Team & Workflow (Not Started)

### Project Workspaces
- [ ] Save cluster + database + query sets as projects
- [ ] Share project configurations

### SQL Formatting
- [ ] Auto-format SQL queries
- [ ] Configurable formatting rules

### Diff Tool
- [ ] Compare schemas between databases
- [ ] Compare data between tables
- [ ] Generate sync scripts

### Notifications
- [ ] Cluster start/stop notifications
- [ ] Query completion alerts
- [ ] Error alerts

---

## Technical Improvements

### Performance
- [x] Paginated data loading
- [ ] Lazy loading for large datasets
- [ ] Query result streaming
- [ ] Background refresh for stats

### UI/UX
- [x] Monospace fonts for code/data display
- [x] Loading states for async operations
- [x] Status messages and error display
- [ ] Dark/light theme toggle
- [ ] Configurable keyboard shortcuts
- [ ] Resizable panels with memory
- [ ] Full-text search across all objects

### Platform
- [ ] System tray integration
- [ ] Auto-start clusters on app launch
- [ ] Native file dialogs for backup/restore paths
- [ ] Drag-and-drop SQL file execution

---

## Out of Scope for MVP

- Multi-user collaboration
- Cloud database support (RDS, Cloud SQL, etc.)
- BI/charting features
- Scheduled job execution
- Full pgAdmin feature parity
