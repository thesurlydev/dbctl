// src/postgres.rs
use anyhow::{Result, bail};
use clap::{Subcommand, ValueEnum};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use crate::connection::{ConnectionSource, connect};

#[derive(Debug, Clone, ValueEnum)]
pub enum GrantTarget {
    Database,
    Schema,
    AllTables,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum RevokeTarget {
    Database,
    Schema,
    AllTables,
}

#[derive(Subcommand)]
pub enum DbAction {
    /// Create database
    Create {
        #[arg(long)]
        cluster: String,

        #[arg(long)]
        database: String,
    },

    /// Drop database
    Drop {
        #[arg(long)]
        cluster: String,

        #[arg(long)]
        database: String,
    },

    /// List databases
    List {
        #[arg(long)]
        cluster: String,
    },

    /// Open interactive psql shell
    Shell {
        #[arg(long)]
        cluster: String,

        #[arg(long)]
        database: Option<String>,
    },

    /// Execute SQL file or command
    Exec {
        #[arg(long)]
        cluster: String,

        #[arg(long)]
        database: String,

        #[arg(long, conflicts_with = "sql")]
        file: Option<String>,

        #[arg(long, conflicts_with = "file")]
        sql: Option<String>,
    },

    /// Backup database to SQL file
    Backup {
        #[arg(long)]
        cluster: String,

        #[arg(long)]
        database: String,

        #[arg(long)]
        output: String,
    },

    /// Restore database from SQL file
    Restore {
        #[arg(long)]
        cluster: String,

        #[arg(long)]
        database: String,

        #[arg(long)]
        input: String,
    },

    /// Show database information
    Info {
        #[arg(long)]
        cluster: String,

        #[arg(long)]
        database: String,
    },

    /// User management
    User {
        #[command(subcommand)]
        action: UserAction,
    },

    /// Grant privileges to a user
    Grant {
        #[arg(long)]
        cluster: String,

        #[arg(long)]
        database: String,

        #[arg(long)]
        user: String,

        #[arg(long)]
        on: GrantTarget,

        #[arg(long)]
        privileges: String,

        #[arg(long, default_value = "public")]
        schema: String,
    },

    /// Revoke privileges from a user
    Revoke {
        #[arg(long)]
        cluster: String,

        #[arg(long)]
        database: String,

        #[arg(long)]
        user: String,

        #[arg(long)]
        on: RevokeTarget,

        #[arg(long)]
        privileges: String,

        #[arg(long, default_value = "public")]
        schema: String,
    },

    /// Show user privileges on a database
    Privileges {
        #[arg(long)]
        cluster: String,

        #[arg(long)]
        database: String,

        #[arg(long)]
        user: String,
    },
}

#[derive(Subcommand)]
pub enum UserAction {
    /// Create a new database user
    Create {
        #[arg(long)]
        cluster: String,

        #[arg(long)]
        username: String,

        #[arg(long)]
        password: Option<String>,

        #[arg(long)]
        superuser: bool,
    },

    /// List database users
    List {
        #[arg(long)]
        cluster: String,
    },

    /// Drop a database user
    Drop {
        #[arg(long)]
        cluster: String,

        #[arg(long)]
        username: String,
    },
}

pub async fn handle_action(action: DbAction) -> Result<()> {
    match action {
        DbAction::Create { cluster, database } => {
            create_database(&cluster, &database).await?;
        }
        DbAction::Drop { cluster, database } => {
            drop_database(&cluster, &database).await?;
        }
        DbAction::List { cluster } => {
            print_databases(&cluster).await?;
        }
        DbAction::Shell { cluster, database } => {
            shell(&cluster, database.as_deref())?;
        }
        DbAction::Exec {
            cluster,
            database,
            file,
            sql,
        } => {
            exec(&cluster, &database, file.as_deref(), sql.as_deref())?;
        }
        DbAction::Backup {
            cluster,
            database,
            output,
        } => {
            backup(&cluster, &database, &output)?;
        }
        DbAction::Restore {
            cluster,
            database,
            input,
        } => {
            restore(&cluster, &database, &input)?;
        }
        DbAction::Info { cluster, database } => {
            info(&cluster, &database).await?;
        }
        DbAction::User { action } => match action {
            UserAction::Create {
                cluster,
                username,
                password,
                superuser,
            } => {
                create_user(&cluster, &username, password.as_deref(), superuser).await?;
            }
            UserAction::List { cluster } => {
                print_users(&cluster).await?;
            }
            UserAction::Drop { cluster, username } => {
                drop_user(&cluster, &username).await?;
            }
        },
        DbAction::Grant {
            cluster,
            database,
            user,
            on,
            privileges,
            schema,
        } => {
            grant_privileges(&cluster, &database, &user, &on, &privileges, &schema).await?;
        }
        DbAction::Revoke {
            cluster,
            database,
            user,
            on,
            privileges,
            schema,
        } => {
            revoke_privileges(&cluster, &database, &user, &on, &privileges, &schema).await?;
        }
        DbAction::Privileges {
            cluster,
            database,
            user,
        } => {
            show_privileges(&cluster, &database, &user).await?;
        }
    }
    Ok(())
}

/// Create database using a connection source
pub async fn create_database_with_source(
    source: &ConnectionSource,
    database: &str,
) -> Result<bool> {
    let (client, _handle) = connect(source, Some("postgres")).await?;

    // Check if exists
    let row = client
        .query_one("SELECT 1 FROM pg_database WHERE datname = $1", &[&database])
        .await;

    if row.is_ok() {
        return Ok(false); // Already exists
    }

    // Create database
    let query = format!("CREATE DATABASE {}", database);
    client.batch_execute(&query).await?;

    Ok(true) // Created successfully
}

pub async fn create_database(cluster: &str, database: &str) -> Result<()> {
    let source = ConnectionSource::from_cluster(cluster)?;
    let created = create_database_with_source(&source, database).await?;

    if created {
        println!("✓ Created database '{}' in cluster '{}'", database, cluster);
    } else {
        println!(
            "Database '{}' already exists in cluster '{}'",
            database, cluster
        );
    }

    Ok(())
}

/// Drop database using a connection source
pub async fn drop_database_with_source(source: &ConnectionSource, database: &str) -> Result<()> {
    let (client, _handle) = connect(source, Some("postgres")).await?;

    // Terminate connections
    let terminate_query = format!(
        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}' AND pid <> pg_backend_pid()",
        database
    );
    client.batch_execute(&terminate_query).await.ok();

    // Drop database
    let query = format!("DROP DATABASE IF EXISTS {}", database);
    client.batch_execute(&query).await?;

    Ok(())
}

pub async fn drop_database(cluster: &str, database: &str) -> Result<()> {
    let source = ConnectionSource::from_cluster(cluster)?;
    drop_database_with_source(&source, database).await?;

    println!(
        "✓ Dropped database '{}' from cluster '{}'",
        database, cluster
    );

    Ok(())
}

/// List databases using a connection source (works with both clusters and remote connections)
pub async fn list_databases_with_source(source: &ConnectionSource) -> Result<Vec<String>> {
    let (client, _handle) = connect(source, Some("postgres")).await?;

    let rows = client
        .query(
            "SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY datname",
            &[],
        )
        .await?;

    let databases: Vec<String> = rows.iter().map(|row| row.get(0)).collect();
    Ok(databases)
}

pub async fn list_databases(cluster: &str) -> Result<Vec<String>> {
    let source = ConnectionSource::from_cluster(cluster)?;
    list_databases_with_source(&source).await
}

async fn print_databases(cluster: &str) -> Result<()> {
    let databases = list_databases(cluster).await?;
    if databases.is_empty() {
        println!("No databases found in cluster '{}'", cluster);
    } else {
        println!("Databases in cluster '{}':", cluster);
        for db in databases {
            println!("  {}", db);
        }
    }
    Ok(())
}

fn shell(cluster: &str, database: Option<&str>) -> Result<()> {
    let metadata = crate::server::load_cluster_metadata(cluster)?;
    let dbname = database.unwrap_or("postgres");

    let version_dir = crate::server::get_version_dir(&metadata.version);
    let psql_path = version_dir.join("bin/psql");
    if !psql_path.exists() {
        bail!("psql not found at {}", psql_path.display());
    }

    let status = Command::new(&psql_path)
        .arg("-h")
        .arg("localhost")
        .arg("-p")
        .arg(metadata.port.to_string())
        .arg("-U")
        .arg(&metadata.username)
        .arg("-d")
        .arg(dbname)
        .status()?;

    if !status.success() {
        bail!("psql exited with status: {}", status);
    }

    Ok(())
}

pub fn exec_sql(cluster: &str, database: &str, sql: &str) -> Result<String> {
    let metadata = crate::server::load_cluster_metadata(cluster)?;

    let version_dir = crate::server::get_version_dir(&metadata.version);
    let psql_path = version_dir.join("bin/psql");
    if !psql_path.exists() {
        bail!("psql not found at {}", psql_path.display());
    }

    let output = Command::new(&psql_path)
        .arg("-h")
        .arg("localhost")
        .arg("-p")
        .arg(metadata.port.to_string())
        .arg("-U")
        .arg(&metadata.username)
        .arg("-d")
        .arg(database)
        .arg("-c")
        .arg(sql)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("SQL execution failed: {}", stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn exec(cluster: &str, database: &str, file: Option<&str>, sql: Option<&str>) -> Result<()> {
    let metadata = crate::server::load_cluster_metadata(cluster)?;

    let version_dir = crate::server::get_version_dir(&metadata.version);
    let psql_path = version_dir.join("bin/psql");
    if !psql_path.exists() {
        bail!("psql not found at {}", psql_path.display());
    }

    let mut cmd = Command::new(&psql_path);
    cmd.arg("-h")
        .arg("localhost")
        .arg("-p")
        .arg(metadata.port.to_string())
        .arg("-U")
        .arg(&metadata.username)
        .arg("-d")
        .arg(database);

    if let Some(file_path) = file {
        if !Path::new(file_path).exists() {
            bail!("SQL file not found: {}", file_path);
        }
        cmd.arg("-f").arg(file_path);
    } else if let Some(sql_cmd) = sql {
        cmd.arg("-c").arg(sql_cmd);
    } else {
        bail!("Either --file or --sql must be specified");
    }

    let output = cmd.output()?;

    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    if !output.status.success() {
        bail!("Command failed with status: {}", output.status);
    }

    Ok(())
}

pub fn backup(cluster: &str, database: &str, output: &str) -> Result<()> {
    let metadata = crate::server::load_cluster_metadata(cluster)?;

    let version_dir = crate::server::get_version_dir(&metadata.version);
    let pg_dump_path = version_dir.join("bin/pg_dump");
    if !pg_dump_path.exists() {
        bail!("pg_dump not found at {}", pg_dump_path.display());
    }

    println!("Backing up database '{}' to '{}'...", database, output);

    let status = Command::new(&pg_dump_path)
        .arg("-h")
        .arg("localhost")
        .arg("-p")
        .arg(metadata.port.to_string())
        .arg("-U")
        .arg(&metadata.username)
        .arg("-d")
        .arg(database)
        .arg("-f")
        .arg(output)
        .arg("--no-password")
        .status()?;

    if !status.success() {
        bail!("pg_dump failed with status: {}", status);
    }

    println!("✓ Backup completed: {}", output);

    Ok(())
}

pub fn restore(cluster: &str, database: &str, input: &str) -> Result<()> {
    let metadata = crate::server::load_cluster_metadata(cluster)?;

    if !Path::new(input).exists() {
        bail!("Input file not found: {}", input);
    }

    let version_dir = crate::server::get_version_dir(&metadata.version);
    let psql_path = version_dir.join("bin/psql");
    if !psql_path.exists() {
        bail!("psql not found at {}", psql_path.display());
    }

    println!("Restoring database '{}' from '{}'...", database, input);

    let sql_content = fs::read_to_string(input)?;

    let mut child = Command::new(&psql_path)
        .arg("-h")
        .arg("localhost")
        .arg("-p")
        .arg(metadata.port.to_string())
        .arg("-U")
        .arg(&metadata.username)
        .arg("-d")
        .arg(database)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(sql_content.as_bytes())?;
    }

    let output = child.wait_with_output()?;

    if !output.status.success() {
        if !output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }
        bail!("Restore failed with status: {}", output.status);
    }

    println!("✓ Restore completed");

    Ok(())
}

async fn info(cluster: &str, database: &str) -> Result<()> {
    let metadata = crate::server::load_cluster_metadata(cluster)?;

    let conn_str = format!(
        "host=localhost port={} user={} dbname={}",
        metadata.port, metadata.username, database
    );

    let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });

    // Get database size
    let size_row = client
        .query_one("SELECT pg_size_pretty(pg_database_size($1))", &[&database])
        .await?;
    let size: String = size_row.get(0);

    // Get connection count
    let conn_row = client
        .query_one(
            "SELECT count(*) FROM pg_stat_activity WHERE datname = $1",
            &[&database],
        )
        .await?;
    let connections: i64 = conn_row.get(0);

    // Get table count
    let table_row = client
        .query_one(
            "SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public' AND table_type = 'BASE TABLE'",
            &[],
        )
        .await?;
    let tables: i64 = table_row.get(0);

    // Get extensions
    let ext_rows = client
        .query("SELECT extname FROM pg_extension ORDER BY extname", &[])
        .await?;

    println!("Database: {}", database);
    println!("Cluster: {}", cluster);
    println!("Size: {}", size);
    println!("Connections: {}", connections);
    println!("Tables: {}", tables);
    println!("Extensions:");
    if ext_rows.is_empty() {
        println!("  (none)");
    } else {
        for row in ext_rows {
            let extname: String = row.get(0);
            println!("  {}", extname);
        }
    }

    Ok(())
}

pub async fn create_user(
    cluster: &str,
    username: &str,
    password: Option<&str>,
    superuser: bool,
) -> Result<()> {
    let metadata = crate::server::load_cluster_metadata(cluster)?;

    let conn_str = format!(
        "host=localhost port={} user={} dbname=postgres",
        metadata.port, metadata.username
    );

    let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });

    // Check if user exists
    let row = client
        .query_one("SELECT 1 FROM pg_roles WHERE rolname = $1", &[&username])
        .await;

    if row.is_ok() {
        println!(
            "User '{}' already exists in cluster '{}'",
            username, cluster
        );
        return Ok(());
    }

    // Build CREATE USER query
    let mut query = format!("CREATE USER {}", username);
    if let Some(pwd) = password {
        query.push_str(&format!(" PASSWORD '{}'", pwd));
    }
    if superuser {
        query.push_str(" SUPERUSER");
    }

    client.batch_execute(&query).await?;

    println!("✓ Created user '{}' in cluster '{}'", username, cluster);

    Ok(())
}

/// List users using a connection source
pub async fn list_users_with_source(source: &ConnectionSource) -> Result<Vec<UserInfo>> {
    let (client, _handle) = connect(source, Some("postgres")).await?;

    let rows = client
        .query(
            "SELECT rolname, rolsuper, rolcreatedb, rolcreaterole FROM pg_roles WHERE rolname NOT LIKE 'pg_%' ORDER BY rolname",
            &[],
        )
        .await?;

    let users: Vec<UserInfo> = rows
        .iter()
        .map(|row| UserInfo {
            name: row.get(0),
            superuser: row.get(1),
            create_db: row.get(2),
            create_role: row.get(3),
        })
        .collect();

    Ok(users)
}

pub async fn list_users(cluster: &str) -> Result<Vec<UserInfo>> {
    let source = ConnectionSource::from_cluster(cluster)?;
    list_users_with_source(&source).await
}

async fn print_users(cluster: &str) -> Result<()> {
    let users = list_users(cluster).await?;
    if users.is_empty() {
        println!("No users found in cluster '{}'", cluster);
    } else {
        println!("Users in cluster '{}':", cluster);
        println!("  Name               Superuser  CreateDB  CreateRole");
        println!("  ------------------ ---------- --------- ----------");
        for user in users {
            println!(
                "  {:<18} {:<10} {:<9} {}",
                user.name,
                if user.superuser { "yes" } else { "no" },
                if user.create_db { "yes" } else { "no" },
                if user.create_role { "yes" } else { "no" }
            );
        }
    }
    Ok(())
}

async fn drop_user(cluster: &str, username: &str) -> Result<()> {
    let metadata = crate::server::load_cluster_metadata(cluster)?;

    let conn_str = format!(
        "host=localhost port={} user={} dbname=postgres",
        metadata.port, metadata.username
    );

    let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });

    // Drop user
    let query = format!("DROP USER IF EXISTS {}", username);
    client.batch_execute(&query).await?;

    println!("✓ Dropped user '{}' from cluster '{}'", username, cluster);

    Ok(())
}

async fn grant_privileges(
    cluster: &str,
    database: &str,
    user: &str,
    on: &GrantTarget,
    privileges: &str,
    schema: &str,
) -> Result<()> {
    let metadata = crate::server::load_cluster_metadata(cluster)?;

    let conn_str = format!(
        "host=localhost port={} user={} dbname={}",
        metadata.port, metadata.username, database
    );

    let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });

    // Build GRANT query based on target
    let query = match on {
        GrantTarget::Database => {
            format!("GRANT {} ON DATABASE {} TO {}", privileges, database, user)
        }
        GrantTarget::Schema => {
            format!("GRANT {} ON SCHEMA {} TO {}", privileges, schema, user)
        }
        GrantTarget::AllTables => {
            format!(
                "GRANT {} ON ALL TABLES IN SCHEMA {} TO {}",
                privileges, schema, user
            )
        }
    };

    client.batch_execute(&query).await?;

    let target_desc = match on {
        GrantTarget::Database => format!("database '{}'", database),
        GrantTarget::Schema => format!("schema '{}'", schema),
        GrantTarget::AllTables => format!("all tables in schema '{}'", schema),
    };

    println!(
        "✓ Granted {} on {} to user '{}'",
        privileges, target_desc, user
    );

    Ok(())
}

async fn revoke_privileges(
    cluster: &str,
    database: &str,
    user: &str,
    on: &RevokeTarget,
    privileges: &str,
    schema: &str,
) -> Result<()> {
    let metadata = crate::server::load_cluster_metadata(cluster)?;

    let conn_str = format!(
        "host=localhost port={} user={} dbname={}",
        metadata.port, metadata.username, database
    );

    let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });

    // Build REVOKE query based on target
    let query = match on {
        RevokeTarget::Database => {
            format!(
                "REVOKE {} ON DATABASE {} FROM {}",
                privileges, database, user
            )
        }
        RevokeTarget::Schema => {
            format!("REVOKE {} ON SCHEMA {} FROM {}", privileges, schema, user)
        }
        RevokeTarget::AllTables => {
            format!(
                "REVOKE {} ON ALL TABLES IN SCHEMA {} FROM {}",
                privileges, schema, user
            )
        }
    };

    client.batch_execute(&query).await?;

    let target_desc = match on {
        RevokeTarget::Database => format!("database '{}'", database),
        RevokeTarget::Schema => format!("schema '{}'", schema),
        RevokeTarget::AllTables => format!("all tables in schema '{}'", schema),
    };

    println!(
        "✓ Revoked {} on {} from user '{}'",
        privileges, target_desc, user
    );

    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct UserInfo {
    pub name: String,
    pub superuser: bool,
    pub create_db: bool,
    pub create_role: bool,
}

#[derive(Debug, Clone)]
pub struct ClusterStats {
    pub database_count: i64,
    pub user_count: i64,
    pub table_count: i64,
}

// ============================================
// Table Browser Structures
// ============================================

/// Information about a database schema
#[derive(Debug, Clone)]
pub struct SchemaInfo {
    pub name: String,
    pub owner: String,
}

/// Basic table information for listing
#[derive(Debug, Clone)]
pub struct TableInfo {
    pub name: String,
    pub table_type: String, // "BASE TABLE", "VIEW", etc.
    pub row_estimate: i64,
    pub size_bytes: i64,
}

/// Column definition
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub column_default: Option<String>,
    pub ordinal_position: i32,
    pub is_primary_key: bool,
}

/// Constraint information
#[derive(Debug, Clone)]
pub struct ConstraintInfo {
    pub name: String,
    pub constraint_type: String, // "PRIMARY KEY", "FOREIGN KEY", "UNIQUE", "CHECK"
    pub columns: Vec<String>,
    pub foreign_table: Option<String>,
    pub foreign_columns: Option<Vec<String>>,
}

/// Index information
#[derive(Debug, Clone)]
pub struct IndexInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
    pub is_primary: bool,
    pub index_type: String, // "btree", "hash", etc.
}

/// Complete table structure
#[derive(Debug, Clone)]
pub struct TableStructure {
    pub schema: String,
    pub table: String,
    pub columns: Vec<ColumnInfo>,
    pub constraints: Vec<ConstraintInfo>,
    pub indexes: Vec<IndexInfo>,
    pub primary_key_columns: Vec<String>,
}

/// Single row of data (column name -> value as string)
pub type TableRow = std::collections::HashMap<String, Option<String>>;

/// Result of a paginated data query
#[derive(Debug, Clone)]
pub struct TableDataResult {
    pub columns: Vec<String>,
    pub rows: Vec<TableRow>,
    pub total_rows: i64,
    pub page: usize,
    pub page_size: usize,
}

/// Get cluster stats using a connection source
pub async fn get_cluster_stats_with_source(source: &ConnectionSource) -> Result<ClusterStats> {
    let (client, _handle) = connect(source, Some("postgres")).await?;

    let db_count_row = client
        .query_one(
            "SELECT count(*) FROM pg_database WHERE datistemplate = false",
            &[],
        )
        .await?;
    let database_count: i64 = db_count_row.get(0);

    let user_count_row = client
        .query_one(
            "SELECT count(*) FROM pg_roles WHERE rolname NOT LIKE 'pg_%'",
            &[],
        )
        .await?;
    let user_count: i64 = user_count_row.get(0);

    let databases = client
        .query(
            "SELECT datname FROM pg_database WHERE datistemplate = false",
            &[],
        )
        .await?;

    let mut table_count = 0i64;
    for db_row in databases {
        let dbname: String = db_row.get(0);

        // Connect to each database to count tables
        if let Ok((db_client, _db_handle)) = connect(source, Some(&dbname)).await {
            if let Ok(table_row) = db_client
                .query_one(
                    "SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public' AND table_type = 'BASE TABLE'",
                    &[],
                )
                .await
            {
                let count: i64 = table_row.get(0);
                table_count += count;
            }
        }
    }

    Ok(ClusterStats {
        database_count,
        user_count,
        table_count,
    })
}

pub async fn get_cluster_stats(cluster: &str) -> Result<ClusterStats> {
    let source = ConnectionSource::from_cluster(cluster)?;
    get_cluster_stats_with_source(&source).await
}

async fn show_privileges(cluster: &str, database: &str, user: &str) -> Result<()> {
    let metadata = crate::server::load_cluster_metadata(cluster)?;

    let conn_str = format!(
        "host=localhost port={} user={} dbname={}",
        metadata.port, metadata.username, database
    );

    let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });

    println!("Privileges for user '{}' on database '{}':", user, database);
    println!();

    // Database privileges
    let db_privs = client
        .query(
            "SELECT has_database_privilege($1, $2, 'CONNECT') as connect,
                    has_database_privilege($1, $2, 'CREATE') as create,
                    has_database_privilege($1, $2, 'TEMPORARY') as temporary",
            &[&user, &database],
        )
        .await?;

    if let Some(row) = db_privs.first() {
        let connect: bool = row.get(0);
        let create: bool = row.get(1);
        let temporary: bool = row.get(2);

        println!("Database Privileges:");
        println!("  CONNECT:   {}", if connect { "YES" } else { "NO" });
        println!("  CREATE:    {}", if create { "YES" } else { "NO" });
        println!("  TEMPORARY: {}", if temporary { "YES" } else { "NO" });
        println!();
    }

    // Schema privileges (public schema)
    let schema_privs = client
        .query(
            "SELECT has_schema_privilege($1, 'public', 'USAGE') as usage,
                    has_schema_privilege($1, 'public', 'CREATE') as create",
            &[&user],
        )
        .await?;

    if let Some(row) = schema_privs.first() {
        let usage: bool = row.get(0);
        let create: bool = row.get(1);

        println!("Schema Privileges (public):");
        println!("  USAGE:  {}", if usage { "YES" } else { "NO" });
        println!("  CREATE: {}", if create { "YES" } else { "NO" });
        println!();
    }

    // Table privileges on public schema
    let table_rows = client
        .query(
            "SELECT table_name,
                    has_table_privilege($1, schemaname || '.' || tablename, 'SELECT') as select,
                    has_table_privilege($1, schemaname || '.' || tablename, 'INSERT') as insert,
                    has_table_privilege($1, schemaname || '.' || tablename, 'UPDATE') as update,
                    has_table_privilege($1, schemaname || '.' || tablename, 'DELETE') as delete
             FROM pg_tables
             WHERE schemaname = 'public'
             ORDER BY table_name
             LIMIT 10",
            &[&user],
        )
        .await?;

    if !table_rows.is_empty() {
        println!("Table Privileges (public schema, up to 10 tables):");
        println!("  Table                SELECT INSERT UPDATE DELETE");
        println!("  -------------------- ------ ------ ------ ------");
        for row in table_rows {
            let table_name: String = row.get(0);
            let select: bool = row.get(1);
            let insert: bool = row.get(2);
            let update: bool = row.get(3);
            let delete: bool = row.get(4);

            println!(
                "  {:<20} {:<6} {:<6} {:<6} {}",
                table_name,
                if select { "YES" } else { "NO" },
                if insert { "YES" } else { "NO" },
                if update { "YES" } else { "NO" },
                if delete { "YES" } else { "NO" }
            );
        }
    } else {
        println!("Table Privileges: No tables found in public schema");
    }

    Ok(())
}

// ============================================
// Table Browser Functions
// ============================================

/// List all schemas using a connection source
pub async fn list_schemas_with_source(
    source: &ConnectionSource,
    database: &str,
) -> Result<Vec<SchemaInfo>> {
    let (client, _handle) = connect(source, Some(database)).await?;

    let rows = client
        .query(
            "SELECT schema_name, COALESCE(schema_owner, '') as schema_owner
             FROM information_schema.schemata
             WHERE schema_name NOT IN ('pg_catalog', 'information_schema', 'pg_toast')
               AND schema_name NOT LIKE 'pg_temp_%'
               AND schema_name NOT LIKE 'pg_toast_temp_%'
             ORDER BY schema_name",
            &[],
        )
        .await?;

    Ok(rows
        .iter()
        .map(|row| SchemaInfo {
            name: row.get(0),
            owner: row.get(1),
        })
        .collect())
}

/// List all schemas in a database
pub async fn list_schemas(cluster: &str, database: &str) -> Result<Vec<SchemaInfo>> {
    let source = ConnectionSource::from_cluster(cluster)?;
    list_schemas_with_source(&source, database).await
}

/// List all tables using a connection source
pub async fn list_tables_with_source(
    source: &ConnectionSource,
    database: &str,
    schema: &str,
) -> Result<Vec<TableInfo>> {
    let (client, _handle) = connect(source, Some(database)).await?;

    let rows = client
        .query(
            "SELECT
                t.table_name,
                t.table_type,
                COALESCE(c.reltuples::bigint, 0) as row_estimate,
                COALESCE(pg_total_relation_size(c.oid), 0) as size_bytes
             FROM information_schema.tables t
             LEFT JOIN pg_class c ON c.relname = t.table_name
             LEFT JOIN pg_namespace n ON n.oid = c.relnamespace AND n.nspname = t.table_schema
             WHERE t.table_schema = $1
             ORDER BY t.table_name",
            &[&schema],
        )
        .await?;

    Ok(rows
        .iter()
        .map(|row| TableInfo {
            name: row.get(0),
            table_type: row.get(1),
            row_estimate: row.get(2),
            size_bytes: row.get(3),
        })
        .collect())
}

/// List all tables in a schema
pub async fn list_tables(cluster: &str, database: &str, schema: &str) -> Result<Vec<TableInfo>> {
    let source = ConnectionSource::from_cluster(cluster)?;
    list_tables_with_source(&source, database, schema).await
}

/// Get table structure using a connection source
pub async fn get_table_structure_with_source(
    source: &ConnectionSource,
    database: &str,
    schema: &str,
    table: &str,
) -> Result<TableStructure> {
    let (client, _handle) = connect(source, Some(database)).await?;

    // Get primary key columns first
    let pk_rows = client
        .query(
            "SELECT a.attname
             FROM pg_index i
             JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
             JOIN pg_class c ON c.oid = i.indrelid
             JOIN pg_namespace n ON n.oid = c.relnamespace
             WHERE i.indisprimary
               AND n.nspname = $1
               AND c.relname = $2",
            &[&schema, &table],
        )
        .await?;

    let pk_columns: std::collections::HashSet<String> =
        pk_rows.iter().map(|r| r.get::<_, String>(0)).collect();
    let primary_key_columns: Vec<String> = pk_columns.iter().cloned().collect();

    // Get columns
    let col_rows = client
        .query(
            "SELECT
                column_name,
                CASE
                    WHEN data_type = 'character varying' THEN
                        CASE WHEN character_maximum_length IS NOT NULL
                            THEN 'varchar(' || character_maximum_length || ')'
                            ELSE 'varchar'
                        END
                    WHEN data_type = 'character' THEN
                        CASE WHEN character_maximum_length IS NOT NULL
                            THEN 'char(' || character_maximum_length || ')'
                            ELSE 'char'
                        END
                    WHEN data_type = 'numeric' THEN
                        CASE WHEN numeric_precision IS NOT NULL AND numeric_scale IS NOT NULL
                            THEN 'numeric(' || numeric_precision || ',' || numeric_scale || ')'
                            WHEN numeric_precision IS NOT NULL
                            THEN 'numeric(' || numeric_precision || ')'
                            ELSE 'numeric'
                        END
                    ELSE data_type
                END as data_type,
                is_nullable = 'YES' as is_nullable,
                column_default,
                ordinal_position::int4
             FROM information_schema.columns
             WHERE table_schema = $1 AND table_name = $2
             ORDER BY ordinal_position",
            &[&schema, &table],
        )
        .await?;

    let columns: Vec<ColumnInfo> = col_rows
        .iter()
        .map(|row| {
            let name: String = row.get(0);
            ColumnInfo {
                is_primary_key: pk_columns.contains(&name),
                name,
                data_type: row.get(1),
                is_nullable: row.get(2),
                column_default: row.get(3),
                ordinal_position: row.get(4),
            }
        })
        .collect();

    // Get constraints
    let constraint_rows = client
        .query(
            "SELECT
                tc.constraint_name,
                tc.constraint_type,
                COALESCE(array_agg(DISTINCT kcu.column_name::text), ARRAY[]::text[]) as columns,
                ccu.table_name as foreign_table,
                array_agg(DISTINCT ccu.column_name::text) FILTER (WHERE ccu.column_name IS NOT NULL AND tc.constraint_type = 'FOREIGN KEY') as foreign_columns
             FROM information_schema.table_constraints tc
             JOIN information_schema.key_column_usage kcu
               ON tc.constraint_name = kcu.constraint_name
               AND tc.table_schema = kcu.table_schema
             LEFT JOIN information_schema.constraint_column_usage ccu
               ON tc.constraint_name = ccu.constraint_name
               AND tc.table_schema = ccu.table_schema
               AND tc.constraint_type = 'FOREIGN KEY'
             WHERE tc.table_schema = $1 AND tc.table_name = $2
             GROUP BY tc.constraint_name, tc.constraint_type, ccu.table_name",
            &[&schema, &table],
        )
        .await?;

    let constraints: Vec<ConstraintInfo> = constraint_rows
        .iter()
        .map(|row| ConstraintInfo {
            name: row.get(0),
            constraint_type: row.get(1),
            columns: row.get(2),
            foreign_table: row.get(3),
            foreign_columns: row.get(4),
        })
        .collect();

    // Get indexes
    let index_rows = client
        .query(
            "SELECT
                i.relname as index_name,
                array_agg(a.attname ORDER BY array_position(ix.indkey, a.attnum)) as columns,
                ix.indisunique,
                ix.indisprimary,
                am.amname as index_type
             FROM pg_index ix
             JOIN pg_class i ON i.oid = ix.indexrelid
             JOIN pg_class t ON t.oid = ix.indrelid
             JOIN pg_namespace n ON n.oid = t.relnamespace
             JOIN pg_am am ON am.oid = i.relam
             JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey)
             WHERE n.nspname = $1 AND t.relname = $2
             GROUP BY i.relname, ix.indisunique, ix.indisprimary, am.amname",
            &[&schema, &table],
        )
        .await?;

    let indexes: Vec<IndexInfo> = index_rows
        .iter()
        .map(|row| IndexInfo {
            name: row.get(0),
            columns: row.get(1),
            is_unique: row.get(2),
            is_primary: row.get(3),
            index_type: row.get(4),
        })
        .collect();

    Ok(TableStructure {
        schema: schema.to_string(),
        table: table.to_string(),
        columns,
        constraints,
        indexes,
        primary_key_columns,
    })
}

/// Get complete table structure (columns, constraints, indexes)
pub async fn get_table_structure(
    cluster: &str,
    database: &str,
    schema: &str,
    table: &str,
) -> Result<TableStructure> {
    let source = ConnectionSource::from_cluster(cluster)?;
    get_table_structure_with_source(&source, database, schema, table).await
}

/// Get paginated table data using a connection source
pub async fn get_table_data_with_source(
    source: &ConnectionSource,
    database: &str,
    schema: &str,
    table: &str,
    page: usize,
    page_size: usize,
) -> Result<TableDataResult> {
    let (client, _handle) = connect(source, Some(database)).await?;

    // Get total row count
    let count_query = format!(
        "SELECT count(*) FROM \"{}\".\"{}\"",
        schema.replace('"', "\"\""),
        table.replace('"', "\"\"")
    );
    let count_row = client.query_one(&count_query, &[]).await?;
    let total_rows: i64 = count_row.get(0);

    // Get column names and types
    let col_rows = client
        .query(
            "SELECT column_name, data_type
             FROM information_schema.columns
             WHERE table_schema = $1 AND table_name = $2
             ORDER BY ordinal_position",
            &[&schema, &table],
        )
        .await?;
    let columns: Vec<String> = col_rows.iter().map(|r| r.get(0)).collect();

    // Get paginated data
    let offset = page * page_size;
    let data_query = format!(
        "SELECT * FROM \"{}\".\"{}\" LIMIT {} OFFSET {}",
        schema.replace('"', "\"\""),
        table.replace('"', "\"\""),
        page_size,
        offset
    );
    let data_rows = client.query(&data_query, &[]).await?;

    let rows: Vec<TableRow> = data_rows
        .iter()
        .map(|row| {
            let mut map = std::collections::HashMap::new();
            for (i, col) in columns.iter().enumerate() {
                let value = convert_pg_value_to_string(row, i);
                map.insert(col.clone(), value);
            }
            map
        })
        .collect();

    Ok(TableDataResult {
        columns,
        rows,
        total_rows,
        page,
        page_size,
    })
}

/// Get paginated table data
pub async fn get_table_data(
    cluster: &str,
    database: &str,
    schema: &str,
    table: &str,
    page: usize,
    page_size: usize,
) -> Result<TableDataResult> {
    let source = ConnectionSource::from_cluster(cluster)?;
    get_table_data_with_source(&source, database, schema, table, page, page_size).await
}

/// Helper to convert PostgreSQL values to strings
fn convert_pg_value_to_string(row: &tokio_postgres::Row, idx: usize) -> Option<String> {
    // Try common types in order of likelihood
    if let Ok(v) = row.try_get::<_, Option<String>>(idx) {
        return v;
    }
    if let Ok(v) = row.try_get::<_, Option<i32>>(idx) {
        return v.map(|n| n.to_string());
    }
    if let Ok(v) = row.try_get::<_, Option<i64>>(idx) {
        return v.map(|n| n.to_string());
    }
    if let Ok(v) = row.try_get::<_, Option<i16>>(idx) {
        return v.map(|n| n.to_string());
    }
    if let Ok(v) = row.try_get::<_, Option<f32>>(idx) {
        return v.map(|n| n.to_string());
    }
    if let Ok(v) = row.try_get::<_, Option<f64>>(idx) {
        return v.map(|n| n.to_string());
    }
    if let Ok(v) = row.try_get::<_, Option<bool>>(idx) {
        return v.map(|b| b.to_string());
    }
    // Fallback for unsupported types
    Some("[unsupported type]".to_string())
}

/// Insert a new row using a connection source
pub async fn insert_row_with_source(
    source: &ConnectionSource,
    database: &str,
    schema: &str,
    table: &str,
    values: &std::collections::HashMap<String, String>,
) -> Result<()> {
    let (client, _handle) = connect(source, Some(database)).await?;

    let columns: Vec<&str> = values.keys().map(|s| s.as_str()).collect();

    // Quote identifiers properly
    let quoted_columns: Vec<String> = columns
        .iter()
        .map(|c| format!("\"{}\"", c.replace('"', "\"\"")))
        .collect();

    // Escape values and format as SQL literals
    // This allows PostgreSQL to handle type coercion during INSERT
    let escaped_values: Vec<String> = columns
        .iter()
        .map(|col| {
            let val = &values[*col];
            if val.is_empty() {
                "NULL".to_string()
            } else {
                // Escape single quotes by doubling them
                format!("'{}'", val.replace('\'', "''"))
            }
        })
        .collect();

    let query = format!(
        "INSERT INTO \"{}\".\"{}\" ({}) VALUES ({})",
        schema.replace('"', "\"\""),
        table.replace('"', "\"\""),
        quoted_columns.join(", "),
        escaped_values.join(", ")
    );

    client.execute(&query, &[]).await?;
    Ok(())
}

/// Insert a new row into a table
pub async fn insert_row(
    cluster: &str,
    database: &str,
    schema: &str,
    table: &str,
    values: &std::collections::HashMap<String, String>,
) -> Result<()> {
    let source = ConnectionSource::from_cluster(cluster)?;
    insert_row_with_source(&source, database, schema, table, values).await
}

/// Update a specific cell using a connection source
pub async fn update_cell_with_source(
    source: &ConnectionSource,
    database: &str,
    schema: &str,
    table: &str,
    pk_column: &str,
    pk_value: &str,
    column: &str,
    new_value: Option<&str>,
) -> Result<()> {
    let (client, _handle) = connect(source, Some(database)).await?;

    // Use SQL literals for the new value to allow PostgreSQL type coercion
    // Use parameterized query only for pk_value (cast to text for comparison)
    let escaped_pk = pk_value.replace('\'', "''");
    let query = match new_value {
        Some(val) => {
            let escaped_val = val.replace('\'', "''");
            format!(
                "UPDATE \"{}\".\"{}\" SET \"{}\" = '{}' WHERE \"{}\"::text = '{}'",
                schema.replace('"', "\"\""),
                table.replace('"', "\"\""),
                column.replace('"', "\"\""),
                escaped_val,
                pk_column.replace('"', "\"\""),
                escaped_pk
            )
        }
        None => format!(
            "UPDATE \"{}\".\"{}\" SET \"{}\" = NULL WHERE \"{}\"::text = '{}'",
            schema.replace('"', "\"\""),
            table.replace('"', "\"\""),
            column.replace('"', "\"\""),
            pk_column.replace('"', "\"\""),
            escaped_pk
        ),
    };

    client.execute(&query, &[]).await?;
    Ok(())
}

/// Update a specific cell in a table
pub async fn update_cell(
    cluster: &str,
    database: &str,
    schema: &str,
    table: &str,
    pk_column: &str,
    pk_value: &str,
    column: &str,
    new_value: Option<&str>,
) -> Result<()> {
    let source = ConnectionSource::from_cluster(cluster)?;
    update_cell_with_source(
        &source, database, schema, table, pk_column, pk_value, column, new_value,
    )
    .await
}

/// Delete a row using a connection source
pub async fn delete_row_with_source(
    source: &ConnectionSource,
    database: &str,
    schema: &str,
    table: &str,
    pk_column: &str,
    pk_value: &str,
) -> Result<()> {
    let (client, _handle) = connect(source, Some(database)).await?;

    let query = format!(
        "DELETE FROM \"{}\".\"{}\" WHERE \"{}\"::text = $1",
        schema.replace('"', "\"\""),
        table.replace('"', "\"\""),
        pk_column.replace('"', "\"\"")
    );

    client.execute(&query, &[&pk_value]).await?;
    Ok(())
}

/// Delete a row from a table
pub async fn delete_row(
    cluster: &str,
    database: &str,
    schema: &str,
    table: &str,
    pk_column: &str,
    pk_value: &str,
) -> Result<()> {
    let source = ConnectionSource::from_cluster(cluster)?;
    delete_row_with_source(&source, database, schema, table, pk_column, pk_value).await
}

/// Execute migration statements in a transaction
pub async fn execute_migration(cluster: &str, database: &str, statements: &[String]) -> Result<()> {
    let source = ConnectionSource::from_cluster(cluster)?;
    execute_migration_with_source(&source, database, statements).await
}

/// Execute migration statements in a transaction (with connection source)
pub async fn execute_migration_with_source(
    source: &ConnectionSource,
    database: &str,
    statements: &[String],
) -> Result<()> {
    if statements.is_empty() {
        return Ok(());
    }

    let (client, _handle) = connect(source, Some(database)).await?;

    // Start transaction
    client.batch_execute("BEGIN;").await?;

    // Execute each statement
    for stmt in statements {
        if let Err(e) = client.batch_execute(stmt).await {
            // Rollback on error
            client.batch_execute("ROLLBACK;").await.ok();
            bail!("Migration failed at:\n{}\n\nError: {}", stmt, e);
        }
    }

    // Commit transaction
    client.batch_execute("COMMIT;").await?;

    Ok(())
}

/// Export table data to CSV format
pub fn export_to_csv(data: &TableDataResult) -> String {
    let mut output = String::new();

    // Header
    output.push_str(&data.columns.join(","));
    output.push('\n');

    // Rows
    for row in &data.rows {
        let values: Vec<String> = data
            .columns
            .iter()
            .map(|col| match row.get(col).and_then(|v| v.as_ref()) {
                Some(val) => {
                    // Escape CSV values
                    if val.contains(',') || val.contains('"') || val.contains('\n') {
                        format!("\"{}\"", val.replace('"', "\"\""))
                    } else {
                        val.clone()
                    }
                }
                None => String::new(),
            })
            .collect();
        output.push_str(&values.join(","));
        output.push('\n');
    }

    output
}

/// Export table data to JSON format
pub fn export_to_json(data: &TableDataResult) -> Result<String> {
    Ok(serde_json::to_string_pretty(&data.rows)?)
}

// ============================================
// Interactive SQL REPL
// ============================================

/// Run an interactive SQL REPL connected to the specified database
pub async fn run_repl(cluster: &str, database: &str) -> Result<()> {
    use colored::Colorize;

    let source = ConnectionSource::from_cluster(cluster)?;
    let (client, _handle) = crate::connection::connect(&source, Some(database)).await?;
    let metadata = crate::server::load_cluster_metadata(cluster)?;

    println!(
        "Connected to {} on {} (PostgreSQL {})",
        database.cyan().bold(),
        cluster.cyan(),
        metadata.version
    );
    println!(
        "Type {} to quit, {} for help.\n",
        "\\q".green(),
        "\\?".green()
    );

    let mut rl = rustyline::DefaultEditor::new()?;
    let history_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".dbctl")
        .join("repl_history");
    let _ = rl.load_history(&history_path);

    let prompt = format!("{}=> ", database);
    let cont_prompt = format!("{}-> ", database);
    let mut buffer = String::new();

    loop {
        let p = if buffer.is_empty() {
            &prompt
        } else {
            &cont_prompt
        };
        match rl.readline(p) {
            Ok(line) => {
                let trimmed = line.trim();

                // Handle backslash commands when buffer is empty
                if buffer.is_empty() {
                    if trimmed == "\\q" || trimmed == "quit" || trimmed == "exit" {
                        break;
                    }
                    if trimmed == "\\?" || trimmed == "\\help" {
                        print_repl_help();
                        continue;
                    }
                    if trimmed == "\\dt" {
                        let sql = "SELECT table_name as \"Name\", table_type as \"Type\" FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name";
                        execute_and_display(&client, sql).await;
                        continue;
                    }
                    if trimmed == "\\l" {
                        let sql = "SELECT datname as \"Name\", pg_size_pretty(pg_database_size(datname)) as \"Size\" FROM pg_database WHERE datistemplate = false ORDER BY datname";
                        execute_and_display(&client, sql).await;
                        continue;
                    }
                    if let Some(table) = trimmed.strip_prefix("\\d ") {
                        let table = table.trim();
                        let sql = format!(
                            "SELECT column_name as \"Column\", data_type as \"Type\", \
                             is_nullable as \"Nullable\", column_default as \"Default\" \
                             FROM information_schema.columns \
                             WHERE table_schema = 'public' AND table_name = '{}' \
                             ORDER BY ordinal_position",
                            table.replace('\'', "''")
                        );
                        execute_and_display(&client, &sql).await;
                        continue;
                    }
                    if trimmed.is_empty() {
                        continue;
                    }
                }

                if !buffer.is_empty() {
                    buffer.push('\n');
                }
                buffer.push_str(&line);

                // Execute when statement ends with semicolon
                if buffer.trim_end().ends_with(';') {
                    let sql = buffer.trim().to_string();
                    let _ = rl.add_history_entry(&sql);
                    execute_and_display(&client, &sql).await;
                    buffer.clear();
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => {
                if !buffer.is_empty() {
                    buffer.clear();
                    println!();
                }
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                eprintln!("Error: {}", err);
                break;
            }
        }
    }

    let _ = rl.save_history(&history_path);
    println!("Goodbye.");
    Ok(())
}

fn print_repl_help() {
    println!("Available commands:");
    println!("  \\q         Quit");
    println!("  \\dt        List tables");
    println!("  \\d TABLE   Describe a table");
    println!("  \\l         List databases");
    println!("  \\?         Show this help");
    println!();
    println!("Enter SQL statements terminated with a semicolon (;).");
    println!("Multi-line statements are supported.");
    println!();
}

async fn execute_and_display(client: &tokio_postgres::Client, sql: &str) {
    use tokio_postgres::SimpleQueryMessage;

    let start = std::time::Instant::now();

    match client.simple_query(sql).await {
        Ok(messages) => {
            let elapsed = start.elapsed();
            let mut columns: Vec<String> = Vec::new();
            let mut rows: Vec<Vec<Option<String>>> = Vec::new();

            for msg in messages {
                match msg {
                    SimpleQueryMessage::Row(row) => {
                        if columns.is_empty() {
                            columns = row.columns().iter().map(|c| c.name().to_string()).collect();
                        }
                        let values: Vec<Option<String>> = (0..row.columns().len())
                            .map(|i| row.get(i).map(|s| s.to_string()))
                            .collect();
                        rows.push(values);
                    }
                    SimpleQueryMessage::CommandComplete(count) => {
                        if !columns.is_empty() {
                            print_result_table(&columns, &rows);
                            println!(
                                "({} row{})",
                                rows.len(),
                                if rows.len() == 1 { "" } else { "s" }
                            );
                            columns.clear();
                            rows.clear();
                        } else if count > 0 {
                            println!(
                                "{} row{} affected",
                                count,
                                if count == 1 { "" } else { "s" }
                            );
                        } else {
                            println!("OK");
                        }
                    }
                    _ => {}
                }
            }

            if elapsed.as_millis() >= 1 {
                println!("Time: {:.3}s", elapsed.as_secs_f64());
            }
            println!();
        }
        Err(e) => {
            eprintln!("ERROR: {}\n", e);
        }
    }
}

fn print_result_table(columns: &[String], rows: &[Vec<Option<String>>]) {
    if columns.is_empty() {
        return;
    }

    // Calculate column widths
    let mut widths: Vec<usize> = columns.iter().map(|c| c.len()).collect();
    for row in rows {
        for (i, val) in row.iter().enumerate() {
            if i < widths.len() {
                let len = val.as_ref().map_or(0, |s| s.len());
                widths[i] = widths[i].max(len);
            }
        }
    }

    // Header
    let header: String = columns
        .iter()
        .enumerate()
        .map(|(i, c)| format!(" {:<width$} ", c, width = widths[i]))
        .collect::<Vec<_>>()
        .join("|");
    println!("{}", header);

    // Separator
    let separator: String = widths
        .iter()
        .map(|w| "-".repeat(w + 2))
        .collect::<Vec<_>>()
        .join("+");
    println!("{}", separator);

    // Rows
    for row in rows {
        let line: String = row
            .iter()
            .enumerate()
            .map(|(i, val)| {
                let w = if i < widths.len() { widths[i] } else { 0 };
                match val {
                    Some(s) => format!(" {:<width$} ", s, width = w),
                    None => format!(" {:<width$} ", "", width = w),
                }
            })
            .collect::<Vec<_>>()
            .join("|");
        println!("{}", line);
    }
}
