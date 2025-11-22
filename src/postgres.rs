// src/postgres.rs
use anyhow::{Result, bail};
use clap::{Subcommand, ValueEnum};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

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
            list_databases(&cluster).await?;
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
                list_users(&cluster).await?;
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

async fn create_database(cluster: &str, database: &str) -> Result<()> {
    let metadata = super::server::load_cluster_metadata(cluster)?;

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

    // Check if exists
    let row = client
        .query_one("SELECT 1 FROM pg_database WHERE datname = $1", &[&database])
        .await;

    if row.is_ok() {
        println!(
            "Database '{}' already exists in cluster '{}'",
            database, cluster
        );
        return Ok(());
    }

    // Create database
    let query = format!("CREATE DATABASE {}", database);
    client.batch_execute(&query).await?;

    println!("✓ Created database '{}' in cluster '{}'", database, cluster);

    Ok(())
}

async fn drop_database(cluster: &str, database: &str) -> Result<()> {
    let metadata = super::server::load_cluster_metadata(cluster)?;

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

    // Terminate connections
    let terminate_query = format!(
        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}' AND pid <> pg_backend_pid()",
        database
    );
    client.batch_execute(&terminate_query).await.ok();

    // Drop database
    let query = format!("DROP DATABASE IF EXISTS {}", database);
    client.batch_execute(&query).await?;

    println!(
        "✓ Dropped database '{}' from cluster '{}'",
        database, cluster
    );

    Ok(())
}

async fn list_databases(cluster: &str) -> Result<()> {
    let metadata = super::server::load_cluster_metadata(cluster)?;

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

    let rows = client
        .query(
            "SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY datname",
            &[],
        )
        .await?;

    if rows.is_empty() {
        println!("No databases found in cluster '{}'", cluster);
    } else {
        println!("Databases in cluster '{}':", cluster);
        for row in rows {
            let datname: String = row.get(0);
            println!("  {}", datname);
        }
    }

    Ok(())
}

fn shell(cluster: &str, database: Option<&str>) -> Result<()> {
    let metadata = super::server::load_cluster_metadata(cluster)?;
    let dbname = database.unwrap_or("postgres");

    let version_dir = super::server::get_version_dir(&metadata.version);
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

fn exec(cluster: &str, database: &str, file: Option<&str>, sql: Option<&str>) -> Result<()> {
    let metadata = super::server::load_cluster_metadata(cluster)?;

    let version_dir = super::server::get_version_dir(&metadata.version);
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

fn backup(cluster: &str, database: &str, output: &str) -> Result<()> {
    let metadata = super::server::load_cluster_metadata(cluster)?;

    let version_dir = super::server::get_version_dir(&metadata.version);
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

fn restore(cluster: &str, database: &str, input: &str) -> Result<()> {
    let metadata = super::server::load_cluster_metadata(cluster)?;

    if !Path::new(input).exists() {
        bail!("Input file not found: {}", input);
    }

    let version_dir = super::server::get_version_dir(&metadata.version);
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
    let metadata = super::server::load_cluster_metadata(cluster)?;

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

async fn create_user(
    cluster: &str,
    username: &str,
    password: Option<&str>,
    superuser: bool,
) -> Result<()> {
    let metadata = super::server::load_cluster_metadata(cluster)?;

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

async fn list_users(cluster: &str) -> Result<()> {
    let metadata = super::server::load_cluster_metadata(cluster)?;

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

    let rows = client
        .query(
            "SELECT rolname, rolsuper, rolcreatedb, rolcreaterole FROM pg_roles WHERE rolname NOT LIKE 'pg_%' ORDER BY rolname",
            &[],
        )
        .await?;

    if rows.is_empty() {
        println!("No users found in cluster '{}'", cluster);
    } else {
        println!("Users in cluster '{}':", cluster);
        println!("  Name               Superuser  CreateDB  CreateRole");
        println!("  ------------------ ---------- --------- ----------");
        for row in rows {
            let rolname: String = row.get(0);
            let rolsuper: bool = row.get(1);
            let rolcreatedb: bool = row.get(2);
            let rolcreaterole: bool = row.get(3);
            println!(
                "  {:<18} {:<10} {:<9} {}",
                rolname,
                if rolsuper { "yes" } else { "no" },
                if rolcreatedb { "yes" } else { "no" },
                if rolcreaterole { "yes" } else { "no" }
            );
        }
    }

    Ok(())
}

async fn drop_user(cluster: &str, username: &str) -> Result<()> {
    let metadata = super::server::load_cluster_metadata(cluster)?;

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
    let metadata = super::server::load_cluster_metadata(cluster)?;

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
    let metadata = super::server::load_cluster_metadata(cluster)?;

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

#[derive(Debug, Clone)]
pub struct ClusterStats {
    pub database_count: i64,
    pub user_count: i64,
    pub table_count: i64,
}

pub async fn get_cluster_stats(cluster: &str) -> Result<ClusterStats> {
    let metadata = super::server::load_cluster_metadata(cluster)?;

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

        let db_conn_str = format!(
            "host=localhost port={} user={} dbname={}",
            metadata.port, metadata.username, dbname
        );

        if let Ok((db_client, db_connection)) =
            tokio_postgres::connect(&db_conn_str, tokio_postgres::NoTls).await
        {
            tokio::spawn(async move {
                let _ = db_connection.await;
            });

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

async fn show_privileges(cluster: &str, database: &str, user: &str) -> Result<()> {
    let metadata = super::server::load_cluster_metadata(cluster)?;

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
