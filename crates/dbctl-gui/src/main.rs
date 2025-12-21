use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "dbctl - PostgreSQL Manager",
        options,
        Box::new(|cc| Ok(Box::new(DbctlApp::new(cc)))),
    )
}

#[derive(Debug, Clone, PartialEq)]
enum Tab {
    Overview,
    Databases,
    Users,
    Logs,
    Sql,
    Tables,
}

#[derive(Debug, Clone, PartialEq, Default)]
enum ExportFormat {
    #[default]
    Csv,
    Json,
}

#[derive(Debug, Clone)]
enum AsyncResult {
    Databases(String, Vec<String>),
    Users(String, Vec<dbctl::UserInfo>),
    Stats(String, dbctl::ClusterStats),
    SqlResult(String),
    Message(String),
    Error(String),
    // Table browser results
    Schemas(String, String, Vec<dbctl::SchemaInfo>),
    SchemaTables(String, String, String, Vec<dbctl::TableInfo>),
    TableStructure(String, String, String, String, dbctl::TableStructure),
    TableData(String, String, String, String, dbctl::TableDataResult),
}

/// State for the table browser tab
#[derive(Default)]
struct TableBrowserState {
    // Navigation
    selected_database: Option<String>,
    schemas: Vec<dbctl::SchemaInfo>,
    selected_schema: Option<String>,
    tables: Vec<dbctl::TableInfo>,
    selected_table: Option<String>,

    // Schema tree expansion state
    expanded_schemas: HashSet<String>,

    // Table structure
    table_structure: Option<dbctl::TableStructure>,

    // Data browsing
    table_data: Option<dbctl::TableDataResult>,
    current_page: usize,
    page_size: usize,
    total_rows: i64,

    // Inline editing
    editing_row: Option<usize>,
    editing_column: Option<String>,
    edit_buffer: String,

    // New row insertion
    show_insert_dialog: bool,
    new_row_values: HashMap<String, String>,

    // Export
    show_export_dialog: bool,
    export_format: ExportFormat,
    export_path: String,

    // Delete confirmation
    show_delete_dialog: bool,
    pending_delete_pk: Option<String>,

    // Loading states
    loading_schemas: bool,
    loading_tables: bool,
    loading_structure: bool,
    loading_data: bool,
}

struct DbctlApp {
    // Cluster state
    clusters: Vec<ClusterInfo>,
    selected_cluster: Option<usize>,

    // Tab state
    current_tab: Tab,

    // Database state
    databases: Vec<String>,
    selected_database: Option<usize>,
    new_db_name: String,

    // Table browser state
    table_browser: TableBrowserState,

    // User state
    users: Vec<dbctl::UserInfo>,
    new_user_name: String,
    new_user_password: String,
    new_user_superuser: bool,

    // Logs state
    logs_content: String,
    logs_lines: usize,

    // SQL state
    sql_query: String,
    sql_result: String,
    sql_database: String,

    // Install state
    show_install_dialog: bool,
    install_version: String,
    installed_versions: Vec<String>,

    // Create cluster state
    show_create_cluster_dialog: bool,
    new_cluster_name: String,
    new_cluster_version: String,
    new_cluster_port: String,
    new_cluster_username: String,

    // Backup/restore state
    backup_path: String,
    restore_path: String,

    // Confirmation dialogs
    show_remove_cluster_dialog: bool,
    show_drop_db_dialog: bool,
    pending_drop_db: Option<String>,

    // Status
    status_message: String,

    // Async handling
    runtime: tokio::runtime::Runtime,
    result_rx: mpsc::Receiver<AsyncResult>,
    result_tx: mpsc::Sender<AsyncResult>,
}

#[derive(Clone)]
struct ClusterInfo {
    name: String,
    metadata: dbctl::ClusterMetadata,
    running: bool,
    stats: Option<dbctl::ClusterStats>,
}

impl DbctlApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        let (result_tx, result_rx) = mpsc::channel();

        let mut app = Self {
            clusters: Vec::new(),
            selected_cluster: None,
            current_tab: Tab::Overview,
            databases: Vec::new(),
            selected_database: None,
            new_db_name: String::new(),
            table_browser: TableBrowserState {
                page_size: 50,
                export_format: ExportFormat::Csv,
                ..Default::default()
            },
            users: Vec::new(),
            new_user_name: String::new(),
            new_user_password: String::new(),
            new_user_superuser: false,
            logs_content: String::new(),
            logs_lines: 100,
            sql_query: String::new(),
            sql_result: String::new(),
            sql_database: String::from("postgres"),
            show_install_dialog: false,
            install_version: String::from("17"),
            installed_versions: Vec::new(),
            show_create_cluster_dialog: false,
            new_cluster_name: String::new(),
            new_cluster_version: String::new(),
            new_cluster_port: String::new(),
            new_cluster_username: String::from("postgres"),
            backup_path: String::new(),
            restore_path: String::new(),
            show_remove_cluster_dialog: false,
            show_drop_db_dialog: false,
            pending_drop_db: None,
            status_message: String::new(),
            runtime,
            result_rx,
            result_tx,
        };

        app.refresh_clusters();
        app.refresh_installed_versions();
        app
    }

    fn refresh_clusters(&mut self) {
        match dbctl::server::get_all_clusters() {
            Ok(clusters) => {
                self.clusters = clusters
                    .into_iter()
                    .map(|(name, metadata)| {
                        let running = dbctl::server::is_cluster_running(&name).unwrap_or(false);
                        ClusterInfo {
                            name,
                            metadata,
                            running,
                            stats: None,
                        }
                    })
                    .collect();
                self.status_message = format!("Loaded {} cluster(s)", self.clusters.len());

                // Fetch stats for running clusters
                let running_clusters: Vec<String> = self.clusters.iter()
                    .filter(|c| c.running)
                    .map(|c| c.name.clone())
                    .collect();
                for name in running_clusters {
                    self.fetch_cluster_stats(&name);
                }
            }
            Err(e) => {
                self.status_message = format!("Error: {}", e);
            }
        }
    }

    fn refresh_installed_versions(&mut self) {
        match dbctl::server::get_installed_versions() {
            Ok(versions) => {
                self.installed_versions = versions;
                if let Some(first) = self.installed_versions.first() {
                    self.new_cluster_version = first.clone();
                }
            }
            Err(e) => {
                self.status_message = format!("Error loading versions: {}", e);
            }
        }
    }

    fn selected_cluster(&self) -> Option<&ClusterInfo> {
        self.selected_cluster.and_then(|i| self.clusters.get(i))
    }

    fn fetch_databases(&mut self, cluster: &str) {
        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        self.runtime.spawn(async move {
            match dbctl::postgres::list_databases(&cluster).await {
                Ok(dbs) => { let _ = tx.send(AsyncResult::Databases(cluster, dbs)); }
                Err(e) => { let _ = tx.send(AsyncResult::Error(e.to_string())); }
            }
        });
    }

    fn fetch_users(&mut self, cluster: &str) {
        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        self.runtime.spawn(async move {
            match dbctl::postgres::list_users(&cluster).await {
                Ok(users) => { let _ = tx.send(AsyncResult::Users(cluster, users)); }
                Err(e) => { let _ = tx.send(AsyncResult::Error(e.to_string())); }
            }
        });
    }

    fn fetch_cluster_stats(&mut self, cluster: &str) {
        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        self.runtime.spawn(async move {
            match dbctl::postgres::get_cluster_stats(&cluster).await {
                Ok(stats) => { let _ = tx.send(AsyncResult::Stats(cluster, stats)); }
                Err(_) => {} // Ignore stats errors
            }
        });
    }

    // Table browser async methods
    fn fetch_schemas(&mut self, cluster: &str, database: &str) {
        self.table_browser.loading_schemas = true;
        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        let database = database.to_string();
        self.runtime.spawn(async move {
            match dbctl::postgres::list_schemas(&cluster, &database).await {
                Ok(schemas) => {
                    let _ = tx.send(AsyncResult::Schemas(cluster, database, schemas));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(e.to_string()));
                }
            }
        });
    }

    fn fetch_schema_tables(&mut self, cluster: &str, database: &str, schema: &str) {
        self.table_browser.loading_tables = true;
        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        let database = database.to_string();
        let schema = schema.to_string();
        self.runtime.spawn(async move {
            match dbctl::postgres::list_tables(&cluster, &database, &schema).await {
                Ok(tables) => {
                    let _ = tx.send(AsyncResult::SchemaTables(cluster, database, schema, tables));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(e.to_string()));
                }
            }
        });
    }

    fn fetch_table_structure(&mut self, cluster: &str, database: &str, schema: &str, table: &str) {
        self.table_browser.loading_structure = true;
        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        let database = database.to_string();
        let schema = schema.to_string();
        let table = table.to_string();
        self.runtime.spawn(async move {
            match dbctl::postgres::get_table_structure(&cluster, &database, &schema, &table).await {
                Ok(structure) => {
                    let _ = tx.send(AsyncResult::TableStructure(
                        cluster, database, schema, table, structure,
                    ));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(e.to_string()));
                }
            }
        });
    }

    fn fetch_table_data(
        &mut self,
        cluster: &str,
        database: &str,
        schema: &str,
        table: &str,
        page: usize,
    ) {
        self.table_browser.loading_data = true;
        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        let database = database.to_string();
        let schema = schema.to_string();
        let table = table.to_string();
        let page_size = self.table_browser.page_size;
        self.runtime.spawn(async move {
            match dbctl::postgres::get_table_data(&cluster, &database, &schema, &table, page, page_size)
                .await
            {
                Ok(data) => {
                    let _ = tx.send(AsyncResult::TableData(cluster, database, schema, table, data));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(e.to_string()));
                }
            }
        });
    }

    fn refresh_table_data(&mut self, cluster: &str) {
        if let (Some(db), Some(schema), Some(table)) = (
            self.table_browser.selected_database.clone(),
            self.table_browser.selected_schema.clone(),
            self.table_browser.selected_table.clone(),
        ) {
            self.fetch_table_data(cluster, &db, &schema, &table, self.table_browser.current_page);
        }
    }

    fn insert_table_row(&mut self, cluster: &str, database: &str, schema: &str, table: &str) {
        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        let database = database.to_string();
        let schema = schema.to_string();
        let table = table.to_string();
        let values = self.table_browser.new_row_values.clone();
        self.runtime.spawn(async move {
            match dbctl::postgres::insert_row(&cluster, &database, &schema, &table, &values).await {
                Ok(()) => {
                    let _ = tx.send(AsyncResult::Message("Row inserted successfully".to_string()));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(e.to_string()));
                }
            }
        });
    }

    fn update_table_cell(
        &mut self,
        cluster: &str,
        database: &str,
        schema: &str,
        table: &str,
        pk_column: &str,
        pk_value: &str,
        column: &str,
        new_value: Option<&str>,
    ) {
        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        let database = database.to_string();
        let schema = schema.to_string();
        let table = table.to_string();
        let pk_column = pk_column.to_string();
        let pk_value = pk_value.to_string();
        let column = column.to_string();
        let new_value = new_value.map(|s| s.to_string());
        self.runtime.spawn(async move {
            match dbctl::postgres::update_cell(
                &cluster,
                &database,
                &schema,
                &table,
                &pk_column,
                &pk_value,
                &column,
                new_value.as_deref(),
            )
            .await
            {
                Ok(()) => {
                    let _ = tx.send(AsyncResult::Message("Cell updated successfully".to_string()));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(e.to_string()));
                }
            }
        });
    }

    fn delete_table_row(
        &mut self,
        cluster: &str,
        database: &str,
        schema: &str,
        table: &str,
        pk_column: &str,
        pk_value: &str,
    ) {
        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        let database = database.to_string();
        let schema = schema.to_string();
        let table = table.to_string();
        let pk_column = pk_column.to_string();
        let pk_value = pk_value.to_string();
        self.runtime.spawn(async move {
            match dbctl::postgres::delete_row(&cluster, &database, &schema, &table, &pk_column, &pk_value)
                .await
            {
                Ok(()) => {
                    let _ = tx.send(AsyncResult::Message("Row deleted successfully".to_string()));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(e.to_string()));
                }
            }
        });
    }

    fn create_database(&mut self, cluster: &str, database: &str) {
        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        let database = database.to_string();
        self.runtime.spawn(async move {
            match dbctl::postgres::create_database(&cluster, &database).await {
                Ok(()) => { let _ = tx.send(AsyncResult::Message(format!("Created database '{}'", database))); }
                Err(e) => { let _ = tx.send(AsyncResult::Error(e.to_string())); }
            }
        });
    }

    fn drop_database(&mut self, cluster: &str, database: &str) {
        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        let database = database.to_string();
        self.runtime.spawn(async move {
            match dbctl::postgres::drop_database(&cluster, &database).await {
                Ok(()) => { let _ = tx.send(AsyncResult::Message(format!("Dropped database '{}'", database))); }
                Err(e) => { let _ = tx.send(AsyncResult::Error(e.to_string())); }
            }
        });
    }

    fn create_user(&mut self, cluster: &str, username: &str, password: Option<&str>, superuser: bool) {
        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        let username = username.to_string();
        let password = password.map(|s| s.to_string());
        self.runtime.spawn(async move {
            match dbctl::postgres::create_user(&cluster, &username, password.as_deref(), superuser).await {
                Ok(()) => { let _ = tx.send(AsyncResult::Message(format!("Created user '{}'", username))); }
                Err(e) => { let _ = tx.send(AsyncResult::Error(e.to_string())); }
            }
        });
    }

    fn install_postgres(&mut self, version: &str) {
        let tx = self.result_tx.clone();
        let version = version.to_string();
        self.runtime.spawn(async move {
            match dbctl::server::install_postgres(&version).await {
                Ok(()) => { let _ = tx.send(AsyncResult::Message(format!("Installed PostgreSQL {}", version))); }
                Err(e) => { let _ = tx.send(AsyncResult::Error(e.to_string())); }
            }
        });
    }

    fn process_async_results(&mut self) {
        while let Ok(result) = self.result_rx.try_recv() {
            match result {
                AsyncResult::Databases(cluster, dbs) => {
                    if self.selected_cluster().map(|c| &c.name) == Some(&cluster) {
                        self.databases = dbs;
                        if !self.databases.is_empty() && self.sql_database.is_empty() {
                            self.sql_database = self.databases[0].clone();
                        }
                    }
                }
                AsyncResult::Users(cluster, users) => {
                    if self.selected_cluster().map(|c| &c.name) == Some(&cluster) {
                        self.users = users;
                    }
                }
                AsyncResult::Stats(cluster, stats) => {
                    if let Some(c) = self.clusters.iter_mut().find(|c| c.name == cluster) {
                        c.stats = Some(stats);
                    }
                }
                AsyncResult::SqlResult(result) => {
                    self.sql_result = result;
                }
                AsyncResult::Message(msg) => {
                    self.status_message = msg;
                    // Refresh data after successful operations
                    if let Some(cluster) = self.selected_cluster().map(|c| c.name.clone()) {
                        self.fetch_databases(&cluster);
                        self.fetch_users(&cluster);
                    }
                    self.refresh_installed_versions();
                }
                AsyncResult::Error(err) => {
                    self.status_message = format!("Error: {}", err);
                    // Clear loading states on error
                    self.table_browser.loading_schemas = false;
                    self.table_browser.loading_tables = false;
                    self.table_browser.loading_structure = false;
                    self.table_browser.loading_data = false;
                }
                // Table browser results
                AsyncResult::Schemas(cluster, database, schemas) => {
                    if self.selected_cluster().map(|c| &c.name) == Some(&cluster)
                        && self.table_browser.selected_database.as_ref() == Some(&database)
                    {
                        self.table_browser.schemas = schemas;
                        self.table_browser.loading_schemas = false;
                        // Auto-expand public schema if it exists
                        if self.table_browser.schemas.iter().any(|s| s.name == "public") {
                            self.table_browser.expanded_schemas.insert("public".to_string());
                            self.table_browser.selected_schema = Some("public".to_string());
                            self.fetch_schema_tables(&cluster, &database, "public");
                        }
                    }
                }
                AsyncResult::SchemaTables(cluster, database, schema, tables) => {
                    if self.selected_cluster().map(|c| &c.name) == Some(&cluster)
                        && self.table_browser.selected_database.as_ref() == Some(&database)
                        && self.table_browser.selected_schema.as_ref() == Some(&schema)
                    {
                        self.table_browser.tables = tables;
                        self.table_browser.loading_tables = false;
                    }
                }
                AsyncResult::TableStructure(cluster, database, schema, table, structure) => {
                    if self.selected_cluster().map(|c| &c.name) == Some(&cluster)
                        && self.table_browser.selected_database.as_ref() == Some(&database)
                        && self.table_browser.selected_schema.as_ref() == Some(&schema)
                        && self.table_browser.selected_table.as_ref() == Some(&table)
                    {
                        self.table_browser.table_structure = Some(structure);
                        self.table_browser.loading_structure = false;
                    }
                }
                AsyncResult::TableData(cluster, database, schema, table, data) => {
                    if self.selected_cluster().map(|c| &c.name) == Some(&cluster)
                        && self.table_browser.selected_database.as_ref() == Some(&database)
                        && self.table_browser.selected_schema.as_ref() == Some(&schema)
                        && self.table_browser.selected_table.as_ref() == Some(&table)
                    {
                        self.table_browser.total_rows = data.total_rows;
                        self.table_browser.table_data = Some(data);
                        self.table_browser.loading_data = false;
                    }
                }
            }
        }
    }

    fn start_cluster(&mut self, name: &str) {
        if let Err(e) = dbctl::server::ensure_cluster_running(name) {
            self.status_message = format!("Failed to start: {}", e);
        } else {
            self.status_message = format!("Started cluster '{}'", name);
            self.refresh_clusters();
        }
    }

    fn stop_cluster(&mut self, name: &str) {
        if let Err(e) = dbctl::server::stop_cluster(name) {
            self.status_message = format!("Failed to stop: {}", e);
        } else {
            self.status_message = format!("Stopped cluster '{}'", name);
            self.refresh_clusters();
        }
    }

    fn restart_cluster(&mut self, name: &str) {
        if let Err(e) = dbctl::server::restart_cluster(name) {
            self.status_message = format!("Failed to restart: {}", e);
        } else {
            self.status_message = format!("Restarted cluster '{}'", name);
            self.refresh_clusters();
        }
    }

    fn remove_cluster(&mut self, name: &str) {
        if let Err(e) = dbctl::server::remove_cluster(name) {
            self.status_message = format!("Failed to remove: {}", e);
        } else {
            self.status_message = format!("Removed cluster '{}'", name);
            self.selected_cluster = None;
            self.refresh_clusters();
        }
    }

    fn create_cluster(&mut self) {
        let port: u16 = self.new_cluster_port.parse().unwrap_or(5432);
        match dbctl::server::init_cluster(
            &self.new_cluster_version,
            &self.new_cluster_name,
            port,
            &self.new_cluster_username,
        ) {
            Ok(()) => {
                self.status_message = format!("Created cluster '{}'", self.new_cluster_name);
                self.show_create_cluster_dialog = false;
                self.new_cluster_name.clear();
                self.refresh_clusters();
            }
            Err(e) => {
                self.status_message = format!("Failed to create cluster: {}", e);
            }
        }
    }

    fn refresh_logs(&mut self, cluster: &str) {
        match dbctl::server::get_logs(cluster, self.logs_lines) {
            Ok(logs) => self.logs_content = logs,
            Err(e) => self.logs_content = format!("Error: {}", e),
        }
    }

    fn execute_sql(&mut self, cluster: &str) {
        match dbctl::postgres::exec_sql(cluster, &self.sql_database, &self.sql_query) {
            Ok(result) => {
                self.sql_result = result;
                self.status_message = "SQL executed successfully".to_string();
            }
            Err(e) => {
                self.sql_result = format!("Error: {}", e);
                self.status_message = format!("SQL error: {}", e);
            }
        }
    }

    fn backup_database(&mut self, cluster: &str, database: &str) {
        match dbctl::postgres::backup(cluster, database, &self.backup_path) {
            Ok(()) => {
                self.status_message = format!("Backed up '{}' to '{}'", database, self.backup_path);
            }
            Err(e) => {
                self.status_message = format!("Backup failed: {}", e);
            }
        }
    }

    fn restore_database(&mut self, cluster: &str, database: &str) {
        match dbctl::postgres::restore(cluster, database, &self.restore_path) {
            Ok(()) => {
                self.status_message = format!("Restored '{}' from '{}'", database, self.restore_path);
            }
            Err(e) => {
                self.status_message = format!("Restore failed: {}", e);
            }
        }
    }
}

impl eframe::App for DbctlApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_async_results();

        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Install PostgreSQL...").clicked() {
                        self.show_install_dialog = true;
                        ui.close_menu();
                    }
                    if ui.button("Create Cluster...").clicked() {
                        self.show_create_cluster_dialog = true;
                        // Auto-find port
                        if let Ok(port) = dbctl::server::find_unused_port() {
                            self.new_cluster_port = port.to_string();
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Refresh").clicked() {
                        self.refresh_clusters();
                        self.refresh_installed_versions();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
        });

        // Left panel - cluster list
        egui::SidePanel::left("clusters_panel")
            .resizable(true)
            .default_width(220.0)
            .show(ctx, |ui| {
                ui.heading("Clusters");
                ui.separator();

                if ui.button("+ New Cluster").clicked() {
                    self.show_create_cluster_dialog = true;
                    if let Ok(port) = dbctl::server::find_unused_port() {
                        self.new_cluster_port = port.to_string();
                    }
                }

                ui.separator();

                let mut new_selection = None;
                for (i, cluster) in self.clusters.iter().enumerate() {
                    let status = if cluster.running { "[R]" } else { "[S]" };
                    let label = format!("{} {} (v{}, :{})",
                        status, cluster.name, cluster.metadata.version, cluster.metadata.port);

                    let selected = self.selected_cluster == Some(i);
                    if ui.selectable_label(selected, label).clicked() {
                        new_selection = Some(i);
                    }
                }

                if let Some(i) = new_selection {
                    self.selected_cluster = Some(i);
                    self.current_tab = Tab::Overview;
                    self.databases.clear();
                    self.users.clear();
                    self.logs_content.clear();

                    if let Some(cluster) = self.clusters.get(i).cloned() {
                        if cluster.running {
                            self.fetch_databases(&cluster.name);
                            self.fetch_users(&cluster.name);
                        }
                        self.refresh_logs(&cluster.name);
                    }
                }

                if self.clusters.is_empty() {
                    ui.separator();
                    ui.label("No clusters found");
                    ui.label("Click '+ New Cluster' or");
                    ui.label("install PostgreSQL first");
                }

                ui.separator();
                ui.heading("Installed");
                for v in &self.installed_versions {
                    ui.label(format!("  PostgreSQL {}", v));
                }
                if self.installed_versions.is_empty() {
                    ui.label("  (none)");
                }
            });

        // Status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
            });
        });

        // Main content
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(idx) = self.selected_cluster {
                if let Some(cluster) = self.clusters.get(idx).cloned() {
                    // Cluster header with controls
                    ui.horizontal(|ui| {
                        ui.heading(&cluster.name);
                        ui.separator();

                        if cluster.running {
                            ui.label(egui::RichText::new("Running").color(egui::Color32::GREEN));
                        } else {
                            ui.label(egui::RichText::new("Stopped").color(egui::Color32::RED));
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Remove").clicked() {
                                self.show_remove_cluster_dialog = true;
                            }

                            if cluster.running {
                                if ui.button("Restart").clicked() {
                                    self.restart_cluster(&cluster.name);
                                }
                                if ui.button("Stop").clicked() {
                                    self.stop_cluster(&cluster.name);
                                }
                            } else {
                                if ui.button("Start").clicked() {
                                    self.start_cluster(&cluster.name);
                                }
                            }
                        });
                    });

                    ui.separator();

                    // Tab bar
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.current_tab, Tab::Overview, "Overview");
                        if cluster.running {
                            ui.selectable_value(&mut self.current_tab, Tab::Databases, "Databases");
                            ui.selectable_value(&mut self.current_tab, Tab::Tables, "Tables");
                            ui.selectable_value(&mut self.current_tab, Tab::Users, "Users");
                            ui.selectable_value(&mut self.current_tab, Tab::Sql, "SQL");
                        }
                        ui.selectable_value(&mut self.current_tab, Tab::Logs, "Logs");
                    });

                    ui.separator();

                    // Tab content
                    match self.current_tab {
                        Tab::Overview => self.render_overview_tab(ui, &cluster),
                        Tab::Databases => self.render_databases_tab(ui, &cluster),
                        Tab::Tables => self.render_tables_tab(ui, &cluster),
                        Tab::Users => self.render_users_tab(ui, &cluster),
                        Tab::Logs => self.render_logs_tab(ui, &cluster),
                        Tab::Sql => self.render_sql_tab(ui, &cluster),
                    }
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Welcome to dbctl");
                        ui.add_space(20.0);
                        ui.label("Select a cluster from the sidebar to get started.");
                        ui.add_space(10.0);
                        if self.clusters.is_empty() {
                            if self.installed_versions.is_empty() {
                                ui.label("No PostgreSQL versions installed.");
                                if ui.button("Install PostgreSQL").clicked() {
                                    self.show_install_dialog = true;
                                }
                            } else {
                                ui.label("No clusters configured.");
                                if ui.button("Create a Cluster").clicked() {
                                    self.show_create_cluster_dialog = true;
                                    if let Ok(port) = dbctl::server::find_unused_port() {
                                        self.new_cluster_port = port.to_string();
                                    }
                                }
                            }
                        }
                    });
                });
            }
        });

        // Dialogs
        self.render_install_dialog(ctx);
        self.render_create_cluster_dialog(ctx);
        self.render_remove_cluster_dialog(ctx);
        self.render_drop_db_dialog(ctx);
    }
}

impl DbctlApp {
    fn render_overview_tab(&mut self, ui: &mut egui::Ui, cluster: &ClusterInfo) {
        egui::Grid::new("cluster_details")
            .num_columns(2)
            .spacing([40.0, 8.0])
            .show(ui, |ui| {
                ui.label("PostgreSQL Version:");
                ui.label(&cluster.metadata.version);
                ui.end_row();

                ui.label("Port:");
                ui.label(cluster.metadata.port.to_string());
                ui.end_row();

                ui.label("Superuser:");
                ui.label(&cluster.metadata.username);
                ui.end_row();

                ui.label("Data Directory:");
                ui.label(cluster.metadata.data_dir.display().to_string());
                ui.end_row();

                ui.label("Logs Directory:");
                ui.label(cluster.metadata.logs_dir.display().to_string());
                ui.end_row();

                if let Some(stats) = &cluster.stats {
                    ui.label("Databases:");
                    ui.label(stats.database_count.to_string());
                    ui.end_row();

                    ui.label("Users:");
                    ui.label(stats.user_count.to_string());
                    ui.end_row();

                    ui.label("Tables:");
                    ui.label(stats.table_count.to_string());
                    ui.end_row();
                }
            });

        if cluster.running {
            ui.add_space(20.0);
            ui.label("Connection string:");
            let conn_str = format!(
                "postgresql://{}@localhost:{}/postgres",
                cluster.metadata.username, cluster.metadata.port
            );
            ui.horizontal(|ui| {
                ui.monospace(&conn_str);
                if ui.button("Copy").clicked() {
                    ui.output_mut(|o| o.copied_text = conn_str.clone());
                    self.status_message = "Connection string copied".to_string();
                }
            });
        }
    }

    fn render_databases_tab(&mut self, ui: &mut egui::Ui, cluster: &ClusterInfo) {
        ui.horizontal(|ui| {
            ui.label("New database:");
            ui.text_edit_singleline(&mut self.new_db_name);
            if ui.button("Create").clicked() && !self.new_db_name.is_empty() {
                let db = self.new_db_name.clone();
                self.create_database(&cluster.name, &db);
                self.new_db_name.clear();
            }

            ui.separator();

            if ui.button("Refresh").clicked() {
                self.fetch_databases(&cluster.name);
            }
        });

        ui.separator();

        let mut action = None;

        egui::ScrollArea::vertical()
            .id_salt("databases_list_scroll")
            .show(ui, |ui| {
                for (i, db) in self.databases.iter().enumerate() {
                ui.horizontal(|ui| {
                    let selected = self.selected_database == Some(i);
                    if ui.selectable_label(selected, db).clicked() {
                        self.selected_database = Some(i);
                        self.sql_database = db.clone();
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if db != "postgres" && db != "template0" && db != "template1" {
                            if ui.small_button("Drop").clicked() {
                                action = Some(("drop", db.clone()));
                            }
                        }
                    });
                });
            }
        });

        if let Some((act, db)) = action {
            if act == "drop" {
                self.pending_drop_db = Some(db);
                self.show_drop_db_dialog = true;
            }
        }

        // Backup/Restore section
        if let Some(idx) = self.selected_database {
            if let Some(db) = self.databases.get(idx).cloned() {
                ui.separator();
                ui.heading("Backup / Restore");

                let cluster_name = cluster.name.clone();
                let db_clone = db.clone();

                ui.horizontal(|ui| {
                    ui.label("Backup to:");
                    ui.text_edit_singleline(&mut self.backup_path);
                });
                if ui.button("Backup").clicked() && !self.backup_path.is_empty() {
                    self.backup_database(&cluster_name, &db_clone);
                }

                ui.horizontal(|ui| {
                    ui.label("Restore from:");
                    ui.text_edit_singleline(&mut self.restore_path);
                });
                if ui.button("Restore").clicked() && !self.restore_path.is_empty() {
                    self.restore_database(&cluster_name, &db);
                }
            }
        }
    }

    fn render_users_tab(&mut self, ui: &mut egui::Ui, cluster: &ClusterInfo) {
        let cluster_name = cluster.name.clone();
        let mut should_create = false;
        let mut should_refresh = false;

        ui.horizontal(|ui| {
            ui.label("Username:");
            ui.text_edit_singleline(&mut self.new_user_name);
            ui.label("Password:");
            ui.add(egui::TextEdit::singleline(&mut self.new_user_password).password(true));
            ui.checkbox(&mut self.new_user_superuser, "Superuser");
            if ui.button("Create").clicked() && !self.new_user_name.is_empty() {
                should_create = true;
            }

            ui.separator();

            if ui.button("Refresh").clicked() {
                should_refresh = true;
            }
        });

        if should_create {
            let username = self.new_user_name.clone();
            let pwd = if self.new_user_password.is_empty() { None } else { Some(self.new_user_password.clone()) };
            let superuser = self.new_user_superuser;
            self.create_user(&cluster_name, &username, pwd.as_deref(), superuser);
            self.new_user_name.clear();
            self.new_user_password.clear();
            self.new_user_superuser = false;
        }

        if should_refresh {
            self.fetch_users(&cluster_name);
        }

        ui.separator();

        egui::Grid::new("users_grid")
            .num_columns(4)
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Username");
                ui.strong("Superuser");
                ui.strong("Create DB");
                ui.strong("Create Role");
                ui.end_row();

                for user in &self.users {
                    ui.label(&user.name);
                    ui.label(if user.superuser { "Yes" } else { "No" });
                    ui.label(if user.create_db { "Yes" } else { "No" });
                    ui.label(if user.create_role { "Yes" } else { "No" });
                    ui.end_row();
                }
            });
    }

    fn render_logs_tab(&mut self, ui: &mut egui::Ui, cluster: &ClusterInfo) {
        ui.horizontal(|ui| {
            ui.label("Lines:");
            ui.add(egui::DragValue::new(&mut self.logs_lines).range(10..=1000));
            if ui.button("Refresh").clicked() {
                self.refresh_logs(&cluster.name);
            }
        });

        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt("logs_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.logs_content.as_str())
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                );
            });
    }

    fn render_sql_tab(&mut self, ui: &mut egui::Ui, cluster: &ClusterInfo) {
        ui.horizontal(|ui| {
            ui.label("Database:");
            egui::ComboBox::from_id_salt("sql_db")
                .selected_text(&self.sql_database)
                .show_ui(ui, |ui| {
                    for db in &self.databases {
                        ui.selectable_value(&mut self.sql_database, db.clone(), db);
                    }
                });

            if ui.button("Execute").clicked() && !self.sql_query.is_empty() {
                self.execute_sql(&cluster.name);
            }

            if ui.button("Clear").clicked() {
                self.sql_query.clear();
                self.sql_result.clear();
            }
        });

        ui.separator();

        ui.label("SQL Query:");
        ui.add(
            egui::TextEdit::multiline(&mut self.sql_query)
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY)
                .desired_rows(5)
        );

        ui.separator();

        ui.label("Result:");
        egui::ScrollArea::vertical()
            .id_salt("sql_result_scroll")
            .max_height(200.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.sql_result.as_str())
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                );
            });
    }

    fn render_tables_tab(&mut self, ui: &mut egui::Ui, cluster: &ClusterInfo) {
        let cluster_name = cluster.name.clone();

        // Database selector at top
        ui.horizontal(|ui| {
            ui.label("Database:");
            egui::ComboBox::from_id_salt("table_browser_db")
                .selected_text(
                    self.table_browser
                        .selected_database
                        .as_deref()
                        .unwrap_or("Select database..."),
                )
                .show_ui(ui, |ui| {
                    for db in &self.databases.clone() {
                        let selected = self.table_browser.selected_database.as_ref() == Some(db);
                        if ui.selectable_label(selected, db).clicked() {
                            // Clear previous state when switching databases
                            self.table_browser.schemas.clear();
                            self.table_browser.tables.clear();
                            self.table_browser.selected_schema = None;
                            self.table_browser.selected_table = None;
                            self.table_browser.table_structure = None;
                            self.table_browser.table_data = None;
                            self.table_browser.expanded_schemas.clear();
                            self.table_browser.selected_database = Some(db.clone());
                            self.fetch_schemas(&cluster_name, db);
                        }
                    }
                });

            if ui.button("Refresh").clicked() {
                if let Some(db) = &self.table_browser.selected_database.clone() {
                    self.fetch_schemas(&cluster_name, db);
                }
            }
        });

        ui.separator();

        if self.table_browser.selected_database.is_none() {
            ui.centered_and_justified(|ui| {
                ui.label("Select a database to browse tables");
            });
            return;
        }

        // Main content area with split panels
        let available_width = ui.available_width();

        ui.horizontal(|ui| {
            // Left panel - Schema tree (fixed width)
            ui.allocate_ui_with_layout(
                egui::vec2(200.0, ui.available_height()),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    self.render_schema_tree(ui, &cluster_name);
                },
            );

            ui.separator();

            // Right panel - Table details
            ui.allocate_ui_with_layout(
                egui::vec2(available_width - 210.0, ui.available_height()),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    if self.table_browser.selected_table.is_some() {
                        // Structure panel (collapsible)
                        egui::CollapsingHeader::new("Table Structure")
                            .default_open(true)
                            .show(ui, |ui| {
                                self.render_table_structure(ui);
                            });

                        ui.separator();

                        // Data panel
                        self.render_table_data(ui, &cluster_name);
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label("Select a table from the schema tree to view its data");
                        });
                    }
                },
            );
        });
    }

    fn render_schema_tree(&mut self, ui: &mut egui::Ui, cluster: &str) {
        ui.heading("Schemas");

        if self.table_browser.loading_schemas {
            ui.spinner();
            return;
        }

        egui::ScrollArea::vertical()
            .id_salt("schema_tree_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let schemas: Vec<_> = self.table_browser.schemas.iter().cloned().collect();
                for schema in schemas {
                    let is_expanded = self.table_browser.expanded_schemas.contains(&schema.name);

                    let header_response = egui::CollapsingHeader::new(&schema.name)
                        .default_open(schema.name == "public")
                        .show(ui, |ui| {
                            // Show loading indicator or tables
                            if self.table_browser.loading_tables
                                && self.table_browser.selected_schema.as_ref() == Some(&schema.name)
                            {
                                ui.spinner();
                            } else if self.table_browser.selected_schema.as_ref() == Some(&schema.name) {
                                for table in &self.table_browser.tables.clone() {
                                    let selected =
                                        self.table_browser.selected_table.as_ref() == Some(&table.name);
                                    let icon = if table.table_type == "VIEW" { "V" } else { "T" };
                                    let label = format!("[{}] {}", icon, table.name);
                                    if ui.selectable_label(selected, label).clicked() {
                                        self.table_browser.selected_table = Some(table.name.clone());
                                        self.table_browser.current_page = 0;
                                        if let Some(db) = &self.table_browser.selected_database.clone() {
                                            self.fetch_table_structure(
                                                cluster,
                                                db,
                                                &schema.name,
                                                &table.name,
                                            );
                                            self.fetch_table_data(
                                                cluster,
                                                db,
                                                &schema.name,
                                                &table.name,
                                                0,
                                            );
                                        }
                                    }
                                }
                                if self.table_browser.tables.is_empty() {
                                    ui.label("(no tables)");
                                }
                            }
                        });

                    // Handle schema expansion
                    if header_response.fully_open() && !is_expanded {
                        self.table_browser.expanded_schemas.insert(schema.name.clone());
                        self.table_browser.selected_schema = Some(schema.name.clone());
                        if let Some(db) = &self.table_browser.selected_database.clone() {
                            self.fetch_schema_tables(cluster, db, &schema.name);
                        }
                    } else if !header_response.fully_open() && is_expanded {
                        self.table_browser.expanded_schemas.remove(&schema.name);
                    }
                }
            });
    }

    fn render_table_structure(&mut self, ui: &mut egui::Ui) {
        if self.table_browser.loading_structure {
            ui.spinner();
            return;
        }

        if let Some(structure) = &self.table_browser.table_structure {
            ui.label(format!("{}.{}", structure.schema, structure.table));

            // Columns
            egui::CollapsingHeader::new(format!("Columns ({})", structure.columns.len()))
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("columns_grid")
                        .num_columns(5)
                        .striped(true)
                        .show(ui, |ui| {
                            ui.strong("Name");
                            ui.strong("Type");
                            ui.strong("Nullable");
                            ui.strong("Default");
                            ui.strong("PK");
                            ui.end_row();

                            for col in &structure.columns {
                                ui.label(&col.name);
                                ui.label(&col.data_type);
                                ui.label(if col.is_nullable { "YES" } else { "NO" });
                                ui.label(col.column_default.as_deref().unwrap_or("-"));
                                ui.label(if col.is_primary_key { "PK" } else { "" });
                                ui.end_row();
                            }
                        });
                });

            // Constraints
            if !structure.constraints.is_empty() {
                egui::CollapsingHeader::new(format!("Constraints ({})", structure.constraints.len()))
                    .show(ui, |ui| {
                        for constraint in &structure.constraints {
                            let fk_info = if constraint.constraint_type == "FOREIGN KEY" {
                                format!(
                                    " -> {}",
                                    constraint.foreign_table.as_deref().unwrap_or("?")
                                )
                            } else {
                                String::new()
                            };
                            ui.label(format!(
                                "{}: {} ({}){}",
                                constraint.name,
                                constraint.constraint_type,
                                constraint.columns.join(", "),
                                fk_info
                            ));
                        }
                    });
            }

            // Indexes
            if !structure.indexes.is_empty() {
                egui::CollapsingHeader::new(format!("Indexes ({})", structure.indexes.len())).show(
                    ui,
                    |ui| {
                        for index in &structure.indexes {
                            let unique = if index.is_unique { " UNIQUE" } else { "" };
                            let primary = if index.is_primary { " (PRIMARY)" } else { "" };
                            ui.label(format!(
                                "{}: [{}] {}{}{}",
                                index.name,
                                index.index_type,
                                index.columns.join(", "),
                                unique,
                                primary
                            ));
                        }
                    },
                );
            }
        }
    }

    fn render_table_data(&mut self, ui: &mut egui::Ui, cluster: &str) {
        if self.table_browser.loading_data {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Loading data...");
            });
            return;
        }

        let has_pk = self
            .table_browser
            .table_structure
            .as_ref()
            .map(|s| !s.primary_key_columns.is_empty())
            .unwrap_or(false);

        // Toolbar
        ui.horizontal(|ui| {
            if has_pk {
                if ui.button("+ Insert Row").clicked() {
                    self.table_browser.show_insert_dialog = true;
                    self.table_browser.new_row_values.clear();
                }
            } else {
                ui.add_enabled(false, egui::Button::new("+ Insert Row"))
                    .on_disabled_hover_text("Table has no primary key - inline editing disabled");
            }

            if ui.button("Export CSV").clicked() {
                if let Some(data) = &self.table_browser.table_data {
                    let csv = dbctl::postgres::export_to_csv(data);
                    ui.output_mut(|o| o.copied_text = csv);
                    self.status_message = "CSV copied to clipboard".to_string();
                }
            }

            if ui.button("Export JSON").clicked() {
                if let Some(data) = &self.table_browser.table_data {
                    if let Ok(json) = dbctl::postgres::export_to_json(data) {
                        ui.output_mut(|o| o.copied_text = json);
                        self.status_message = "JSON copied to clipboard".to_string();
                    }
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("{} rows total", self.table_browser.total_rows));
            });
        });

        ui.separator();

        if let Some(data) = &self.table_browser.table_data.clone() {
            if data.rows.is_empty() {
                ui.label("No data in table");
            } else {
                // Data grid
                egui::ScrollArea::both()
                    .id_salt("table_data_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Grid::new("data_grid")
                            .num_columns(data.columns.len() + if has_pk { 1 } else { 0 })
                            .striped(true)
                            .show(ui, |ui| {
                                // Header
                                for col in &data.columns {
                                    ui.strong(col);
                                }
                                if has_pk {
                                    ui.strong("Actions");
                                }
                                ui.end_row();

                                // Data rows
                                for (_row_idx, row) in data.rows.iter().enumerate() {
                                    for col in &data.columns {
                                        let value = row.get(col).and_then(|v| v.as_ref());
                                        match value {
                                            Some(v) => {
                                                // Truncate long values for display
                                                let display = if v.len() > 50 {
                                                    format!("{}...", &v[..50])
                                                } else {
                                                    v.clone()
                                                };
                                                ui.label(display).on_hover_text(v);
                                            }
                                            None => {
                                                ui.label(
                                                    egui::RichText::new("NULL")
                                                        .italics()
                                                        .color(egui::Color32::GRAY),
                                                );
                                            }
                                        }
                                    }

                                    // Actions column
                                    if has_pk {
                                        if let Some(structure) = &self.table_browser.table_structure {
                                            if let Some(pk_col) = structure.primary_key_columns.first() {
                                                if let Some(pk_val) =
                                                    row.get(pk_col).and_then(|v| v.as_ref())
                                                {
                                                    if ui.small_button("Delete").clicked() {
                                                        self.table_browser.show_delete_dialog = true;
                                                        self.table_browser.pending_delete_pk =
                                                            Some(pk_val.clone());
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    ui.end_row();
                                }
                            });
                    });
            }

            ui.separator();

            // Pagination controls
            ui.horizontal(|ui| {
                let total_pages = ((data.total_rows as usize).saturating_sub(1)) / data.page_size + 1;

                if ui
                    .add_enabled(data.page > 0, egui::Button::new("<"))
                    .clicked()
                {
                    self.table_browser.current_page = data.page.saturating_sub(1);
                    self.refresh_table_data(cluster);
                }

                ui.label(format!("Page {} of {}", data.page + 1, total_pages.max(1)));

                if ui
                    .add_enabled(data.page + 1 < total_pages, egui::Button::new(">"))
                    .clicked()
                {
                    self.table_browser.current_page = data.page + 1;
                    self.refresh_table_data(cluster);
                }

                ui.separator();

                ui.label("Page size:");
                egui::ComboBox::from_id_salt("page_size")
                    .selected_text(format!("{}", self.table_browser.page_size))
                    .show_ui(ui, |ui| {
                        for size in [25, 50, 100, 250] {
                            if ui
                                .selectable_value(
                                    &mut self.table_browser.page_size,
                                    size,
                                    format!("{}", size),
                                )
                                .clicked()
                            {
                                self.table_browser.current_page = 0;
                                self.refresh_table_data(cluster);
                            }
                        }
                    });
            });
        }

        // Render dialogs
        self.render_insert_row_dialog(ui.ctx());
        self.render_delete_row_dialog(ui.ctx());
    }

    fn render_insert_row_dialog(&mut self, ctx: &egui::Context) {
        if !self.table_browser.show_insert_dialog {
            return;
        }

        let structure = match &self.table_browser.table_structure {
            Some(s) => s.clone(),
            None => return,
        };

        egui::Window::new("Insert Row")
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                egui::Grid::new("insert_row_grid")
                    .num_columns(2)
                    .show(ui, |ui| {
                        for col in &structure.columns {
                            let label = if col.is_primary_key {
                                format!("{} (PK)", col.name)
                            } else if !col.is_nullable && col.column_default.is_none() {
                                format!("{} *", col.name)
                            } else {
                                col.name.clone()
                            };
                            ui.label(label);

                            let value = self
                                .table_browser
                                .new_row_values
                                .entry(col.name.clone())
                                .or_default();
                            ui.text_edit_singleline(value);
                            ui.end_row();
                        }
                    });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Insert").clicked() {
                        if let (Some(cluster), Some(db), Some(schema), Some(table)) = (
                            self.selected_cluster().map(|c| c.name.clone()),
                            self.table_browser.selected_database.clone(),
                            self.table_browser.selected_schema.clone(),
                            self.table_browser.selected_table.clone(),
                        ) {
                            // Filter out empty values
                            self.table_browser.new_row_values.retain(|_, v| !v.is_empty());
                            self.insert_table_row(&cluster, &db, &schema, &table);
                            self.table_browser.show_insert_dialog = false;
                            // Refresh data after a short delay
                            self.refresh_table_data(&cluster);
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.table_browser.show_insert_dialog = false;
                        self.table_browser.new_row_values.clear();
                    }
                });
            });
    }

    fn render_delete_row_dialog(&mut self, ctx: &egui::Context) {
        if !self.table_browser.show_delete_dialog {
            return;
        }

        let pk_value = match &self.table_browser.pending_delete_pk {
            Some(v) => v.clone(),
            None => return,
        };

        egui::Window::new("Confirm Delete")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!("Delete row with primary key = '{}'?", pk_value));
                ui.colored_label(egui::Color32::RED, "This cannot be undone!");

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Delete").clicked() {
                        if let (Some(cluster), Some(db), Some(schema), Some(table), Some(structure)) = (
                            self.selected_cluster().map(|c| c.name.clone()),
                            self.table_browser.selected_database.clone(),
                            self.table_browser.selected_schema.clone(),
                            self.table_browser.selected_table.clone(),
                            self.table_browser.table_structure.clone(),
                        ) {
                            if let Some(pk_col) = structure.primary_key_columns.first() {
                                self.delete_table_row(&cluster, &db, &schema, &table, pk_col, &pk_value);
                                self.table_browser.show_delete_dialog = false;
                                self.table_browser.pending_delete_pk = None;
                                // Refresh data
                                self.refresh_table_data(&cluster);
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.table_browser.show_delete_dialog = false;
                        self.table_browser.pending_delete_pk = None;
                    }
                });
            });
    }

    fn render_install_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_install_dialog {
            return;
        }

        egui::Window::new("Install PostgreSQL")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Version:");
                    ui.text_edit_singleline(&mut self.install_version);
                });

                ui.label("Examples: 17, 16, 15.8, 14.12");

                ui.separator();

                let version = self.install_version.clone();
                ui.horizontal(|ui| {
                    if ui.button("Install").clicked() {
                        self.install_postgres(&version);
                        self.show_install_dialog = false;
                        self.status_message = format!("Installing PostgreSQL {}...", version);
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_install_dialog = false;
                    }
                });
            });
    }

    fn render_create_cluster_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_create_cluster_dialog {
            return;
        }

        egui::Window::new("Create Cluster")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("create_cluster_grid")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut self.new_cluster_name);
                        ui.end_row();

                        ui.label("Version:");
                        egui::ComboBox::from_id_salt("version_select")
                            .selected_text(&self.new_cluster_version)
                            .show_ui(ui, |ui| {
                                for v in &self.installed_versions {
                                    ui.selectable_value(&mut self.new_cluster_version, v.clone(), v);
                                }
                            });
                        ui.end_row();

                        ui.label("Port:");
                        ui.text_edit_singleline(&mut self.new_cluster_port);
                        ui.end_row();

                        ui.label("Superuser:");
                        ui.text_edit_singleline(&mut self.new_cluster_username);
                        ui.end_row();
                    });

                if self.installed_versions.is_empty() {
                    ui.colored_label(egui::Color32::RED, "No PostgreSQL versions installed!");
                }

                ui.separator();

                ui.horizontal(|ui| {
                    let can_create = !self.new_cluster_name.is_empty()
                        && !self.new_cluster_version.is_empty()
                        && !self.new_cluster_port.is_empty();

                    if ui.add_enabled(can_create, egui::Button::new("Create")).clicked() {
                        self.create_cluster();
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_create_cluster_dialog = false;
                    }
                });
            });
    }

    fn render_remove_cluster_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_remove_cluster_dialog {
            return;
        }

        let cluster_name = self.selected_cluster()
            .map(|c| c.name.clone())
            .unwrap_or_default();

        egui::Window::new("Confirm Remove")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!("Are you sure you want to remove cluster '{}'?", cluster_name));
                ui.colored_label(egui::Color32::RED, "This will delete all data!");

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Remove").clicked() {
                        self.remove_cluster(&cluster_name);
                        self.show_remove_cluster_dialog = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_remove_cluster_dialog = false;
                    }
                });
            });
    }

    fn render_drop_db_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_drop_db_dialog {
            return;
        }

        let db_name = self.pending_drop_db.clone().unwrap_or_default();
        let cluster_name = self.selected_cluster()
            .map(|c| c.name.clone())
            .unwrap_or_default();

        egui::Window::new("Confirm Drop Database")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!("Are you sure you want to drop database '{}'?", db_name));
                ui.colored_label(egui::Color32::RED, "This cannot be undone!");

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Drop").clicked() {
                        self.drop_database(&cluster_name, &db_name);
                        self.show_drop_db_dialog = false;
                        self.pending_drop_db = None;
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_drop_db_dialog = false;
                        self.pending_drop_db = None;
                    }
                });
            });
    }
}
