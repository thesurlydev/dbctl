// src/extension.rs
//
// Build, install, and enable PostgreSQL extensions against dbctl-managed
// clusters. Extensions are compiled with PGXS against the exact PostgreSQL
// installation in ~/.dbctl/versions/<ver>/ that the target cluster runs,
// so the resulting binaries always match the server.
use anyhow::{Context as _, Result, bail};
use clap::Subcommand;
use colored::Colorize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Default release used when `--git-tag` is not given, per extension: the
/// tag, and the commit it must resolve to. A tag can be moved, so a clone of
/// the default is checked against the commit before anything is built.
fn default_release(name: &str) -> Option<(&'static str, &'static str)> {
    match name {
        "pgvector" => Some(("v0.8.1", "778dacf20c07caf904557a88705142631818d8cb")),
        _ => None,
    }
}

/// Map an extension name to its source repository.
fn git_url(name: &str) -> Option<&'static str> {
    match name {
        "pgvector" => Some("https://github.com/pgvector/pgvector.git"),
        _ => None,
    }
}

/// The SQL name passed to `CREATE EXTENSION` (may differ from our name).
fn sql_extension_name(name: &str) -> &str {
    match name {
        "pgvector" => "vector",
        other => other,
    }
}

#[derive(Subcommand)]
pub enum ExtensionAction {
    /// Build and install an extension into the PostgreSQL version a cluster runs
    Install {
        /// Extension name (currently supported: pgvector)
        name: String,

        /// Cluster whose PostgreSQL version the extension is built against
        #[arg(long, short = 'c')]
        cluster: String,

        /// Git tag/branch/commit to build (default: pinned per extension)
        #[arg(long)]
        git_tag: Option<String>,

        /// Use an existing local source checkout instead of cloning
        #[arg(long)]
        source: Option<PathBuf>,

        /// Rebuild and reinstall even if already installed
        #[arg(long)]
        force: bool,
    },

    /// Enable an installed extension in a database (CREATE EXTENSION)
    Enable {
        /// Extension name
        name: String,

        /// Cluster name
        #[arg(long, short = 'c')]
        cluster: String,

        /// Database to enable the extension in
        #[arg(long, short = 'd', default_value = "postgres")]
        database: String,
    },

    /// Disable an extension in a database (DROP EXTENSION)
    Disable {
        /// Extension name
        name: String,

        /// Cluster name
        #[arg(long, short = 'c')]
        cluster: String,

        /// Database to disable the extension in
        #[arg(long, short = 'd', default_value = "postgres")]
        database: String,
    },

    /// List extensions available to a cluster's PostgreSQL version, or with
    /// --database, the ones created in that database
    List {
        /// Cluster name
        #[arg(long, short = 'c')]
        cluster: String,

        /// List the extensions created in this database (pg_extension)
        /// instead of the ones available to install
        #[arg(long, short = 'd')]
        database: Option<String>,

        /// Print the names as a JSON array
        #[arg(long)]
        json: bool,
    },

    /// Remove an extension's files from a cluster's PostgreSQL version
    Uninstall {
        /// Extension name
        name: String,

        /// Cluster name
        #[arg(long, short = 'c')]
        cluster: String,
    },
}

pub async fn handle_action(action: ExtensionAction) -> Result<()> {
    match action {
        ExtensionAction::Install {
            name,
            cluster,
            git_tag,
            source,
            force,
        } => install(
            &name,
            &cluster,
            git_tag.as_deref(),
            source.as_deref(),
            force,
        ),
        ExtensionAction::Enable {
            name,
            cluster,
            database,
        } => enable(&name, &cluster, &database),
        ExtensionAction::Disable {
            name,
            cluster,
            database,
        } => disable(&name, &cluster, &database),
        ExtensionAction::List {
            cluster,
            database: Some(database),
            json,
        } => list_created(&cluster, &database, json).await,
        ExtensionAction::List {
            cluster,
            database: None,
            json,
        } => list(&cluster, json),
        ExtensionAction::Uninstall { name, cluster } => uninstall(&name, &cluster),
    }
}

fn install(
    name: &str,
    cluster: &str,
    git_tag: Option<&str>,
    source: Option<&std::path::Path>,
    force: bool,
) -> Result<()> {
    let url = git_url(name).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown extension '{}'. Currently supported: pgvector. \
             Pass --source to build from a local checkout of another extension.",
            name
        )
    })?;
    let (tag, pinned_commit) = match git_tag {
        Some(tag) => (tag.to_string(), None),
        None => default_release(name)
            .map(|(tag, commit)| (tag.to_string(), Some(commit)))
            .ok_or_else(|| anyhow::anyhow!("No pinned release for '{}'; pass --git-tag", name))?,
    };
    // The tag names a directory under the cache and is passed to git, so it
    // must be one plain component that cannot read as an option.
    crate::server::validate_name("git tag", &tag)?;

    let metadata = crate::server::load_cluster_metadata(cluster)?;
    let version_dir = crate::server::get_version_dir(&metadata.version)?;
    let pg_config = version_dir.join("bin/pg_config");
    if !pg_config.exists() {
        bail!(
            "pg_config not found at {}. Is PostgreSQL {} installed? (dbctl install {})",
            pg_config.display(),
            metadata.version,
            metadata.version
        );
    }

    // Where the extension's files will land.
    let share_dir = pg_config_out(&pg_config, "--sharedir")?;
    let lib_dir = pg_config_out(&pg_config, "--pkglibdir")?;
    let control_file = PathBuf::from(&share_dir)
        .join("extension")
        .join(format!("{}.control", sql_extension_name(name)));

    if control_file.exists() && !force {
        println!(
            "{} Extension '{}' is already installed for PostgreSQL {} ({}).",
            "✓".green(),
            name,
            metadata.version,
            control_file.display()
        );
        println!("  Use --force to rebuild and reinstall.");
        return Ok(());
    }

    check_build_tools()?;

    // Binary PostgreSQL distributions (e.g. EDB) are compiled against the
    // macOS SDK that existed on their build machine; pgxs hardcodes that path
    // via `pg_config --cppflags`. If that SDK is missing locally, the build
    // fails inside system headers. Detect a usable SDK and override PG_SYSROOT.
    let sysroot = detect_sysroot(&pg_config)?;
    if let Some(root) = &sysroot {
        println!("Using macOS SDK at {}", root);
    }

    // Get the source tree: either the user's checkout or a pinned clone under
    // the dbctl cache directory.
    let source_dir = match source {
        Some(dir) => {
            if !dir.join("Makefile").exists() {
                bail!(
                    "--source {} does not look like a PGXS extension (no Makefile)",
                    dir.display()
                );
            }
            dir.to_path_buf()
        }
        None => clone_source(name, url, &tag, pinned_commit)?,
    };

    println!(
        "Building {} ({}) against PostgreSQL {} ...",
        name, tag, metadata.version
    );

    // Build and install with PGXS pointed at the cluster's exact toolchain.
    let mut build = Command::new("make");
    build
        .arg("USE_PGXS=1")
        .arg(format!("PG_CONFIG={}", pg_config.display()))
        .current_dir(&source_dir);
    let mut install_cmd = Command::new("make");
    install_cmd
        .arg("USE_PGXS=1")
        .arg(format!("PG_CONFIG={}", pg_config.display()))
        .arg("install")
        .current_dir(&source_dir);
    if let Some(root) = &sysroot {
        build.arg(format!("PG_SYSROOT={}", root));
        install_cmd.arg(format!("PG_SYSROOT={}", root));
    }
    run(&mut build, "build")?;
    run(&mut install_cmd, "install")?;

    if !control_file.exists() {
        bail!(
            "install appeared to succeed but {} was not created",
            control_file.display()
        );
    }

    // Confirm the shared library landed too.
    let so = PathBuf::from(&lib_dir).join(format!("{}.dylib", sql_extension_name(name)));
    let so_alt = so.with_extension("so");
    if !so.exists() && !so_alt.exists() {
        eprintln!(
            "{} warning: shared library not found at {} or {}",
            "!".yellow(),
            so.display(),
            so_alt.display()
        );
    }

    println!(
        "{} Installed '{}' into PostgreSQL {} ({}).",
        "✓".green(),
        name,
        metadata.version,
        version_dir.display()
    );
    println!("  Enable it in a database with:");
    println!(
        "    dbctl extension enable {} --cluster {} --database <db>",
        name, cluster
    );
    Ok(())
}

fn enable(name: &str, cluster: &str, database: &str) -> Result<()> {
    let sql_name = sql_extension_name(name);
    let sql = format!(
        "CREATE EXTENSION IF NOT EXISTS {};",
        crate::postgres::quote_ident(sql_name)
    );
    crate::postgres::exec_sql(cluster, database, &sql)
        .with_context(|| format!("failed to enable extension '{}'", sql_name))?;
    println!(
        "{} Extension '{}' enabled in database '{}' on cluster '{}'.",
        "✓".green(),
        sql_name,
        database,
        cluster
    );
    Ok(())
}

fn disable(name: &str, cluster: &str, database: &str) -> Result<()> {
    let sql_name = sql_extension_name(name);
    let sql = format!(
        "DROP EXTENSION IF EXISTS {};",
        crate::postgres::quote_ident(sql_name)
    );
    crate::postgres::exec_sql(cluster, database, &sql)
        .with_context(|| format!("failed to disable extension '{}'", sql_name))?;
    println!(
        "{} Extension '{}' dropped from database '{}' on cluster '{}'.",
        "✓".green(),
        sql_name,
        database,
        cluster
    );
    Ok(())
}

/// The extensions created in one database. Like dbctl's other reads of a
/// running cluster, this does not start a stopped one: it reports the
/// cluster as it is.
async fn list_created(cluster: &str, database: &str, json: bool) -> Result<()> {
    if !crate::server::is_cluster_running(cluster)? {
        bail!("Cluster '{}' is not running", cluster);
    }
    let created = crate::postgres::list_extensions(cluster, database).await?;
    if json {
        println!("{}", serde_json::to_string(&created)?);
        return Ok(());
    }
    println!(
        "Extensions created in database '{}' on cluster '{}':",
        database, cluster
    );
    for name in &created {
        println!("  {}", name);
    }
    Ok(())
}

fn list(cluster: &str, json: bool) -> Result<()> {
    let metadata = crate::server::load_cluster_metadata(cluster)?;
    let version_dir = crate::server::get_version_dir(&metadata.version)?;
    let pg_config = version_dir.join("bin/pg_config");
    let share_dir = pg_config_out(&pg_config, "--sharedir")?;
    let ext_dir = PathBuf::from(&share_dir).join("extension");

    let mut found = Vec::new();
    if ext_dir.exists() {
        for entry in fs::read_dir(&ext_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("control")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                found.push(stem.to_string());
            }
        }
    }
    found.sort();
    if json {
        println!("{}", serde_json::to_string(&found)?);
        return Ok(());
    }
    println!(
        "Extensions available to cluster '{}' (PostgreSQL {}):",
        cluster, metadata.version
    );
    for f in &found {
        println!("  {}", f);
    }
    if found.is_empty() {
        println!("  (none)");
    }
    Ok(())
}

fn uninstall(name: &str, cluster: &str) -> Result<()> {
    let metadata = crate::server::load_cluster_metadata(cluster)?;
    let version_dir = crate::server::get_version_dir(&metadata.version)?;
    let pg_config = version_dir.join("bin/pg_config");
    let share_dir = pg_config_out(&pg_config, "--sharedir")?;
    let lib_dir = pg_config_out(&pg_config, "--pkglibdir")?;
    let sql_name = sql_extension_name(name);

    let ext_dir = PathBuf::from(&share_dir).join("extension");
    let mut removed = 0;

    // Remove control + SQL files for this extension.
    for entry in fs::read_dir(&ext_dir)? {
        let entry = entry?;
        let fname = entry.file_name().to_string_lossy().to_string();
        if fname == format!("{}.control", sql_name)
            || (fname.starts_with(&format!("{}--", sql_name)) && fname.ends_with(".sql"))
        {
            fs::remove_file(entry.path())?;
            removed += 1;
        }
    }

    // Remove shared libraries.
    for dir in [PathBuf::from(&lib_dir)] {
        for candidate in [
            dir.join(format!("{}.dylib", sql_name)),
            dir.join(format!("{}.so", sql_name)),
        ] {
            if candidate.exists() {
                fs::remove_file(&candidate)?;
                removed += 1;
            }
        }
    }

    if removed == 0 {
        println!(
            "No files for extension '{}' found in PostgreSQL {}.",
            name, metadata.version
        );
    } else {
        println!(
            "{} Removed {} file(s) for extension '{}' from PostgreSQL {}.",
            "✓".green(),
            removed,
            name,
            metadata.version
        );
    }
    Ok(())
}

/// Run `pg_config --<flag>` and return the trimmed output path.
fn pg_config_out(pg_config: &PathBuf, flag: &str) -> Result<String> {
    let output = Command::new(pg_config)
        .arg(flag)
        .output()
        .with_context(|| format!("failed to run {} {}", pg_config.display(), flag))?;
    if !output.status.success() {
        bail!(
            "{} {} failed: {}",
            pg_config.display(),
            flag,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Find a macOS SDK to compile against.
///
/// Returns `None` when the SDK path baked into `pg_config --cppflags` already
/// exists (nothing to do), otherwise `Some(path)` for the best available SDK.
/// Absent any SDK, returns an error telling the user what to install.
fn detect_sysroot(pg_config: &PathBuf) -> Result<Option<String>> {
    if cfg!(not(target_os = "macos")) {
        return Ok(None);
    }
    let cppflags = pg_config_out(pg_config, "--cppflags")?;
    let expected = cppflags
        .split_whitespace()
        .skip_while(|t| *t != "-isysroot")
        .nth(1);

    if let Some(path) = expected
        && PathBuf::from(path).exists()
    {
        return Ok(None); // The SDK the binaries expect is present.
    }

    // Prefer the SDK from the active developer directory.
    if let Ok(output) = Command::new("xcrun").args(["--show-sdk-path"]).output()
        && output.status.success()
    {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() && PathBuf::from(&path).exists() {
            return Ok(Some(path));
        }
    }

    // Fall back to any SDK in the Command Line Tools SDKs directory.
    let sdks_dir = PathBuf::from("/Library/Developer/CommandLineTools/SDKs");
    if let Ok(entries) = fs::read_dir(&sdks_dir) {
        let mut names: Vec<String> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.starts_with("MacOSX") && n.ends_with(".sdk"))
            .collect();
        names.sort();
        if let Some(last) = names.pop() {
            return Ok(Some(sdks_dir.join(last).to_string_lossy().to_string()));
        }
    }

    bail!(
        "No macOS SDK found. The PostgreSQL binaries expect {:?}, which is not installed. \
         Install Xcode Command Line Tools (xcode-select --install).",
        expected
    )
}

fn check_build_tools() -> Result<()> {
    for tool in ["make", "clang"] {
        if Command::new(tool).arg("--version").output().is_err() {
            bail!(
                "'{}' not found. Install Xcode Command Line Tools: xcode-select --install",
                tool
            );
        }
    }
    Ok(())
}

/// Clone the extension source at a ref into the dbctl cache. With a pinned
/// commit, the checkout (new or cached) must be exactly that commit.
fn clone_source(
    name: &str,
    url: &str,
    git_ref: &str,
    pinned_commit: Option<&str>,
) -> Result<PathBuf> {
    let cache_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
        .join(".dbctl")
        .join("cache")
        .join("extensions");
    fs::create_dir_all(&cache_dir)?;
    let dest = cache_dir.join(format!("{}-{}", name, git_ref));

    if dest.exists() {
        println!("Using cached source at {}", dest.display());
        check_commit(&dest, pinned_commit)?;
        return Ok(dest);
    }

    println!("Cloning {} at {} ...", url, git_ref);
    let status = Command::new("git")
        .args(["clone", "--depth", "1", "--branch", git_ref, url])
        .arg(&dest)
        .status()?;
    if !status.success() {
        // Shallow clone of a non-branch ref can fail; fall back to full clone + checkout.
        let _ = fs::remove_dir_all(&dest);
        let status = Command::new("git")
            .args(["clone", url])
            .arg(&dest)
            .status()?;
        if !status.success() {
            bail!("git clone of {} failed", url);
        }
        let status = Command::new("git")
            .args(["checkout", git_ref])
            .current_dir(&dest)
            .status()?;
        if !status.success() {
            bail!("git checkout of {} failed", git_ref);
        }
    }
    if let Err(e) = check_commit(&dest, pinned_commit) {
        let _ = fs::remove_dir_all(&dest);
        return Err(e);
    }
    Ok(dest)
}

/// Fail unless the checkout at `dir` is the pinned commit.
fn check_commit(dir: &std::path::Path, pinned_commit: Option<&str>) -> Result<()> {
    let Some(pinned) = pinned_commit else {
        return Ok(());
    };
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir)
        .output()?;
    let head = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() || head != pinned {
        bail!(
            "the source at {} is commit {head}, not the pinned {pinned}; remove it and \
             install again",
            dir.display()
        );
    }
    Ok(())
}

fn run(cmd: &mut Command, stage: &str) -> Result<()> {
    let output = cmd.output()?;
    if !output.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&output.stdout));
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        bail!("extension {} step failed (see output above)", stage);
    }
    Ok(())
}
