// src/server.rs
use anyhow::{Result, bail};
use clap::Subcommand;
use colored::Colorize;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

/// Available PostgreSQL versions and their latest minor releases (newest first).
/// This is the source of truth used by both `normalize_version()` and `list_available_versions()`.
/// See https://www.enterprisedb.com/software-downloads-postgres for the latest versions.
pub const AVAILABLE_VERSIONS: &[(&str, &str)] = &[
    ("18", "18.4"),
    ("17", "17.10"),
    ("16", "16.14"),
    ("15", "15.18"),
    ("14", "14.23"),
    ("13", "13.23"),
];

#[derive(Subcommand)]
pub enum ServerAction {
    /// Install PostgreSQL version
    Install {
        /// PostgreSQL version
        #[arg(long)]
        version: String,
    },

    /// List installed PostgreSQL versions
    List,

    /// Initialize a new database cluster
    Init {
        /// PostgreSQL version to use
        #[arg(long)]
        version: String,

        /// Name for this cluster
        #[arg(long)]
        name: String,

        /// Port number
        #[arg(long, default_value = "5432")]
        port: u16,

        /// Superuser name
        #[arg(long, default_value = "postgres")]
        username: String,
    },

    /// Start a database cluster
    Start {
        /// Cluster name
        #[arg(long)]
        name: String,
    },

    /// Stop a database cluster
    Stop {
        /// Cluster name
        #[arg(long)]
        name: String,
    },

    /// Restart a database cluster
    Restart {
        /// Cluster name
        #[arg(long)]
        name: String,
    },

    /// Reload cluster configuration without restart
    Reload {
        /// Cluster name
        #[arg(long)]
        name: String,
    },

    /// Show status of a cluster
    Status {
        /// Cluster name
        #[arg(long)]
        name: String,
    },

    /// List all clusters
    ListClusters,

    /// Remove a cluster (WARNING: deletes all data)
    RemoveCluster {
        /// Cluster name
        #[arg(long)]
        name: String,
    },

    /// Uninstall PostgreSQL version
    Uninstall {
        /// PostgreSQL version to uninstall
        #[arg(long)]
        version: String,
    },

    /// View cluster logs
    Logs {
        /// Cluster name
        #[arg(long)]
        name: String,

        /// Number of lines to show
        #[arg(long, short = 'n', default_value = "50")]
        lines: usize,

        /// Follow log output
        #[arg(long, short = 'f')]
        follow: bool,
    },
}

pub async fn handle_action(action: ServerAction) -> Result<()> {
    match action {
        ServerAction::Install { version } => {
            install_postgres(&version).await?;
        }
        ServerAction::List => {
            list_installed()?;
        }
        ServerAction::Init {
            version,
            name,
            port,
            username,
        } => {
            init_cluster(&version, &name, port, &username)?;
        }
        ServerAction::Start { name } => {
            start_cluster(&name)?;
        }
        ServerAction::Stop { name } => {
            stop_cluster(&name)?;
        }
        ServerAction::Restart { name } => {
            restart_cluster(&name)?;
        }
        ServerAction::Reload { name } => {
            reload_cluster(&name)?;
        }
        ServerAction::Status { name } => {
            show_status(&name).await?;
        }
        ServerAction::ListClusters => {
            list_clusters()?;
        }
        ServerAction::RemoveCluster { name } => {
            remove_cluster(&name)?;
        }
        ServerAction::Uninstall { version } => {
            uninstall_postgres(&version)?;
        }
        ServerAction::Logs {
            name,
            lines,
            follow,
        } => {
            show_logs(&name, lines, follow)?;
        }
    }
    Ok(())
}

fn get_base_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Could not determine home directory")
        .join(".dbctl")
}

fn get_versions_dir() -> PathBuf {
    get_base_dir().join("versions")
}

fn get_clusters_dir() -> PathBuf {
    get_base_dir().join("clusters")
}

pub fn get_version_dir(version: &str) -> PathBuf {
    get_versions_dir().join(version)
}

pub fn get_installed_versions() -> Result<Vec<String>> {
    let versions_dir = get_versions_dir();

    if !versions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut versions = Vec::new();
    for entry in fs::read_dir(&versions_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            versions.push(entry.file_name().to_string_lossy().to_string());
        }
    }

    versions.sort();
    versions.reverse(); // Latest first

    Ok(versions)
}

pub fn get_cluster_dir(name: &str) -> PathBuf {
    get_clusters_dir().join(name)
}

pub async fn install_postgres(version: &str) -> Result<()> {
    let version_dir = get_version_dir(version);

    if version_dir.exists() {
        println!("PostgreSQL {} is already installed", version);
        return Ok(());
    }

    println!("Installing PostgreSQL {}...", version);

    // Detect platform
    let (os, arch) = detect_platform()?;
    println!("Detected platform: {} ({})", os, arch);

    match os.as_str() {
        "macos" => install_postgres_macos(version, &version_dir, &arch).await?,
        "linux" => install_postgres_linux(version, &version_dir, &arch).await?,
        _ => bail!("Unsupported platform: {}", os),
    }

    println!("✓ PostgreSQL {} installed successfully", version);
    println!("\nNext steps:");
    println!("  dbctl init my_cluster --version {} --port 5432", version);

    Ok(())
}

fn detect_platform() -> Result<(String, String)> {
    let os = if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        bail!("Unsupported operating system")
    };

    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else {
        bail!("Unsupported architecture")
    };

    Ok((os.to_string(), arch.to_string()))
}

fn normalize_version(version: &str) -> String {
    // Normalize version strings like "17" to latest minor version (e.g., "17.7")
    // This ensures compatibility with EnterpriseDB URL patterns and provides
    // the latest stable release for each major version
    let parts: Vec<&str> = version.split('.').collect();
    match parts.len() {
        1 => {
            for (major, full) in AVAILABLE_VERSIONS {
                if *major == parts[0] {
                    return full.to_string();
                }
            }
            // For unknown versions, default to .0
            format!("{}.0", version)
        }
        _ => version.to_string(),
    }
}

/// What `dbctl versions --json` prints: what can be installed, and what is.
#[derive(Debug, serde::Serialize)]
pub struct VersionsReport {
    pub available: Vec<String>,
    pub installed: Vec<String>,
}

pub fn versions_report() -> Result<VersionsReport> {
    Ok(VersionsReport {
        available: AVAILABLE_VERSIONS
            .iter()
            .map(|(_major, full)| full.to_string())
            .collect(),
        installed: get_installed_versions()?,
    })
}

pub fn list_available_versions() -> Result<()> {
    let installed = get_installed_versions()?;

    println!("Available PostgreSQL versions:\n");

    for (_major, full_version) in AVAILABLE_VERSIONS {
        let is_installed = installed.iter().any(|v| v == full_version);
        if is_installed {
            println!("  {}     {}", full_version, "(installed)".green());
        } else {
            println!("  {}", full_version);
        }
    }

    Ok(())
}

async fn install_postgres_macos(version: &str, install_dir: &PathBuf, _arch: &str) -> Result<()> {
    // EDB provides universal binaries for macOS
    // Normalize version (e.g., "17" -> "17.7") to latest minor version for URL compatibility
    let normalized_version = normalize_version(version);

    fs::create_dir_all(install_dir)?;

    // Use "universal" for cache key since macOS binaries work on all architectures
    let os = "macos";
    let cache_arch = "universal";

    if crate::cache::is_cached(&normalized_version, os, cache_arch)? {
        println!("✓ Found in cache, extracting...");
        let cache_path = crate::cache::get_cache_path(&normalized_version, os, cache_arch)?;
        extract_zip_from_file(&cache_path, install_dir)?;
    } else {
        let url = format!(
            "https://get.enterprisedb.com/postgresql/postgresql-{}-1-osx-binaries.zip",
            normalized_version
        );
        println!("Downloading from: {}", url);

        let data = download_binary(&url).await?;
        println!("✓ Saving to cache...");
        let cache_path = crate::cache::save_to_cache(&normalized_version, os, cache_arch, &data)?;

        extract_zip_from_file(&cache_path, install_dir)?;
    }

    // The extracted structure is pgsql/
    let pgsql_dir = install_dir.join("pgsql");
    if pgsql_dir.exists() {
        // Move contents up
        for entry in fs::read_dir(&pgsql_dir)? {
            let entry = entry?;
            let dest = install_dir.join(entry.file_name());
            fs::rename(entry.path(), dest)?;
        }
        fs::remove_dir_all(&pgsql_dir)?;
    }

    println!("✓ Installation complete");

    Ok(())
}

async fn install_postgres_linux(version: &str, install_dir: &PathBuf, arch: &str) -> Result<()> {
    let arch_str = if arch == "arm64" { "arm64" } else { "x64" };

    // Normalize version (e.g., "17" -> "17.7") to latest minor version for URL compatibility
    let normalized_version = normalize_version(version);

    fs::create_dir_all(install_dir)?;

    // Check cache first - Linux binaries are cached per-architecture
    let os = "linux";
    if crate::cache::is_cached(&normalized_version, os, arch_str)? {
        println!("✓ Found in cache, extracting...");
        let cache_path = crate::cache::get_cache_path(&normalized_version, os, arch_str)?;
        extract_targz_from_file(&cache_path, install_dir)?;
    } else {
        let url = format!(
            "https://get.enterprisedb.com/postgresql/postgresql-{}-1-linux-{}-binaries.tar.gz",
            normalized_version, arch_str
        );
        println!("Downloading from: {}", url);

        let data = download_binary(&url).await?;
        println!("✓ Saving to cache...");
        let cache_path = crate::cache::save_to_cache(&normalized_version, os, arch_str, &data)?;

        extract_targz_from_file(&cache_path, install_dir)?;
    }

    // The extracted structure is pgsql/
    let pgsql_dir = install_dir.join("pgsql");
    if pgsql_dir.exists() {
        // Move contents up
        for entry in fs::read_dir(&pgsql_dir)? {
            let entry = entry?;
            let dest = install_dir.join(entry.file_name());
            fs::rename(entry.path(), dest)?;
        }
        fs::remove_dir_all(&pgsql_dir)?;
    }

    println!("✓ Installation complete");

    Ok(())
}

async fn download_binary(url: &str) -> Result<Vec<u8>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()?;

    let response = client.get(url).send().await?;
    let total_size = response.content_length().unwrap_or(0);

    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
        .progress_chars("#>-"));

    let mut downloaded = 0u64;
    let mut stream = response.bytes_stream();
    let mut buffer = Vec::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buffer.extend_from_slice(&chunk);
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message("Download complete");

    Ok(buffer)
}

fn extract_targz_from_file(file_path: &std::path::Path, dest_dir: &std::path::Path) -> Result<()> {
    println!("Extracting...");

    let file = fs::File::open(file_path)?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(dest_dir)?;

    println!("✓ Extraction complete");

    Ok(())
}

fn extract_zip_from_file(file_path: &std::path::Path, dest_dir: &std::path::Path) -> Result<()> {
    println!("Extracting...");

    let file = fs::File::open(file_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    // The crate's extractor sanitizes entry paths (no `..` or absolute
    // paths), only creates symlinks that resolve inside dest_dir, and applies
    // Unix permissions without following links - none of which a hand-rolled
    // loop over raw entry names gets right.
    archive.extract(dest_dir)?;

    println!("✓ Extraction complete");

    Ok(())
}

fn list_installed() -> Result<()> {
    let versions_dir = get_versions_dir();

    if !versions_dir.exists() {
        println!("No PostgreSQL versions installed.");
        println!("\nTo install a version:");
        println!("  dbctl install 16.3");
        return Ok(());
    }

    println!("Installed PostgreSQL versions:\n");

    let mut versions = Vec::new();
    for entry in fs::read_dir(&versions_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            versions.push(entry.file_name().to_string_lossy().to_string());
        }
    }

    versions.sort();

    if versions.is_empty() {
        println!("  (none)");
    } else {
        for version in versions {
            let path = versions_dir.join(&version);
            println!("  {} ({})", version, path.display());
        }
    }

    Ok(())
}

pub fn init_cluster(version: &str, name: &str, port: u16, username: &str) -> Result<()> {
    let version_dir = get_version_dir(version);

    if !version_dir.exists() {
        bail!(
            "PostgreSQL {} is not installed. Run: dbctl install {}",
            version,
            version
        );
    }

    let cluster_dir = get_cluster_dir(name);

    if cluster_dir.exists() {
        bail!("Cluster '{}' already exists", name);
    }

    let data_dir = cluster_dir.join("data");
    let logs_dir = cluster_dir.join("logs");

    fs::create_dir_all(&data_dir)?;
    fs::create_dir_all(&logs_dir)?;

    let initdb_path = version_dir.join("bin").join("initdb");

    println!(
        "Initializing cluster '{}' with PostgreSQL {}...",
        name, version
    );

    let status = Command::new(&initdb_path)
        .args([
            "-D",
            data_dir.to_str().unwrap(),
            "-U",
            username,
            "--encoding=UTF8",
            "--locale=C",
        ])
        .status()?;

    if !status.success() {
        fs::remove_dir_all(&cluster_dir)?;
        bail!("Failed to initialize cluster");
    }

    // Save cluster metadata
    let metadata = ClusterMetadata {
        version: version.to_string(),
        port,
        username: username.to_string(),
        data_dir: data_dir.clone(),
        logs_dir: logs_dir.clone(),
    };

    let metadata_path = cluster_dir.join("metadata.json");
    let metadata_json = serde_json::to_string_pretty(&metadata)?;
    fs::write(metadata_path, metadata_json)?;

    // Update postgresql.conf with the port
    let conf_path = data_dir.join("postgresql.conf");
    let mut conf_content = fs::read_to_string(&conf_path)?;
    conf_content.push_str(&format!("\nport = {}\n", port));
    conf_content.push_str(&format!(
        "unix_socket_directories = '{}'\n",
        socket_dir_for(&cluster_dir, port).display()
    ));
    fs::write(conf_path, conf_content)?;

    // Update pg_hba.conf for local connections
    let hba_path = data_dir.join("pg_hba.conf");
    let mut hba_content = fs::read_to_string(&hba_path)?;
    hba_content.push_str("\n# Allow local connections without password\n");
    hba_content.push_str("local   all             all                                     trust\n");
    hba_content.push_str("host    all             all             127.0.0.1/32            trust\n");
    hba_content.push_str("host    all             all             ::1/128                 trust\n");
    fs::write(hba_path, hba_content)?;

    println!("✓ Cluster '{}' initialized", name);
    println!("\nConnection details:");
    println!("  Port: {}", port);
    println!("  User: {}", username);
    println!("\nTo start the cluster:");
    println!("  dbctl start {}", name);

    Ok(())
}

/// Longest unix socket path the platform accepts (`sun_path` less its NUL).
#[cfg(target_os = "macos")]
const MAX_SOCKET_PATH: usize = 103;
#[cfg(not(target_os = "macos"))]
const MAX_SOCKET_PATH: usize = 107;

/// Where a cluster's unix socket goes: its own directory when the socket
/// path fits the platform limit, otherwise /tmp, PostgreSQL's own default.
/// Under a deep HOME the cluster directory alone is too long, and PostgreSQL
/// then refuses to start at all ("Unix-domain socket path ... is too long").
/// dbctl itself always connects over TCP, so only local psql users see where
/// the socket lives.
fn socket_dir_for(cluster_dir: &std::path::Path, port: u16) -> PathBuf {
    let socket = cluster_dir.join(format!(".s.PGSQL.{}", port));
    if socket.as_os_str().len() <= MAX_SOCKET_PATH {
        cluster_dir.to_path_buf()
    } else {
        PathBuf::from("/tmp")
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ClusterMetadata {
    pub version: String,
    pub port: u16,
    pub username: String,
    pub data_dir: PathBuf,
    pub logs_dir: PathBuf,
}

pub fn load_cluster_metadata(name: &str) -> Result<ClusterMetadata> {
    let cluster_dir = get_cluster_dir(name);
    let metadata_path = cluster_dir.join("metadata.json");

    if !metadata_path.exists() {
        bail!("Cluster '{}' does not exist", name);
    }

    let metadata_json = fs::read_to_string(metadata_path)?;
    let metadata: ClusterMetadata = serde_json::from_str(&metadata_json)?;

    Ok(metadata)
}

pub fn find_unused_port() -> Result<u16> {
    use rand::RngExt;

    // Get all ports currently in use by clusters
    let clusters = get_all_clusters()?;
    let used_ports: std::collections::HashSet<u16> =
        clusters.iter().map(|(_, metadata)| metadata.port).collect();

    let mut rng = rand::rng();

    // Try up to 100 times to find an unused port
    for _ in 0..100 {
        // Generate random port in range 5432-6432 (PostgreSQL default range)
        let port = rng.random_range(5432..=6432);

        // Skip if already used by a cluster
        if used_ports.contains(&port) {
            continue;
        }

        // Check if port is actually available
        if is_port_available(port) {
            return Ok(port);
        }
    }

    bail!("Could not find an unused port after 100 attempts")
}

fn is_port_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

pub fn get_all_clusters() -> Result<Vec<(String, ClusterMetadata)>> {
    let clusters_dir = get_clusters_dir();
    let mut clusters = Vec::new();

    if !clusters_dir.exists() {
        return Ok(clusters);
    }

    // An entry that vanishes or cannot be read mid-scan (a concurrent
    // `dbctl remove`) is skipped, like a cluster with unreadable metadata.
    for entry in fs::read_dir(&clusters_dir)?.flatten() {
        if entry.file_type().is_ok_and(|t| t.is_dir()) {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Ok(metadata) = load_cluster_metadata(&name) {
                clusters.push((name, metadata));
            }
        }
    }

    Ok(clusters)
}

pub fn is_cluster_running(name: &str) -> Result<bool> {
    pg_ctl_reports_running(&load_cluster_metadata(name)?)
}

fn pg_ctl_reports_running(metadata: &ClusterMetadata) -> Result<bool> {
    let version_dir = get_version_dir(&metadata.version);
    let pg_ctl_path = version_dir.join("bin").join("pg_ctl");

    let output = Command::new(&pg_ctl_path)
        .args(["-D", metadata.data_dir.to_str().unwrap(), "status"])
        .output()?;

    Ok(output.status.success())
}

/// One cluster as `dbctl list --json` and `dbctl status <name> --json` print it.
#[derive(Debug, serde::Serialize)]
pub struct ClusterSummary {
    pub name: String,
    pub version: String,
    pub port: u16,
    pub username: String,
    /// `running` or `stopped`.
    pub status: &'static str,
}

pub fn cluster_summary(name: &str) -> Result<ClusterSummary> {
    Ok(summarize(name.to_string(), load_cluster_metadata(name)?))
}

/// Every cluster, sorted by name (directory order is not stable).
///
/// Built from the metadata the directory scan already loaded, not by loading
/// each cluster again: a cluster removed in between, or one with unreadable
/// metadata, must not fail the whole list: a caller automating deploys would
/// treat a failed list as a failed deploy.
pub fn cluster_summaries() -> Result<Vec<ClusterSummary>> {
    let mut summaries: Vec<_> = get_all_clusters()?
        .into_iter()
        .map(|(name, metadata)| summarize(name, metadata))
        .collect();
    summaries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(summaries)
}

fn summarize(name: String, metadata: ClusterMetadata) -> ClusterSummary {
    // pg_ctl that cannot even run (its version was uninstalled) means the
    // cluster cannot be running either.
    let status = if pg_ctl_reports_running(&metadata).unwrap_or(false) {
        "running"
    } else {
        "stopped"
    };
    ClusterSummary {
        name,
        version: metadata.version,
        port: metadata.port,
        username: metadata.username,
        status,
    }
}

pub fn ensure_cluster_running(name: &str) -> Result<()> {
    if !is_cluster_running(name)? {
        start_cluster(name)?;
    }
    Ok(())
}

pub fn start_cluster(name: &str) -> Result<()> {
    let metadata = load_cluster_metadata(name)?;

    // Starting a running cluster is the desired end state, not an error:
    // pg_ctl would fail on the port it already holds.
    if is_cluster_running(name)? {
        println!("Cluster '{}' is already running", name);
        return Ok(());
    }
    let version_dir = get_version_dir(&metadata.version);
    let pg_ctl_path = version_dir.join("bin").join("pg_ctl");

    let logfile = metadata.logs_dir.join("postgresql.log");

    println!(
        "Starting cluster '{}' (PostgreSQL {})...",
        name, metadata.version
    );

    let status = Command::new(&pg_ctl_path)
        .args([
            "-D",
            metadata.data_dir.to_str().unwrap(),
            "-l",
            logfile.to_str().unwrap(),
            "start",
        ])
        .status()?;

    if !status.success() {
        bail!("Failed to start cluster");
    }

    println!("✓ Cluster '{}' started", name);
    println!("\nConnection string:");
    println!(
        "  postgresql://{}@localhost:{}/postgres",
        metadata.username, metadata.port
    );

    Ok(())
}

pub fn stop_cluster(name: &str) -> Result<()> {
    let metadata = load_cluster_metadata(name)?;
    let version_dir = get_version_dir(&metadata.version);
    let pg_ctl_path = version_dir.join("bin").join("pg_ctl");

    println!("Stopping cluster '{}'...", name);

    let status = Command::new(&pg_ctl_path)
        .args([
            "-D",
            metadata.data_dir.to_str().unwrap(),
            "stop",
            "-m",
            "fast",
        ])
        .status()?;

    if !status.success() {
        bail!("Failed to stop cluster");
    }

    println!("✓ Cluster '{}' stopped", name);

    Ok(())
}

pub fn restart_cluster(name: &str) -> Result<()> {
    let metadata = load_cluster_metadata(name)?;
    let version_dir = get_version_dir(&metadata.version);
    let pg_ctl_path = version_dir.join("bin").join("pg_ctl");

    let logfile = metadata.logs_dir.join("postgresql.log");

    println!(
        "Restarting cluster '{}' (PostgreSQL {})...",
        name, metadata.version
    );

    let status = Command::new(&pg_ctl_path)
        .args([
            "-D",
            metadata.data_dir.to_str().unwrap(),
            "-l",
            logfile.to_str().unwrap(),
            "restart",
            "-m",
            "fast",
        ])
        .status()?;

    if !status.success() {
        bail!("Failed to restart cluster");
    }

    println!("✓ Cluster '{}' restarted", name);
    println!("\nConnection string:");
    println!(
        "  postgresql://{}@localhost:{}/postgres",
        metadata.username, metadata.port
    );

    Ok(())
}

pub fn reload_cluster(name: &str) -> Result<()> {
    let metadata = load_cluster_metadata(name)?;
    let version_dir = get_version_dir(&metadata.version);
    let pg_ctl_path = version_dir.join("bin").join("pg_ctl");

    println!("Reloading configuration for cluster '{}'...", name);

    let status = Command::new(&pg_ctl_path)
        .args(["-D", metadata.data_dir.to_str().unwrap(), "reload"])
        .status()?;

    if !status.success() {
        bail!("Failed to reload cluster configuration");
    }

    println!("✓ Configuration reloaded for cluster '{}'", name);

    Ok(())
}

async fn show_status(name: &str) -> Result<()> {
    let metadata = load_cluster_metadata(name)?;
    let version_dir = get_version_dir(&metadata.version);
    let pg_ctl_path = version_dir.join("bin").join("pg_ctl");

    println!("Cluster: {}", name);
    println!("Version: {}", metadata.version);
    println!("Port: {}", metadata.port);
    println!("User: {}", metadata.username);
    println!("\nStatus:");

    let output = Command::new(&pg_ctl_path)
        .args(["-D", metadata.data_dir.to_str().unwrap(), "status"])
        .output()?;

    print!("{}", String::from_utf8_lossy(&output.stdout));

    if is_cluster_running(name).unwrap_or(false)
        && let Ok(stats) = crate::postgres::get_cluster_stats(name).await
    {
        println!("\nStatistics:");
        println!("  Databases: {}", stats.database_count);
        println!("  Users:     {}", stats.user_count);
        println!("  Tables:    {}", stats.table_count);
    }

    Ok(())
}

fn list_clusters() -> Result<()> {
    let clusters_dir = get_clusters_dir();

    if !clusters_dir.exists() {
        println!("No clusters found.");
        println!("\nTo create a cluster:");
        println!("  dbctl init my_cluster --version 16.3 --port 5432");
        return Ok(());
    }

    println!("PostgreSQL clusters:\n");

    let mut clusters = Vec::new();
    for entry in fs::read_dir(&clusters_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Ok(metadata) = load_cluster_metadata(&name) {
                clusters.push((name, metadata));
            }
        }
    }

    if clusters.is_empty() {
        println!("  (none)");
    } else {
        for (name, metadata) in clusters {
            println!(
                "  {} (PostgreSQL {}, port {})",
                name, metadata.version, metadata.port
            );
        }
    }

    Ok(())
}

pub fn remove_cluster(name: &str) -> Result<()> {
    let cluster_dir = get_cluster_dir(name);

    if !cluster_dir.exists() {
        bail!("Cluster '{}' does not exist", name);
    }

    // Try to stop it first
    let _ = stop_cluster(name);

    println!("Removing cluster '{}'...", name);
    fs::remove_dir_all(&cluster_dir)?;

    println!("✓ Cluster '{}' removed", name);

    Ok(())
}

fn uninstall_postgres(version: &str) -> Result<()> {
    let version_dir = get_version_dir(version);

    if !version_dir.exists() {
        println!("PostgreSQL {} is not installed", version);
        return Ok(());
    }

    // Check if any clusters are using this version
    let clusters_dir = get_clusters_dir();
    if clusters_dir.exists() {
        for entry in fs::read_dir(&clusters_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Ok(metadata) = load_cluster_metadata(&name)
                    && metadata.version == version
                {
                    bail!(
                        "Cannot uninstall PostgreSQL {} - cluster '{}' is using it",
                        version,
                        name
                    );
                }
            }
        }
    }

    println!("Uninstalling PostgreSQL {}...", version);
    fs::remove_dir_all(&version_dir)?;

    println!("✓ PostgreSQL {} uninstalled", version);

    Ok(())
}

pub fn get_logs(name: &str, lines: usize) -> Result<String> {
    let metadata = load_cluster_metadata(name)?;
    let log_dir = &metadata.logs_dir;

    if !log_dir.exists() {
        bail!("Log directory not found for cluster '{}'", name);
    }

    // Find the most recent log file
    let mut log_files: Vec<_> = fs::read_dir(log_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "log")
                .unwrap_or(false)
        })
        .collect();

    if log_files.is_empty() {
        return Ok(format!("No log files found for cluster '{}'", name));
    }

    log_files.sort_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()));
    let log_file = log_files.last().unwrap().path();

    // Read last N lines
    let content = fs::read_to_string(&log_file)?;
    let all_lines: Vec<&str> = content.lines().collect();
    let start = if all_lines.len() > lines {
        all_lines.len() - lines
    } else {
        0
    };

    Ok(all_lines[start..].join("\n"))
}

fn show_logs(name: &str, lines: usize, follow: bool) -> Result<()> {
    if follow {
        let metadata = load_cluster_metadata(name)?;
        let log_dir = &metadata.logs_dir;

        let mut log_files: Vec<_> = fs::read_dir(log_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s == "log")
                    .unwrap_or(false)
            })
            .collect();

        log_files.sort_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()));
        let log_file = log_files.last().unwrap().path();

        let status = Command::new("tail")
            .arg("-f")
            .arg("-n")
            .arg(lines.to_string())
            .arg(&log_file)
            .status()?;

        if !status.success() {
            bail!("tail command failed");
        }
    } else {
        let logs = get_logs(name, lines)?;
        println!("{}", logs);
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct ClusterResources {
    pub cpu_percent: f32,
    pub memory_mb: u64,
    pub disk_mb: u64,
}

pub fn get_cluster_resources(name: &str) -> Result<Option<ClusterResources>> {
    if !is_cluster_running(name)? {
        return Ok(None);
    }

    let metadata = load_cluster_metadata(name)?;

    // Initialize system info with specific refresh kind
    let refresh_kind = RefreshKind::nothing().with_processes(
        ProcessRefreshKind::nothing()
            .with_cpu()
            .with_memory()
            .with_cmd(sysinfo::UpdateKind::Always),
    );
    let mut system = System::new_with_specifics(refresh_kind);

    // First refresh to initialize
    system.refresh_specifics(refresh_kind);

    // Wait a bit for CPU usage to be accurate
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Second refresh to calculate CPU usage
    system.refresh_specifics(refresh_kind);

    // Find PostgreSQL processes for this cluster by checking data directory
    let data_dir_str = metadata.data_dir.to_string_lossy().to_string();
    let mut total_cpu = 0.0;
    let mut total_memory = 0u64;

    // First, find the main postgres process for this cluster
    let mut main_pid = None;
    for (pid, process) in system.processes() {
        let process_name = process.name().to_string_lossy().to_string();

        if process_name.contains("postgres") {
            let cmd = process.cmd();
            let cmd_line = cmd
                .iter()
                .filter_map(|arg| arg.to_str())
                .collect::<Vec<_>>()
                .join(" ");

            // Find main postgres process with data directory
            if cmd_line.contains(&data_dir_str) && cmd_line.contains("-D") {
                main_pid = Some(*pid);
                total_cpu += process.cpu_usage();
                total_memory += process.memory();
                break;
            }
        }
    }

    // Then find all child processes (they have the same parent PID)
    if let Some(parent_pid) = main_pid {
        for (pid, process) in system.processes() {
            let process_name = process.name().to_string_lossy().to_string();

            if process_name.contains("postgres")
                && *pid != parent_pid
                && let Some(ppid) = process.parent()
                && ppid == parent_pid
            {
                total_cpu += process.cpu_usage();
                total_memory += process.memory();
            }
        }
    }

    // Get disk usage for data directory
    let disk_bytes = get_directory_size_bytes(&metadata.data_dir)?;
    let disk_mb = disk_bytes / (1024 * 1024);

    Ok(Some(ClusterResources {
        cpu_percent: total_cpu,
        memory_mb: total_memory / (1024 * 1024), // Convert bytes to MB
        disk_mb,
    }))
}

fn get_directory_size_bytes(path: &PathBuf) -> Result<u64> {
    let mut total_size = 0u64;

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                total_size += get_directory_size_bytes(&path)?;
            } else if path.is_file()
                && let Ok(metadata) = fs::metadata(&path)
            {
                total_size += metadata.len();
            }
        }
    }

    Ok(total_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_stays_in_the_cluster_dir_when_it_fits() {
        let dir = PathBuf::from("/home/app/.dbctl/clusters/main");
        assert_eq!(socket_dir_for(&dir, 5432), dir);
    }

    #[test]
    fn socket_falls_back_to_tmp_under_a_deep_home() {
        let deep = PathBuf::from(format!("/{}/.dbctl/clusters/main", "d".repeat(100)));
        assert_eq!(socket_dir_for(&deep, 5432), PathBuf::from("/tmp"));
    }

    // `--json` output is a public interface that callers parse: these exact
    // strings are pinned, so change a shape only as a breaking change.
    #[test]
    fn cluster_summary_json_shape() {
        let summary = ClusterSummary {
            name: "main".to_string(),
            version: "16.3".to_string(),
            port: 5432,
            username: "app".to_string(),
            status: "running",
        };
        assert_eq!(
            serde_json::to_string(&vec![summary]).unwrap(),
            r#"[{"name":"main","version":"16.3","port":5432,"username":"app","status":"running"}]"#
        );
    }

    #[test]
    fn versions_report_json_shape() {
        let report = VersionsReport {
            available: vec!["18.4".to_string(), "17.10".to_string()],
            installed: vec!["16.3".to_string()],
        };
        assert_eq!(
            serde_json::to_string(&report).unwrap(),
            r#"{"available":["18.4","17.10"],"installed":["16.3"]}"#
        );
    }

    #[test]
    fn user_info_json_shape() {
        let user = crate::postgres::UserInfo {
            name: "app".to_string(),
            superuser: true,
            create_db: true,
            create_role: true,
        };
        assert_eq!(
            serde_json::to_string(&vec![user]).unwrap(),
            r#"[{"name":"app","superuser":true,"create_db":true,"create_role":true}]"#
        );
    }

    #[test]
    fn test_normalize_version_major_only() {
        // Test that major versions map to latest minor versions
        assert_eq!(normalize_version("18"), "18.4");
        assert_eq!(normalize_version("17"), "17.10");
        assert_eq!(normalize_version("16"), "16.14");
        assert_eq!(normalize_version("15"), "15.18");
        assert_eq!(normalize_version("14"), "14.23");
        assert_eq!(normalize_version("13"), "13.23");
    }

    #[test]
    fn test_normalize_version_major_minor() {
        // Test that full versions are passed through unchanged
        assert_eq!(normalize_version("17.0"), "17.0");
        assert_eq!(normalize_version("17.5"), "17.5");
        assert_eq!(normalize_version("16.3"), "16.3");
        assert_eq!(normalize_version("15.8"), "15.8");
        assert_eq!(normalize_version("14.12"), "14.12");
    }

    #[test]
    fn test_normalize_version_major_minor_patch() {
        // Test that versions with patch numbers are passed through unchanged
        assert_eq!(normalize_version("16.3.1"), "16.3.1");
        assert_eq!(normalize_version("15.8.2"), "15.8.2");
    }

    #[test]
    fn test_normalize_version_unknown_major() {
        // Test that unknown major versions default to .0
        assert_eq!(normalize_version("19"), "19.0");
        assert_eq!(normalize_version("20"), "20.0");
        assert_eq!(normalize_version("99"), "99.0");
    }

    #[test]
    fn test_normalize_version_edge_cases() {
        // Test edge cases
        assert_eq!(normalize_version("17.7"), "17.7"); // Already latest
        assert_eq!(normalize_version("16.11"), "16.11"); // Already latest
        assert_eq!(normalize_version("12"), "12.0"); // EOL version not in mapping
    }

    #[test]
    fn test_normalize_version_preserves_custom_versions() {
        // Test that custom/specific versions are preserved
        assert_eq!(normalize_version("17.2"), "17.2");
        assert_eq!(normalize_version("16.5"), "16.5");
        assert_eq!(normalize_version("15.1"), "15.1");
    }
}
