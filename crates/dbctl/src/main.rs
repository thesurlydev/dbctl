// src/main.rs
use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use colored::Colorize;

use dbctl::context::Context;
use dbctl::{cache, extension, postgres, server};

#[derive(Parser)]
#[command(name = "dbctl")]
#[command(about = "PostgreSQL version and database manager")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Install PostgreSQL version
    Install {
        /// PostgreSQL version (e.g., 16.3)
        version: String,
    },

    /// List available PostgreSQL versions
    Versions,

    /// Manage the local PostgreSQL binary cache
    Cache {
        #[command(subcommand)]
        action: cache::CacheAction,
    },

    /// Initialize a new database cluster (interactive if args missing)
    Init {
        /// Cluster name
        name: Option<String>,

        /// PostgreSQL version to use
        #[arg(long)]
        version: Option<String>,

        /// Port number
        #[arg(long)]
        port: Option<u16>,

        /// Superuser name
        #[arg(long)]
        username: Option<String>,
    },

    /// Start a cluster
    Start {
        /// Cluster name (optional if only one exists)
        name: Option<String>,
    },

    /// Stop a cluster
    Stop {
        /// Cluster name (optional if only one exists)
        name: Option<String>,
    },

    /// Restart a cluster
    Restart {
        /// Cluster name (optional if only one exists)
        name: Option<String>,
    },

    /// Reload cluster configuration
    Reload {
        /// Cluster name (optional if only one exists)
        name: Option<String>,
    },

    /// Show status (all clusters if no name provided)
    Status {
        /// Cluster name (optional - shows all if omitted)
        name: Option<String>,
    },

    /// List everything (alias: ls)
    #[command(alias = "ls")]
    List,

    /// Remove a cluster
    #[command(alias = "rm")]
    Remove {
        /// Cluster name
        name: String,

        /// Skip confirmation
        #[arg(long, short)]
        force: bool,
    },

    /// View cluster logs
    Logs {
        /// Cluster name (optional if only one exists)
        name: Option<String>,

        /// Number of lines to show
        #[arg(long, short = 'n', default_value = "50")]
        lines: usize,

        /// Follow log output
        #[arg(long, short = 'f')]
        follow: bool,
    },

    /// Create a database
    #[command(alias = "createdb")]
    CreateDb {
        /// Database name
        name: String,

        /// Cluster name (optional - auto-selected)
        #[arg(long, short = 'c')]
        cluster: Option<String>,
    },

    /// Drop a database
    #[command(alias = "dropdb")]
    DropDb {
        /// Database name
        name: String,

        /// Cluster name (optional - auto-selected)
        #[arg(long, short = 'c')]
        cluster: Option<String>,

        /// Skip confirmation
        #[arg(long, short)]
        force: bool,
    },

    /// List all databases
    Databases {
        /// Cluster name (optional - shows all if omitted)
        #[arg(long, short = 'c')]
        cluster: Option<String>,
    },

    /// Open interactive psql shell
    Shell {
        /// Database name (optional)
        database: Option<String>,

        /// Cluster name (optional - auto-selected)
        #[arg(long, short = 'c')]
        cluster: Option<String>,
    },

    /// Execute SQL command or file
    Exec {
        /// SQL command to execute
        #[arg(long)]
        sql: Option<String>,

        /// SQL file to execute
        #[arg(long)]
        file: Option<String>,

        /// Database name
        #[arg(long, short = 'd')]
        database: String,

        /// Cluster name (optional - auto-selected)
        #[arg(long, short = 'c')]
        cluster: Option<String>,
    },

    /// Backup database
    Backup {
        /// Database name
        database: String,

        /// Output file path
        #[arg(long, short = 'o')]
        output: String,

        /// Cluster name (optional - auto-selected)
        #[arg(long, short = 'c')]
        cluster: Option<String>,
    },

    /// Restore database
    Restore {
        /// Database name
        database: String,

        /// Input file path
        #[arg(long, short = 'i')]
        input: String,

        /// Cluster name (optional - auto-selected)
        #[arg(long, short = 'c')]
        cluster: Option<String>,
    },

    /// Create a database user
    #[command(alias = "createuser")]
    CreateUser {
        /// Username
        username: String,

        /// Password (will prompt if not provided)
        #[arg(long, short = 'p')]
        password: Option<String>,

        /// Create as superuser
        #[arg(long)]
        superuser: bool,

        /// Cluster name (optional - auto-selected)
        #[arg(long, short = 'c')]
        cluster: Option<String>,
    },

    /// List all database users
    Users {
        /// Cluster name (optional - auto-selected)
        #[arg(long, short = 'c')]
        cluster: Option<String>,
    },

    /// Manage PostgreSQL extensions (build, install, enable)
    Extension {
        #[command(subcommand)]
        action: extension::ExtensionAction,
    },

    /// Connect to a database and start an interactive SQL REPL
    Use {
        /// Database name
        database: String,

        /// Cluster name (optional - auto-selected)
        #[arg(long, short = 'c')]
        cluster: Option<String>,
    },

    /// Backward compatibility commands (hidden from help)
    #[command(hide = true)]
    Server {
        #[command(subcommand)]
        action: server::ServerAction,
    },

    #[command(hide = true)]
    Db {
        #[command(subcommand)]
        action: postgres::DbAction,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => {
            // No command provided - show helpful status
            show_overview().await?;
        }
        Some(Commands::Install { version }) => {
            server::handle_action(server::ServerAction::Install { version }).await?;
        }
        Some(Commands::Versions) => {
            server::list_available_versions()?;
        }
        Some(Commands::Cache { action }) => {
            cache::handle_action(action)?;
        }
        Some(Commands::Init {
            name,
            version,
            port,
            username,
        }) => {
            handle_init(name, version, port, username).await?;
        }
        Some(Commands::Start { name }) => {
            let cluster = resolve_cluster(name)?;
            server::start_cluster(&cluster)?;
            save_context(&cluster)?;
        }
        Some(Commands::Stop { name }) => {
            let cluster = resolve_cluster(name)?;
            server::handle_action(server::ServerAction::Stop { name: cluster }).await?;
        }
        Some(Commands::Restart { name }) => {
            let cluster = resolve_cluster(name)?;
            server::handle_action(server::ServerAction::Restart {
                name: cluster.clone(),
            })
            .await?;
            save_context(&cluster)?;
        }
        Some(Commands::Reload { name }) => {
            let cluster = resolve_cluster(name)?;
            server::handle_action(server::ServerAction::Reload { name: cluster }).await?;
        }
        Some(Commands::Status { name }) => {
            if let Some(cluster) = name {
                server::handle_action(server::ServerAction::Status { name: cluster }).await?;
            } else {
                show_all_status().await?;
            }
        }
        Some(Commands::List) => {
            show_list().await?;
        }
        Some(Commands::Remove { name, force }) => {
            if !force {
                confirm_removal(&name)?;
            }
            server::handle_action(server::ServerAction::RemoveCluster { name }).await?;
        }
        Some(Commands::Logs {
            name,
            lines,
            follow,
        }) => {
            let cluster = resolve_cluster(name)?;
            server::handle_action(server::ServerAction::Logs {
                name: cluster,
                lines,
                follow,
            })
            .await?;
        }
        Some(Commands::CreateDb { name, cluster }) => {
            let cluster = resolve_cluster(cluster)?;
            server::ensure_cluster_running(&cluster)?;
            postgres::handle_action(postgres::DbAction::Create {
                cluster: cluster.clone(),
                database: name,
            })
            .await?;
            save_context(&cluster)?;
        }
        Some(Commands::DropDb {
            name,
            cluster,
            force,
        }) => {
            if !force {
                confirm_db_drop(&name)?;
            }
            let cluster = resolve_cluster(cluster)?;
            server::ensure_cluster_running(&cluster)?;
            postgres::handle_action(postgres::DbAction::Drop {
                cluster: cluster.clone(),
                database: name,
            })
            .await?;
            save_context(&cluster)?;
        }
        Some(Commands::Databases { cluster }) => {
            if let Some(cluster_name) = cluster {
                let cluster = resolve_cluster(Some(cluster_name))?;
                server::ensure_cluster_running(&cluster)?;
                postgres::handle_action(postgres::DbAction::List { cluster }).await?;
            } else {
                show_all_databases().await?;
            }
        }
        Some(Commands::Shell { database, cluster }) => {
            let cluster = resolve_cluster(cluster)?;
            server::ensure_cluster_running(&cluster)?;
            postgres::handle_action(postgres::DbAction::Shell {
                cluster: cluster.clone(),
                database,
            })
            .await?;
            save_context(&cluster)?;
        }
        Some(Commands::Exec {
            sql,
            file,
            database,
            cluster,
        }) => {
            let cluster = resolve_cluster(cluster)?;
            server::ensure_cluster_running(&cluster)?;
            postgres::handle_action(postgres::DbAction::Exec {
                cluster: cluster.clone(),
                database,
                file,
                sql,
            })
            .await?;
            save_context(&cluster)?;
        }
        Some(Commands::Backup {
            database,
            output,
            cluster,
        }) => {
            let cluster = resolve_cluster(cluster)?;
            server::ensure_cluster_running(&cluster)?;
            postgres::handle_action(postgres::DbAction::Backup {
                cluster: cluster.clone(),
                database,
                output,
            })
            .await?;
            save_context(&cluster)?;
        }
        Some(Commands::Restore {
            database,
            input,
            cluster,
        }) => {
            let cluster = resolve_cluster(cluster)?;
            server::ensure_cluster_running(&cluster)?;
            postgres::handle_action(postgres::DbAction::Restore {
                cluster: cluster.clone(),
                database,
                input,
            })
            .await?;
            save_context(&cluster)?;
        }
        Some(Commands::CreateUser {
            username,
            password,
            superuser,
            cluster,
        }) => {
            let cluster = resolve_cluster(cluster)?;
            server::ensure_cluster_running(&cluster)?;
            let password = if let Some(pwd) = password {
                Some(pwd)
            } else {
                prompt_password(&username)?
            };
            postgres::handle_action(postgres::DbAction::User {
                action: postgres::UserAction::Create {
                    cluster: cluster.clone(),
                    username,
                    password,
                    superuser,
                },
            })
            .await?;
            save_context(&cluster)?;
        }
        Some(Commands::Users { cluster }) => {
            let cluster = resolve_cluster(cluster)?;
            server::ensure_cluster_running(&cluster)?;
            postgres::handle_action(postgres::DbAction::User {
                action: postgres::UserAction::List { cluster },
            })
            .await?;
        }
        Some(Commands::Extension { action }) => {
            extension::handle_action(action)?;
        }
        Some(Commands::Use { database, cluster }) => {
            let cluster = resolve_cluster(cluster)?;
            server::ensure_cluster_running(&cluster)?;
            postgres::run_repl(&cluster, &database).await?;
            save_context(&cluster)?;
        }
        Some(Commands::Server { action }) => {
            server::handle_action(action).await?;
        }
        Some(Commands::Db { action }) => {
            postgres::handle_action(action).await?;
        }
    }

    Ok(())
}

fn resolve_cluster(name: Option<String>) -> Result<String> {
    if let Some(name) = name {
        // Explicit name provided - validate it exists
        let _ = server::load_cluster_metadata(&name)?;
        return Ok(name);
    }

    // Try to auto-select
    let clusters = server::get_all_clusters()?;

    if clusters.is_empty() {
        bail!(
            "No clusters found. Create one with: {}",
            "dbctl init <name>".green()
        );
    }

    if clusters.len() == 1 {
        // Only one cluster - use it
        let cluster_name = clusters[0].0.clone();
        return Ok(cluster_name);
    }

    // Multiple clusters - try context
    let context = Context::load()?;
    if let Some(last_cluster) = context.last_cluster {
        // Verify it still exists
        if clusters.iter().any(|(name, _)| name == &last_cluster) {
            return Ok(last_cluster);
        }
    }

    // Need to prompt or error
    bail!(
        "Multiple clusters found. Please specify one:\n{}",
        format_cluster_list(&clusters)
    );
}

fn save_context(cluster: &str) -> Result<()> {
    let mut context = Context::load()?;
    context.set_last_cluster(cluster.to_string())?;
    Ok(())
}

fn format_cluster_list(clusters: &[(String, server::ClusterMetadata)]) -> String {
    clusters
        .iter()
        .map(|(name, metadata)| {
            format!(
                "  {} - PostgreSQL {} on port {}",
                name.cyan(),
                metadata.version,
                metadata.port
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

async fn show_overview() -> Result<()> {
    let clusters = server::get_all_clusters()?;

    if clusters.is_empty() {
        println!("{}", "No clusters found.".yellow());
        println!("\nGet started:");
        println!("  {}", "dbctl install 16.3".green());
        println!("  {}", "dbctl init mydev".green());
        return Ok(());
    }

    println!("Clusters:\n");

    for (name, metadata) in &clusters {
        let is_running = server::is_cluster_running(name).unwrap_or(false);
        let status_symbol = if is_running {
            "●".green()
        } else {
            "○".red()
        };
        let status_text = if is_running {
            "running".green()
        } else {
            "stopped".red()
        };

        print!(
            "  {} {} - PostgreSQL {} on port {} - {}",
            status_symbol,
            name.cyan().bold(),
            metadata.version,
            metadata.port,
            status_text
        );

        // Show resource usage and stats for running clusters
        if is_running {
            if let Ok(Some(resources)) = server::get_cluster_resources(name) {
                println!(
                    "\n    CPU: {:.1}% | Memory: {} MB | Disk: {} MB",
                    resources.cpu_percent, resources.memory_mb, resources.disk_mb
                );
            } else {
                println!();
            }

            if let Ok(stats) = postgres::get_cluster_stats(name).await {
                println!(
                    "    Databases: {} | Users: {} | Tables: {}",
                    stats.database_count, stats.user_count, stats.table_count
                );
            }
        } else {
            println!();
        }
    }

    println!("\nConnection:");
    println!("  psql -h localhost -p <port> -U <username> -d <database>");

    Ok(())
}

async fn show_all_status() -> Result<()> {
    show_overview().await
}

async fn show_list() -> Result<()> {
    show_overview().await
}

async fn show_all_databases() -> Result<()> {
    let clusters = server::get_all_clusters()?;

    if clusters.is_empty() {
        println!("{}", "No clusters found.".yellow());
        return Ok(());
    }

    for (name, _) in &clusters {
        println!("\n{} {}:", "Cluster:".bold(), name.cyan().bold());
        if server::is_cluster_running(name).unwrap_or(false) {
            let _ = postgres::handle_action(postgres::DbAction::List {
                cluster: name.clone(),
            })
            .await;
        } else {
            println!("  {}", "(cluster stopped)".yellow());
        }
    }

    Ok(())
}

async fn handle_init(
    name: Option<String>,
    version: Option<String>,
    port: Option<u16>,
    username: Option<String>,
) -> Result<()> {
    use dialoguer::{Confirm, Input};

    // Get cluster name
    let name = if let Some(n) = name {
        n
    } else {
        Input::<String>::new()
            .with_prompt("Cluster name")
            .interact()?
    };

    // Get version - default to latest installed or prompt
    let version = if let Some(v) = version {
        v
    } else {
        let versions = get_installed_versions()?;
        if versions.is_empty() {
            bail!(
                "No PostgreSQL versions installed. Run: {}",
                "dbctl install 16.3".green()
            );
        } else if versions.len() == 1 {
            let v = versions[0].clone();
            println!("Using PostgreSQL {}", v.green());
            v
        } else {
            let selection = dialoguer::Select::new()
                .with_prompt("Select PostgreSQL version")
                .items(&versions)
                .default(0)
                .interact()?;
            versions[selection].clone()
        }
    };

    // Get port - automatically find an unused one
    let port = if let Some(p) = port {
        p
    } else {
        let found_port = server::find_unused_port()?;
        println!("Using available port: {}", found_port.to_string().green());
        found_port
    };

    // Get username
    let username = if let Some(u) = username {
        u
    } else {
        Input::<String>::new()
            .with_prompt("Superuser name")
            .default("postgres".to_string())
            .interact()?
    };

    // Create cluster
    server::handle_action(server::ServerAction::Init {
        version,
        name: name.clone(),
        port,
        username,
    })
    .await?;

    // Ask if they want to start it
    if Confirm::new()
        .with_prompt("Start the cluster now?")
        .default(true)
        .interact()?
    {
        server::start_cluster(&name)?;
    }

    Ok(())
}

fn get_installed_versions() -> Result<Vec<String>> {
    let versions_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
        .join(".dbctl")
        .join("versions");

    if !versions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut versions = Vec::new();
    for entry in std::fs::read_dir(&versions_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            versions.push(entry.file_name().to_string_lossy().to_string());
        }
    }

    // Sort versions
    versions.sort();
    versions.reverse(); // Latest first

    Ok(versions)
}

fn confirm_removal(cluster: &str) -> Result<()> {
    use dialoguer::Confirm;

    let confirmed = Confirm::new()
        .with_prompt(format!(
            "Are you sure you want to remove cluster '{}' and all its data?",
            cluster.red()
        ))
        .default(false)
        .interact()?;

    if !confirmed {
        bail!("Cancelled");
    }

    Ok(())
}

fn confirm_db_drop(database: &str) -> Result<()> {
    use dialoguer::Confirm;

    let confirmed = Confirm::new()
        .with_prompt(format!(
            "Are you sure you want to drop database '{}'?",
            database.red()
        ))
        .default(false)
        .interact()?;

    if !confirmed {
        bail!("Cancelled");
    }

    Ok(())
}

fn prompt_password(username: &str) -> Result<Option<String>> {
    use dialoguer::Password;

    let password = Password::new()
        .with_prompt(format!("Password for user '{}'", username))
        .allow_empty_password(true)
        .with_confirmation("Confirm password", "Passwords don't match")
        .interact()?;

    if password.is_empty() {
        Ok(None)
    } else {
        Ok(Some(password))
    }
}
