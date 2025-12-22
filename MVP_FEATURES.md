# dbctl-gui Feature Roadmap

## Vision

**TablePlus's beauty + DataGrip's power + Git-native + AI-assisted design**

A PostgreSQL-first schema tool that's fast, keyboard-driven, and understands your workflow.

---

## Implemented

### Core CLI
- [x] Cluster management (start, stop, restart, remove)
- [x] Cluster creation with version selection and auto-port detection
- [x] PostgreSQL installation (multiple versions)
- [x] Database CRUD operations
- [x] User management with privileges
- [x] Backup/restore (pg_dump/psql)

### Table Browser
- [x] Schema tree navigation with collapsible sections
- [x] Table structure view (columns, types, constraints, indexes)
- [x] Paginated data browsing
- [x] Insert/update/delete rows
- [x] Export to CSV/JSON

### Query Editor
- [x] Multiple tabs (up to 10)
- [x] Syntax highlighting
- [x] Query history (50 entries, persisted)
- [x] Save/load queries
- [x] EXPLAIN / EXPLAIN ANALYZE
- [x] Keyboard shortcuts (Cmd+Enter, Cmd+S, Cmd+Up/Down)

### Connection Management
- [x] Local clusters and remote servers
- [x] SSL/TLS support (disable, require)
- [x] Connection profiles (persisted)
- [x] Connection string display with copy

### Schema Designer
- [x] Canvas-based visual editor
- [x] Draggable table positioning
- [x] Create/alter/drop tables, columns, indexes
- [x] Foreign key visualization (bezier curves)
- [x] Live migration SQL preview
- [x] Undo/redo with action history
- [x] Auto-layout
- [x] Semantic column ordering (PK, FK, regular, timestamps)
- [x] Formatted column types (varchar(N), numeric(p,s))
- [x] Keyboard shortcuts (Cmd+N, Cmd+S, Cmd+D, F2, Cmd+Z)

---

## Phase 1: Developer Experience Polish

*Goal: Make it feel fast, professional, and keyboard-native*

### Command Palette (High Impact)
- [ ] Cmd+K to open command palette
- [ ] Fuzzy search all actions, tables, databases
- [ ] Recent commands history
- [ ] Context-aware suggestions

### Dark Mode
- [ ] System preference detection
- [ ] Manual toggle
- [ ] Consistent theming across all components

### Keyboard Navigation
- [ ] Vim-style navigation in table browser (j/k/h/l)
- [ ] Tab/Shift+Tab through form fields
- [ ] Escape to close dialogs/cancel
- [ ] Global shortcuts reference panel (Cmd+?)

### Connection Safety
- [ ] Color-coded connection tabs (red for production)
- [ ] Environment labels (dev/staging/prod)
- [ ] Confirmation dialogs for destructive operations on prod

### Performance
- [ ] Lazy loading for large schemas
- [ ] Virtual scrolling for table data
- [ ] Background schema refresh

---

## Phase 2: Migration Intelligence

*Goal: Make schema changes safe and predictable*

### Migration Safety (Key Differentiator)
- [ ] Lock time estimation before applying changes
- [ ] Affected queries preview (which queries touch changed columns)
- [ ] Dependency graph visualization
- [ ] Auto-generated rollback scripts
- [ ] "Safe mode" that uses shadow table approach for large tables

### Schema Diff
- [ ] Side-by-side visual comparison
- [ ] Color-coded additions/deletions/modifications
- [ ] Compare: working vs database, database vs database
- [ ] Export diff as migration SQL

### Migration History
- [ ] Track applied migrations per database
- [ ] Migration timeline view
- [ ] Rollback to previous state

---

## Phase 3: Git-Native Workflow

*Goal: Schema versioning that feels like code versioning*

### Git Integration (Key Differentiator)
- [ ] Save schema as SQL/YAML files
- [ ] Initialize git repo for schema
- [ ] Commit schema changes with messages
- [ ] Branch support for feature schemas
- [ ] Visual schema diff between branches

### Schema Snapshots
- [ ] Save named snapshots locally
- [ ] Compare current vs snapshot
- [ ] Restore from snapshot

### Export Formats
- [ ] Raw SQL (CREATE TABLE statements)
- [ ] Atlas HCL format
- [ ] Prisma schema format
- [ ] YAML representation

---

## Phase 4: Performance-Aware Design

*Goal: Catch performance issues before deployment*

### Index Advisor (Key Differentiator)
- [ ] Suggest indexes based on table structure
- [ ] "What if" analysis: preview query plan with proposed index
- [ ] Detect missing foreign key indexes
- [ ] Identify unused indexes (connect to pg_stat_user_indexes)

### Query Analysis
- [ ] Import slow queries from pg_stat_statements
- [ ] Suggest schema changes to improve specific queries
- [ ] Visual EXPLAIN with cost breakdown

### Table Health
- [ ] Table bloat detection
- [ ] Suggest VACUUM/ANALYZE
- [ ] Partition recommendations for large tables
- [ ] TOAST column warnings

---

## Phase 5: AI-Assisted Design

*Goal: Natural language to schema, with intelligence*

### Schema Generation (Key Differentiator)
- [ ] Describe requirements in natural language
- [ ] Generate tables, relationships, constraints
- [ ] Iterative refinement through conversation

### Schema Review
- [ ] Anti-pattern detection (EAV, missing indexes, etc.)
- [ ] Normalization analysis (1NF/2NF/3NF violations)
- [ ] Security suggestions (missing RLS, exposed PII columns)

### Documentation
- [ ] Auto-generate table/column descriptions
- [ ] Explain complex queries in plain English
- [ ] Generate example queries for tables

---

## Phase 6: Postgres-Native Features

*Goal: First-class support for advanced PostgreSQL features*

### Advanced Types
- [ ] JSONB schema designer with path visualization
- [ ] Array column support
- [ ] Enum type management
- [ ] Composite type editor
- [ ] Domain type support

### Security
- [ ] Row Level Security (RLS) policy builder
- [ ] Visual policy editor
- [ ] Test policies with sample users

### Functions & Triggers
- [ ] PL/pgSQL editor with syntax highlighting
- [ ] Trigger management UI
- [ ] Function testing interface

### Extensions
- [ ] Browse available extensions
- [ ] One-click install/uninstall
- [ ] Extension-specific UI (PostGIS preview, pgvector)

---

## Phase 7: Code Generation

*Goal: Schema to code, instantly*

### ORM Models
- [ ] Rust (SQLx structs, Diesel models)
- [ ] Python (SQLAlchemy models)
- [ ] TypeScript (Prisma schema, Drizzle)

### Type Definitions
- [ ] TypeScript interfaces
- [ ] Rust structs
- [ ] JSON Schema

### API Stubs
- [ ] REST endpoint templates
- [ ] GraphQL schema

---

## Out of Scope

- Multi-user real-time collaboration
- Cloud database providers (RDS, Cloud SQL) - use connection strings
- BI/charting/dashboards
- Scheduled job execution
- Full pgAdmin feature parity

---

## Priority Matrix

| Feature | Impact | Effort | Priority |
|---------|--------|--------|----------|
| Command palette | High | Low | P0 |
| Dark mode | Medium | Low | P0 |
| Connection safety (color coding) | High | Low | P0 |
| Migration lock time estimation | High | Medium | P1 |
| Schema diff visualization | High | Medium | P1 |
| Index advisor | High | Medium | P1 |
| Git integration | High | High | P2 |
| AI schema generation | High | High | P2 |
| Anti-pattern detection | Medium | Medium | P2 |
| JSONB designer | Medium | Medium | P3 |
| RLS policy builder | Medium | High | P3 |
| Code generation | Medium | Medium | P3 |

---

## Competitive Positioning

| vs Tool | Our Advantage |
|---------|---------------|
| pgAdmin | Native speed, modern UI, keyboard-first |
| DBeaver | Postgres-optimized, no Java bloat |
| TablePlus | Migration intelligence, Git workflow, AI assist |
| DataGrip | Free, lightweight, schema-focused |
| Navicat | Modern aesthetics, developer workflow |
