//! End-to-end tests of the dbctl binary.
//!
//! Every test runs the real binary (`CARGO_BIN_EXE_dbctl`) with HOME pointed
//! at a scratch directory, so nothing here can touch the developer's
//! `~/.dbctl`, and with stdin closed, the way automation and scripts run it:
//! no command may wait on a prompt.
//!
//! Tests that need PostgreSQL are ignored by default. They install it once per
//! run into a shared scratch HOME (reusing the developer's download cache when
//! there is one, otherwise downloading from EnterpriseDB) and give each test
//! its own cluster and port. Run them with:
//!
//! ```text
//! cargo test -p dbctl --test cli -- --ignored
//! ```

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

const PG_VERSION: &str = "16.3";

fn dbctl(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dbctl"))
        .args(args)
        .env("HOME", home)
        .stdin(Stdio::null())
        .output()
        .expect("failed to run dbctl")
}

fn describe(args: &[&str], out: &Output) -> String {
    format!(
        "dbctl {}\n--- status: {}\n--- stdout:\n{}\n--- stderr:\n{}",
        args.join(" "),
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// Run dbctl, require success, return stdout.
fn ok(home: &Path, args: &[&str]) -> String {
    let out = dbctl(home, args);
    assert!(
        out.status.success(),
        "expected success: {}",
        describe(args, &out)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Run dbctl, require failure, return stderr.
fn fails(home: &Path, args: &[&str]) -> String {
    let out = dbctl(home, args);
    assert!(
        !out.status.success(),
        "expected failure: {}",
        describe(args, &out)
    );
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn json(home: &Path, args: &[&str]) -> serde_json::Value {
    let stdout = ok(home, args);
    serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "dbctl {} printed invalid JSON ({e}): {stdout}",
            args.join(" ")
        )
    })
}

/// An empty HOME for one test, removed when the guard drops.
struct ScratchHome(PathBuf);

impl ScratchHome {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("dbctl-cli-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create scratch HOME");
        Self(dir)
    }
}

impl Drop for ScratchHome {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// ---------------------------------------------------------------------------
// No PostgreSQL needed
// ---------------------------------------------------------------------------

#[test]
fn versions_json_reports_available_and_installed() {
    let home = ScratchHome::new("versions");
    let report = json(&home.0, &["versions", "--json"]);
    assert!(
        !report["available"]
            .as_array()
            .expect("available array")
            .is_empty(),
        "{report}"
    );
    assert_eq!(report["installed"], serde_json::json!([]));
}

#[test]
fn list_json_is_an_empty_array_without_clusters() {
    let home = ScratchHome::new("list-empty");
    assert_eq!(json(&home.0, &["list", "--json"]), serde_json::json!([]));
}

#[test]
fn json_reads_name_their_cluster() {
    let home = ScratchHome::new("json-requires");
    assert!(fails(&home.0, &["status", "--json"]).contains("<NAME>"));
    assert!(fails(&home.0, &["databases", "--json"]).contains("--cluster"));
}

#[test]
fn starting_an_unknown_cluster_fails() {
    let home = ScratchHome::new("start-unknown");
    assert!(fails(&home.0, &["start", "nope"]).contains("does not exist"));
}

#[test]
fn init_without_a_name_fails_instead_of_prompting() {
    let home = ScratchHome::new("init-no-name");
    let stderr = fails(
        &home.0,
        &["init", "--version", PG_VERSION, "--port", "25590"],
    );
    assert!(stderr.contains("cluster name is required"), "{stderr}");
}

#[test]
fn init_with_an_uninstalled_version_says_how_to_install_it() {
    let home = ScratchHome::new("init-uninstalled");
    let stderr = fails(
        &home.0,
        &["init", "x", "--version", PG_VERSION, "--port", "25591"],
    );
    assert!(
        stderr.contains(&format!("Run: dbctl install {PG_VERSION}")),
        "{stderr}"
    );
}

// ---------------------------------------------------------------------------
// PostgreSQL installed once per run
// ---------------------------------------------------------------------------

/// A HOME with PostgreSQL installed, shared by every test below. It lives at a
/// fixed path so later runs reuse the unpacked install.
fn pg_home() -> &'static Path {
    static HOME: OnceLock<PathBuf> = OnceLock::new();
    HOME.get_or_init(|| {
        let home = std::env::temp_dir().join("dbctl-cli-pg");
        seed_download_cache(&home);
        ok(&home, &["install", PG_VERSION]);
        home
    })
}

/// Link the developer's cached PostgreSQL archives into `home`, read-only, so
/// the install does not download hundreds of megabytes on every machine
/// that already has them.
fn seed_download_cache(home: &Path) {
    let cache = home.join(".dbctl/cache/postgres");
    std::fs::create_dir_all(&cache).expect("create cache dir");
    let Some(real_home) = std::env::var_os("HOME") else {
        return;
    };
    let real_cache = PathBuf::from(real_home).join(".dbctl/cache/postgres");
    let Ok(entries) = std::fs::read_dir(&real_cache) else {
        return;
    };
    for entry in entries.flatten() {
        let link = cache.join(entry.file_name());
        if !link.exists() {
            #[cfg(unix)]
            let _ = std::os::unix::fs::symlink(entry.path(), &link);
        }
    }
}

/// A cluster in the shared HOME, removed (stopping it first) when dropped, so
/// a failing test never leaves PostgreSQL running.
struct Cluster {
    name: String,
    port: u16,
}

impl Cluster {
    /// Each test passes its own port so tests can run in parallel.
    fn init(name: &str, port: u16) -> Self {
        let home = pg_home();
        // A previous run that was killed may have left this cluster behind.
        let _ = dbctl(home, &["remove", name, "--force"]);
        ok(
            home,
            &[
                "init",
                name,
                "--version",
                PG_VERSION,
                "--port",
                &port.to_string(),
                "--username",
                "admin",
            ],
        );
        Self {
            name: name.to_string(),
            port,
        }
    }

    fn started(name: &str, port: u16) -> Self {
        let cluster = Self::init(name, port);
        ok(pg_home(), &["start", name]);
        cluster
    }

    fn status(&self) -> serde_json::Value {
        json(pg_home(), &["status", &self.name, "--json"])
    }

    fn sql(&self, database: &str, sql: &str) -> String {
        ok(
            pg_home(),
            &["exec", "-c", &self.name, "-d", database, "--sql", sql],
        )
    }
}

impl Drop for Cluster {
    fn drop(&mut self) {
        let _ = dbctl(pg_home(), &["remove", &self.name, "--force"]);
    }
}

#[test]
#[ignore = "installs PostgreSQL"]
fn install_is_idempotent_and_reported_as_installed() {
    let home = pg_home();
    assert!(ok(home, &["install", PG_VERSION]).contains("already installed"));
    let report = json(home, &["versions", "--json"]);
    assert!(
        report["installed"]
            .as_array()
            .expect("installed array")
            .contains(&serde_json::json!(PG_VERSION)),
        "{report}"
    );
}

#[test]
#[ignore = "installs PostgreSQL"]
fn cluster_lifecycle() {
    let home = pg_home();
    let cluster = Cluster::init("it_lifecycle", 25501);

    // Created stopped, without asking "Start the cluster now?".
    assert_eq!(cluster.status()["status"], "stopped");
    let listed = json(home, &["list", "--json"]);
    assert!(
        listed
            .as_array()
            .expect("array")
            .iter()
            .any(|c| c["name"] == "it_lifecycle"),
        "{listed}"
    );

    assert!(ok(home, &["start", "it_lifecycle"]).contains("started"));
    assert!(ok(home, &["start", "it_lifecycle"]).contains("already running"));
    let status = cluster.status();
    assert_eq!(status["status"], "running");
    assert_eq!(status["port"], cluster.port);
    assert_eq!(status["username"], "admin");
    assert_eq!(status["version"], PG_VERSION);

    assert!(ok(home, &["restart", "it_lifecycle"]).contains("restarted"));
    assert_eq!(cluster.status()["status"], "running");
    assert!(ok(home, &["reload", "it_lifecycle"]).contains("reloaded"));
    assert!(
        !ok(home, &["logs", "it_lifecycle", "--lines", "5"])
            .trim()
            .is_empty()
    );

    assert!(ok(home, &["stop", "it_lifecycle"]).contains("stopped"));
    assert_eq!(cluster.status()["status"], "stopped");
}

#[test]
#[ignore = "installs PostgreSQL"]
fn remove_deletes_the_cluster() {
    let home = pg_home();
    let cluster = Cluster::started("it_remove", 25502);
    ok(home, &["remove", &cluster.name, "--force"]);
    assert!(!home.join(".dbctl/clusters/it_remove").exists());
    assert!(fails(home, &["status", "it_remove", "--json"]).contains("does not exist"));
}

#[test]
#[ignore = "installs PostgreSQL"]
fn a_duplicate_cluster_is_rejected() {
    let _cluster = Cluster::init("it_duplicate", 25503);
    let stderr = fails(
        pg_home(),
        &[
            "init",
            "it_duplicate",
            "--version",
            PG_VERSION,
            "--port",
            "25504",
        ],
    );
    assert!(stderr.contains("already exists"), "{stderr}");
}

#[test]
#[ignore = "installs PostgreSQL"]
fn databases_are_created_idempotently_and_dropped() {
    let home = pg_home();
    let cluster = Cluster::started("it_databases", 25505);
    let c = cluster.name.as_str();

    assert!(ok(home, &["create-db", "appdb", "-c", c]).contains("Created database"));
    assert!(ok(home, &["create-db", "appdb", "-c", c]).contains("already exists"));
    let databases = json(home, &["databases", "--cluster", c, "--json"]);
    assert!(
        databases
            .as_array()
            .expect("array")
            .contains(&serde_json::json!("appdb")),
        "{databases}"
    );

    ok(home, &["drop-db", "appdb", "-c", c, "--force"]);
    let databases = json(home, &["databases", "--cluster", c, "--json"]);
    assert!(
        !databases
            .as_array()
            .expect("array")
            .contains(&serde_json::json!("appdb")),
        "{databases}"
    );
}

#[test]
#[ignore = "installs PostgreSQL"]
fn json_reads_do_not_start_a_stopped_cluster() {
    let home = pg_home();
    let cluster = Cluster::init("it_stopped_read", 25506);
    let stderr = fails(home, &["databases", "--cluster", &cluster.name, "--json"]);
    assert!(stderr.contains("not running"), "{stderr}");
    assert_eq!(cluster.status()["status"], "stopped");
}

#[test]
#[ignore = "installs PostgreSQL"]
fn exec_runs_sql_from_a_command_and_a_file() {
    let home = pg_home();
    let cluster = Cluster::started("it_exec", 25507);
    ok(home, &["create-db", "execdb", "-c", &cluster.name]);

    cluster.sql(
        "execdb",
        "create table t (v text); insert into t values ('from_command');",
    );
    let file = home.join("it_exec.sql");
    std::fs::write(&file, "insert into t values ('from_file');").expect("write sql file");
    ok(
        home,
        &[
            "exec",
            "-c",
            &cluster.name,
            "-d",
            "execdb",
            "--file",
            file.to_str().expect("utf-8 path"),
        ],
    );

    let rows = cluster.sql("execdb", "select v from t order by v");
    assert!(
        rows.contains("from_command") && rows.contains("from_file"),
        "{rows}"
    );
}

#[test]
#[ignore = "installs PostgreSQL"]
fn backup_and_restore_round_trip() {
    let home = pg_home();
    let cluster = Cluster::started("it_backup", 25508);
    let c = cluster.name.as_str();
    ok(home, &["create-db", "source", "-c", c]);
    cluster.sql(
        "source",
        "create table t (v text); insert into t values ('backup_data');",
    );

    let dump = home.join("it_backup.sql");
    let dump_path = dump.to_str().expect("utf-8 path");
    ok(home, &["backup", "source", "-o", dump_path, "-c", c]);
    assert!(dump.exists());

    ok(home, &["create-db", "target", "-c", c]);
    ok(home, &["restore", "target", "-i", dump_path, "-c", c]);
    assert!(
        cluster
            .sql("target", "select v from t")
            .contains("backup_data")
    );
}

#[test]
#[ignore = "installs PostgreSQL"]
fn users_are_created_listed_and_dropped() {
    let home = pg_home();
    let cluster = Cluster::started("it_users", 25509);
    let c = cluster.name.as_str();

    let created = ok(
        home,
        &[
            "create-user",
            "appuser",
            "--password",
            "app-password",
            "-c",
            c,
        ],
    );
    assert!(created.contains("Created user"), "{created}");
    let users = json(home, &["users", "--cluster", c, "--json"]);
    let appuser = users
        .as_array()
        .expect("array")
        .iter()
        .find(|u| u["name"] == "appuser")
        .unwrap_or_else(|| panic!("appuser missing: {users}"));
    assert_eq!(appuser["superuser"], false);

    // The hidden legacy path older automation uses to delete a user.
    let dropped = ok(
        home,
        &[
            "db",
            "user",
            "drop",
            "--cluster",
            c,
            "--username",
            "appuser",
        ],
    );
    assert!(dropped.contains("Dropped user"), "{dropped}");
    let users = json(home, &["users", "--cluster", c, "--json"]);
    assert!(
        !users
            .as_array()
            .expect("array")
            .iter()
            .any(|u| u["name"] == "appuser"),
        "{users}"
    );
}

#[test]
#[ignore = "downloads and builds pgvector from source (network, git, make, a C compiler)"]
fn extension_install_enable_list_uninstall() {
    let home = pg_home();
    let cluster = Cluster::started("it_extension", 25510);
    let c = cluster.name.as_str();

    ok(home, &["extension", "install", "pgvector", "--cluster", c]);
    ok(
        home,
        &[
            "extension",
            "enable",
            "pgvector",
            "--cluster",
            c,
            "--database",
            "postgres",
        ],
    );
    let rows = cluster.sql("postgres", "select '[1,2,3]'::vector");
    assert!(rows.contains("[1,2,3]"), "vector type not usable: {rows}");
    assert!(ok(home, &["extension", "list", "--cluster", c]).contains("vector"));
    ok(
        home,
        &["extension", "uninstall", "pgvector", "--cluster", c],
    );
}
