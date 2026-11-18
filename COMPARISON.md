# dbctl vs pgenv

How dbctl compares with [pgenv](https://github.com/theory/pgenv), a PostgreSQL binary manager in the style of rbenv/plenv.

## At a glance

| | dbctl | pgenv |
|---|---|---|
| Implementation | Rust (single binary, tokio async) | Bash script (rbenv/plenv-style) |
| How versions are obtained | Prebuilt EnterpriseDB binaries on macOS; the distribution's packages on Linux | Compiles PostgreSQL from source |
| Version coverage | Curated list of recent majors (`AVAILABLE_VERSIONS` in `server.rs`) | Practically anything, back to PostgreSQL 8.0 |
| Install speed | Fast (archive download + extract, plus local binary cache) | Slow (full source build; needs compiler/deps) |
| Version switching | N/A - each cluster pins its own version; binaries addressed per install | Central concept: `pgenv use/switch` changes the active version |
| Data model | Multiple named clusters under `~/.dbctl/clusters/`, each with `metadata.json` | One data directory per built version |
| Scope | Full workflow: versions, clusters, databases, users, backups/restore, migrations, extensions | Build/switch/run versions; lifecycle (start/stop/status) of the active instance |

## Where pgenv is stronger

- **Version breadth.** Because it builds from source, pgenv can install ancient releases (8.0+) and arbitrary minor/patch versions. dbctl is limited to whatever prebuilt archives are listed in `AVAILABLE_VERSIONS`.
- **Build customization.** pgenv exposes configure flags and build config (`pgenv config`, `pgenv check` for dependencies), useful for testing PostgreSQL itself or unusual build options.
- **Maturity and simplicity.** It's a small, dependency-free Bash script that's been around since 2018.

## Where dbctl is stronger

- **Speed to a running database.** Prebuilt binaries + the local archive cache (`dbctl cache stats/clear`) mean installs take seconds, not a source compile.
- **Multi-cluster management.** pgenv assumes one active version/data dir at a time; dbctl manages many isolated named clusters side by side, with auto-selection when only one exists and auto-start of stopped clusters.
- **Higher-level database operations.** createdb/dropdb, user management, SQL exec, backup/restore, and migrations are built in. pgenv deliberately stops at the server level.
- **Extensions.** dbctl builds and enables extensions via PGXS (e.g. pgvector) against the managed install.
- **UX for local dev.** Interactive prompts (`dialoguer`), colored output, progress bars, `--json` flags, and remembering your last-used cluster in `context.json`.

## Bottom line

They overlap only at the "install and run multiple PostgreSQL versions without root" layer. pgenv is a **version switcher** aimed at compatibility/build testing (build anything from source, swap the active version). dbctl is a **local development environment manager** - fewer, newer versions, but a much broader workflow around them (clusters, databases, backups, users, extensions) with a focus on convenience and speed.
