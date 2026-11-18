# Security

## Reporting a problem

Please report a vulnerability privately, through this repository's GitHub
"Report a vulnerability" form (Security tab), and not in a public issue. Include
the dbctl version (`dbctl --version`), your OS, and the steps to reproduce.

## What dbctl is trying to protect

dbctl installs PostgreSQL and runs clusters as your own user, under
`~/.dbctl/`, and is often driven by scripts and automation. The threats it
designs against:

- **Tampered or broken downloads.** On macOS every PostgreSQL archive,
  downloaded or cached, is checked against a SHA-256 pinned in the source
  before it is unpacked, and only pinned releases install. A download is kept
  only when its hash matches, and an install that fails leaves nothing behind
  that looks installed. The default pgvector source is pinned to a commit.
  On Linux dbctl downloads nothing: it uses the distribution's packages.
- **Names that escape `~/.dbctl`.** Cluster names, versions and git tags are
  joined onto paths that dbctl creates and deletes, so each must be one plain
  path component (letters, digits, `.`, `_`, `-`).
- **SQL injection through names.** Database, role and extension names reach
  SQL only as quoted identifiers, quoted literals or bound parameters.
- **Other local users.** Each cluster's unix socket is owner-only (0700), and
  `dumpall` output, which holds every role's password hash, is created 0600.
  `create-user --password-stdin` keeps a password out of the process list.

## What it does not protect against

- **Trust mode and other local users.** `dbctl init` defaults to
  `--auth trust`, under which any user on the machine can connect over
  127.0.0.1 as the superuser and run commands as you. That is meant for a
  single-user laptop. On a shared machine or a server, use `--auth password`.
- **Anything already running as you.** A process with your account can read
  `~/.dbctl`, connect over the socket, and run dbctl itself.
- **`create-user --password`.** A password passed as an argument is visible in
  the process list and kept in shell history. Use `--password-stdin` or the
  prompt instead.
- **A source you choose.** `extension install --git-tag` or `--source` builds
  whatever that ref or directory contains, without a pinned commit.
