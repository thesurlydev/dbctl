use std::path::PathBuf;
use std::process::Command;

fn dbctl_command() -> Command {
    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("--");
    cmd
}

fn get_test_base_dir() -> PathBuf {
    dirs::home_dir().unwrap().join(".dbctl")
}

fn cleanup_test_cluster(name: &str) {
    let mut cmd = dbctl_command();
    cmd.args(["server", "remove-cluster", "--name", name]);
    let _ = cmd.output();
}

fn cleanup_test_version(version: &str) {
    let mut cmd = dbctl_command();
    cmd.args(["server", "uninstall", "--version", version]);
    let _ = cmd.output();
}

#[test]
fn test_server_list_empty() {
    let output = dbctl_command()
        .args(["server", "list"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Installed PostgreSQL versions"));
}

#[test]
fn test_server_install_and_list() {
    let version = "16.3";

    cleanup_test_version(version);

    let output = dbctl_command()
        .args(["server", "install", "--version", version])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Install command failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("PostgreSQL")
            && (stdout.contains("installed successfully") || stdout.contains("already installed"))
    );

    let output = dbctl_command()
        .args(["server", "list"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(version), "Version {} not in list", version);
}

#[test]
fn test_server_install_duplicate() {
    let version = "16.3";

    let _ = dbctl_command()
        .args(["server", "install", "--version", version])
        .output();

    let output = dbctl_command()
        .args(["server", "install", "--version", version])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("already installed"));
}

#[test]
fn test_cluster_init_start_stop() {
    let version = "16.3";
    let cluster_name = "test_cluster";
    let port = "5440";

    cleanup_test_cluster(cluster_name);

    let _ = dbctl_command()
        .args(["server", "install", "--version", version])
        .output();

    let output = dbctl_command()
        .args([
            "server",
            "init",
            "--version",
            version,
            "--name",
            cluster_name,
            "--port",
            port,
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Init command failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("initialized"));

    let output = dbctl_command()
        .args(["server", "start", "--name", cluster_name])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Start command failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("started"));

    std::thread::sleep(std::time::Duration::from_secs(2));

    let output = dbctl_command()
        .args(["server", "status", "--name", cluster_name])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("running"));

    let output = dbctl_command()
        .args(["server", "stop", "--name", cluster_name])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Stop command failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("stopped"));

    cleanup_test_cluster(cluster_name);
}

#[test]
fn test_cluster_list() {
    let version = "16.3";
    let cluster_name = "test_list_cluster";

    cleanup_test_cluster(cluster_name);

    let _ = dbctl_command()
        .args(["server", "install", "--version", version])
        .output();

    let _ = dbctl_command()
        .args([
            "server",
            "init",
            "--version",
            version,
            "--name",
            cluster_name,
            "--port",
            "5441",
        ])
        .output();

    let output = dbctl_command()
        .args(["server", "list-clusters"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(cluster_name) || stdout.contains("PostgreSQL clusters"));

    cleanup_test_cluster(cluster_name);
}

#[test]
fn test_cluster_remove() {
    let version = "16.3";
    let cluster_name = "test_remove_cluster";

    cleanup_test_cluster(cluster_name);

    let _ = dbctl_command()
        .args(["server", "install", "--version", version])
        .output();

    let _ = dbctl_command()
        .args([
            "server",
            "init",
            "--version",
            version,
            "--name",
            cluster_name,
            "--port",
            "5442",
        ])
        .output();

    let output = dbctl_command()
        .args(["server", "remove-cluster", "--name", cluster_name])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("removed") || output.status.success());

    let cluster_dir = get_test_base_dir().join("clusters").join(cluster_name);
    assert!(!cluster_dir.exists(), "Cluster directory still exists");
}

#[test]
fn test_database_create_and_drop() {
    let version = "16.3";
    let cluster_name = "test_db_cluster";
    let database_name = "test_database";

    cleanup_test_cluster(cluster_name);

    let _ = dbctl_command()
        .args(["server", "install", "--version", version])
        .output();

    let _ = dbctl_command()
        .args([
            "server",
            "init",
            "--version",
            version,
            "--name",
            cluster_name,
            "--port",
            "5443",
        ])
        .output();

    let _ = dbctl_command()
        .args(["server", "start", "--name", cluster_name])
        .output();

    std::thread::sleep(std::time::Duration::from_secs(2));

    let output = dbctl_command()
        .args([
            "db",
            "create",
            "--cluster",
            cluster_name,
            "--database",
            database_name,
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Create database failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Created") || stdout.contains("already exists"));

    let output = dbctl_command()
        .args([
            "db",
            "drop",
            "--cluster",
            cluster_name,
            "--database",
            database_name,
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Drop database failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dropped"));

    let _ = dbctl_command()
        .args(["server", "stop", "--name", cluster_name])
        .output();

    cleanup_test_cluster(cluster_name);
}

#[test]
fn test_database_create_duplicate() {
    let version = "16.3";
    let cluster_name = "test_dup_db_cluster";
    let database_name = "test_dup_database";

    cleanup_test_cluster(cluster_name);

    let _ = dbctl_command()
        .args(["server", "install", "--version", version])
        .output();

    let _ = dbctl_command()
        .args([
            "server",
            "init",
            "--version",
            version,
            "--name",
            cluster_name,
            "--port",
            "5444",
        ])
        .output();

    let _ = dbctl_command()
        .args(["server", "start", "--name", cluster_name])
        .output();

    std::thread::sleep(std::time::Duration::from_secs(2));

    let _ = dbctl_command()
        .args([
            "db",
            "create",
            "--cluster",
            cluster_name,
            "--database",
            database_name,
        ])
        .output();

    let output = dbctl_command()
        .args([
            "db",
            "create",
            "--cluster",
            cluster_name,
            "--database",
            database_name,
        ])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("already exists"));

    let _ = dbctl_command()
        .args(["server", "stop", "--name", cluster_name])
        .output();

    cleanup_test_cluster(cluster_name);
}

#[test]
fn test_error_nonexistent_cluster() {
    let output = dbctl_command()
        .args(["server", "start", "--name", "nonexistent_cluster"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("does not exist"));
}

#[test]
fn test_error_nonexistent_version() {
    let output = dbctl_command()
        .args([
            "server",
            "init",
            "--version",
            "99.0",
            "--name",
            "test",
            "--port",
            "5445",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not installed"));
}

#[test]
fn test_error_duplicate_cluster() {
    let version = "16.3";
    let cluster_name = "test_dup_cluster";

    cleanup_test_cluster(cluster_name);

    let _ = dbctl_command()
        .args(["server", "install", "--version", version])
        .output();

    let _ = dbctl_command()
        .args([
            "server",
            "init",
            "--version",
            version,
            "--name",
            cluster_name,
            "--port",
            "5446",
        ])
        .output();

    let output = dbctl_command()
        .args([
            "server",
            "init",
            "--version",
            version,
            "--name",
            cluster_name,
            "--port",
            "5447",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already exists"));

    cleanup_test_cluster(cluster_name);
}

#[test]
fn test_uninstall_version_in_use() {
    let version = "16.3";
    let cluster_name = "test_uninstall_cluster";

    cleanup_test_cluster(cluster_name);

    let _ = dbctl_command()
        .args(["server", "install", "--version", version])
        .output();

    let _ = dbctl_command()
        .args([
            "server",
            "init",
            "--version",
            version,
            "--name",
            cluster_name,
            "--port",
            "5448",
        ])
        .output();

    let output = dbctl_command()
        .args(["server", "uninstall", "--version", version])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("using it"));

    cleanup_test_cluster(cluster_name);
}

#[test]
fn test_custom_username_and_port() {
    let version = "16.3";
    let cluster_name = "test_custom_cluster";
    let username = "testuser";
    let port = "5449";

    cleanup_test_cluster(cluster_name);

    let _ = dbctl_command()
        .args(["server", "install", "--version", version])
        .output();

    let output = dbctl_command()
        .args([
            "server",
            "init",
            "--version",
            version,
            "--name",
            cluster_name,
            "--port",
            port,
            "--username",
            username,
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(port));
    assert!(stdout.contains(username));

    cleanup_test_cluster(cluster_name);
}

#[test]
fn test_db_exec_sql_command() {
    let version = "16.3";
    let cluster_name = "test_exec_cluster";
    let database_name = "test_exec_db";
    let port = "5450";

    cleanup_test_cluster(cluster_name);

    let _ = dbctl_command()
        .args(["server", "install", "--version", version])
        .output();

    let _ = dbctl_command()
        .args([
            "server",
            "init",
            "--version",
            version,
            "--name",
            cluster_name,
            "--port",
            port,
        ])
        .output();

    let _ = dbctl_command()
        .args(["server", "start", "--name", cluster_name])
        .output();

    std::thread::sleep(std::time::Duration::from_secs(2));

    let _ = dbctl_command()
        .args([
            "db",
            "create",
            "--cluster",
            cluster_name,
            "--database",
            database_name,
        ])
        .output();

    let output = dbctl_command()
        .args([
            "db",
            "exec",
            "--cluster",
            cluster_name,
            "--database",
            database_name,
            "--sql",
            "CREATE TABLE test_table (id INT, name TEXT); INSERT INTO test_table VALUES (1, 'test');",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Exec SQL command failed");

    let output = dbctl_command()
        .args([
            "db",
            "exec",
            "--cluster",
            cluster_name,
            "--database",
            database_name,
            "--sql",
            "SELECT * FROM test_table;",
        ])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test"), "Query result not found");

    let _ = dbctl_command()
        .args(["server", "stop", "--name", cluster_name])
        .output();

    cleanup_test_cluster(cluster_name);
}

#[test]
fn test_db_backup_and_restore() {
    let version = "16.3";
    let cluster_name = "test_backup_cluster";
    let database_name = "test_backup_db";
    let port = "5451";
    let backup_file = "/tmp/test_backup.sql";

    cleanup_test_cluster(cluster_name);

    let _ = dbctl_command()
        .args(["server", "install", "--version", version])
        .output();

    let _ = dbctl_command()
        .args([
            "server",
            "init",
            "--version",
            version,
            "--name",
            cluster_name,
            "--port",
            port,
        ])
        .output();

    let _ = dbctl_command()
        .args(["server", "start", "--name", cluster_name])
        .output();

    std::thread::sleep(std::time::Duration::from_secs(2));

    let _ = dbctl_command()
        .args([
            "db",
            "create",
            "--cluster",
            cluster_name,
            "--database",
            database_name,
        ])
        .output();

    let _ = dbctl_command()
        .args([
            "db",
            "exec",
            "--cluster",
            cluster_name,
            "--database",
            database_name,
            "--sql",
            "CREATE TABLE backup_test (id INT, data TEXT); INSERT INTO backup_test VALUES (42, 'backup_data');",
        ])
        .output();

    let output = dbctl_command()
        .args([
            "db",
            "backup",
            "--cluster",
            cluster_name,
            "--database",
            database_name,
            "--output",
            backup_file,
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Backup command failed");
    assert!(
        std::path::Path::new(backup_file).exists(),
        "Backup file not created"
    );

    let _ = dbctl_command()
        .args([
            "db",
            "exec",
            "--cluster",
            cluster_name,
            "--database",
            database_name,
            "--sql",
            "DROP TABLE backup_test;",
        ])
        .output();

    let output = dbctl_command()
        .args([
            "db",
            "restore",
            "--cluster",
            cluster_name,
            "--database",
            database_name,
            "--input",
            backup_file,
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Restore command failed");

    let output = dbctl_command()
        .args([
            "db",
            "exec",
            "--cluster",
            cluster_name,
            "--database",
            database_name,
            "--sql",
            "SELECT * FROM backup_test;",
        ])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("backup_data"), "Restored data not found");

    let _ = std::fs::remove_file(backup_file);

    let _ = dbctl_command()
        .args(["server", "stop", "--name", cluster_name])
        .output();

    cleanup_test_cluster(cluster_name);
}

#[test]
fn test_db_info() {
    let version = "16.3";
    let cluster_name = "test_info_cluster";
    let database_name = "test_info_db";
    let port = "5452";

    cleanup_test_cluster(cluster_name);

    let _ = dbctl_command()
        .args(["server", "install", "--version", version])
        .output();

    let _ = dbctl_command()
        .args([
            "server",
            "init",
            "--version",
            version,
            "--name",
            cluster_name,
            "--port",
            port,
        ])
        .output();

    let _ = dbctl_command()
        .args(["server", "start", "--name", cluster_name])
        .output();

    std::thread::sleep(std::time::Duration::from_secs(2));

    let _ = dbctl_command()
        .args([
            "db",
            "create",
            "--cluster",
            cluster_name,
            "--database",
            database_name,
        ])
        .output();

    let output = dbctl_command()
        .args([
            "db",
            "info",
            "--cluster",
            cluster_name,
            "--database",
            database_name,
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Info command failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Database:"), "Database info not found");
    assert!(stdout.contains("Size:"), "Size info not found");
    assert!(stdout.contains("Tables:"), "Tables info not found");
    assert!(stdout.contains("Extensions:"), "Extensions info not found");

    let _ = dbctl_command()
        .args(["server", "stop", "--name", cluster_name])
        .output();

    cleanup_test_cluster(cluster_name);
}

#[test]
fn test_db_user_management() {
    let version = "16.3";
    let cluster_name = "test_user_cluster";
    let port = "5453";
    let test_user = "test_user";

    cleanup_test_cluster(cluster_name);

    let _ = dbctl_command()
        .args(["server", "install", "--version", version])
        .output();

    let _ = dbctl_command()
        .args([
            "server",
            "init",
            "--version",
            version,
            "--name",
            cluster_name,
            "--port",
            port,
        ])
        .output();

    let _ = dbctl_command()
        .args(["server", "start", "--name", cluster_name])
        .output();

    std::thread::sleep(std::time::Duration::from_secs(2));

    let output = dbctl_command()
        .args([
            "db",
            "user",
            "create",
            "--cluster",
            cluster_name,
            "--username",
            test_user,
            "--password",
            "test_password",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "User create command failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Created user"),
        "User creation message not found"
    );

    let output = dbctl_command()
        .args(["db", "user", "list", "--cluster", cluster_name])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "User list command failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(test_user), "Created user not in list");

    let output = dbctl_command()
        .args([
            "db",
            "user",
            "drop",
            "--cluster",
            cluster_name,
            "--username",
            test_user,
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "User drop command failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Dropped user"),
        "User drop message not found"
    );

    let _ = dbctl_command()
        .args(["server", "stop", "--name", cluster_name])
        .output();

    cleanup_test_cluster(cluster_name);
}

#[test]
fn test_server_logs() {
    let version = "16.3";
    let cluster_name = "test_logs_cluster";
    let port = "5454";

    cleanup_test_cluster(cluster_name);

    let _ = dbctl_command()
        .args(["server", "install", "--version", version])
        .output();

    let _ = dbctl_command()
        .args([
            "server",
            "init",
            "--version",
            version,
            "--name",
            cluster_name,
            "--port",
            port,
        ])
        .output();

    let _ = dbctl_command()
        .args(["server", "start", "--name", cluster_name])
        .output();

    std::thread::sleep(std::time::Duration::from_secs(2));

    let output = dbctl_command()
        .args(["server", "logs", "--name", cluster_name, "--lines", "10"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Logs command failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.trim().is_empty() || stdout.contains("No log files"),
        "No log output found"
    );

    let _ = dbctl_command()
        .args(["server", "stop", "--name", cluster_name])
        .output();

    cleanup_test_cluster(cluster_name);
}

#[test]
fn test_server_restart() {
    let version = "16.3";
    let cluster_name = "test_restart_cluster";
    let port = "5455";

    cleanup_test_cluster(cluster_name);

    let _ = dbctl_command()
        .args(["server", "install", "--version", version])
        .output();

    let _ = dbctl_command()
        .args([
            "server",
            "init",
            "--version",
            version,
            "--name",
            cluster_name,
            "--port",
            port,
        ])
        .output();

    let _ = dbctl_command()
        .args(["server", "start", "--name", cluster_name])
        .output();

    std::thread::sleep(std::time::Duration::from_secs(2));

    let output = dbctl_command()
        .args(["server", "restart", "--name", cluster_name])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Restart command failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("restarted"), "Restart message not found");

    std::thread::sleep(std::time::Duration::from_secs(2));

    let output = dbctl_command()
        .args(["server", "status", "--name", cluster_name])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("running"),
        "Cluster not running after restart"
    );

    let _ = dbctl_command()
        .args(["server", "stop", "--name", cluster_name])
        .output();

    cleanup_test_cluster(cluster_name);
}

#[test]
fn test_server_reload() {
    let version = "16.3";
    let cluster_name = "test_reload_cluster";
    let port = "5456";

    cleanup_test_cluster(cluster_name);

    let _ = dbctl_command()
        .args(["server", "install", "--version", version])
        .output();

    let _ = dbctl_command()
        .args([
            "server",
            "init",
            "--version",
            version,
            "--name",
            cluster_name,
            "--port",
            port,
        ])
        .output();

    let _ = dbctl_command()
        .args(["server", "start", "--name", cluster_name])
        .output();

    std::thread::sleep(std::time::Duration::from_secs(2));

    let output = dbctl_command()
        .args(["server", "reload", "--name", cluster_name])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Reload command failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Configuration reloaded") || stdout.contains("reloaded"),
        "Reload message not found"
    );

    let output = dbctl_command()
        .args(["server", "status", "--name", cluster_name])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("running"),
        "Cluster not running after reload"
    );

    let _ = dbctl_command()
        .args(["server", "stop", "--name", cluster_name])
        .output();

    cleanup_test_cluster(cluster_name);
}

#[test]
fn test_db_list() {
    let version = "16.3";
    let cluster_name = "test_db_list_cluster";
    let database_name = "test_list_db";
    let port = "5457";

    cleanup_test_cluster(cluster_name);

    let _ = dbctl_command()
        .args(["server", "install", "--version", version])
        .output();

    let _ = dbctl_command()
        .args([
            "server",
            "init",
            "--version",
            version,
            "--name",
            cluster_name,
            "--port",
            port,
        ])
        .output();

    let _ = dbctl_command()
        .args(["server", "start", "--name", cluster_name])
        .output();

    std::thread::sleep(std::time::Duration::from_secs(2));

    let _ = dbctl_command()
        .args([
            "db",
            "create",
            "--cluster",
            cluster_name,
            "--database",
            database_name,
        ])
        .output();

    let output = dbctl_command()
        .args(["db", "list", "--cluster", cluster_name])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "List databases command failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(database_name),
        "Created database not found in list"
    );
    assert!(
        stdout.contains("postgres"),
        "Default postgres database not found"
    );

    let _ = dbctl_command()
        .args(["server", "stop", "--name", cluster_name])
        .output();

    cleanup_test_cluster(cluster_name);
}

#[test]
fn test_db_exec_file() {
    let version = "16.3";
    let cluster_name = "test_exec_file_cluster";
    let database_name = "test_exec_file_db";
    let port = "5458";
    let sql_file = "/tmp/test_exec.sql";

    cleanup_test_cluster(cluster_name);

    let _ = dbctl_command()
        .args(["server", "install", "--version", version])
        .output();

    let _ = dbctl_command()
        .args([
            "server",
            "init",
            "--version",
            version,
            "--name",
            cluster_name,
            "--port",
            port,
        ])
        .output();

    let _ = dbctl_command()
        .args(["server", "start", "--name", cluster_name])
        .output();

    std::thread::sleep(std::time::Duration::from_secs(2));

    let _ = dbctl_command()
        .args([
            "db",
            "create",
            "--cluster",
            cluster_name,
            "--database",
            database_name,
        ])
        .output();

    std::fs::write(
        sql_file,
        "CREATE TABLE test_file_table (id INT, name TEXT);\nINSERT INTO test_file_table VALUES (1, 'from_file');",
    )
    .expect("Failed to write SQL file");

    let output = dbctl_command()
        .args([
            "db",
            "exec",
            "--cluster",
            cluster_name,
            "--database",
            database_name,
            "--file",
            sql_file,
        ])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "Exec file command failed");

    let output = dbctl_command()
        .args([
            "db",
            "exec",
            "--cluster",
            cluster_name,
            "--database",
            database_name,
            "--sql",
            "SELECT * FROM test_file_table;",
        ])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("from_file"), "Data from SQL file not found");

    let _ = std::fs::remove_file(sql_file);

    let _ = dbctl_command()
        .args(["server", "stop", "--name", cluster_name])
        .output();

    cleanup_test_cluster(cluster_name);
}
