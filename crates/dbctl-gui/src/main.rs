mod ai_chat;
mod command_palette;
mod dialogs;
mod schema_designer;
mod sql_state;
mod state;
mod syntax;
mod table_browser;
mod tabs;
pub mod theme;

use std::sync::mpsc;

use display_info::DisplayInfo;
use eframe::egui;

use schema_designer::SchemaDesignerState;
use sql_state::SqlState;
use state::{ActiveConnection, AsyncResult, ClusterInfo, ConnectionInfo, Tab};
use table_browser::TableBrowserState;
use theme::{GuiConfig, Theme, ThemeMode};

fn main() -> eframe::Result<()> {
    // Get screen dimensions and set window to half the size
    let (window_width, window_height) = get_half_screen_size();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([window_width, window_height])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "dbctl - PostgreSQL Manager",
        options,
        Box::new(|cc| Ok(Box::new(DbctlApp::new(cc)))),
    )
}

/// Get half the primary display's dimensions, with fallback defaults
fn get_half_screen_size() -> (f32, f32) {
    if let Ok(displays) = DisplayInfo::all() {
        // Find primary display, or use first one
        let display = displays
            .iter()
            .find(|d| d.is_primary)
            .or_else(|| displays.first());

        if let Some(d) = display {
            return (d.width as f32 / 2.0, d.height as f32 / 2.0);
        }
    }
    // Fallback if we can't get display info
    (1200.0, 700.0)
}

pub struct DbctlApp {
    // Cluster state
    pub clusters: Vec<ClusterInfo>,
    pub selected_cluster: Option<usize>,

    // Connection state (remote PostgreSQL)
    pub connections: Vec<ConnectionInfo>,
    pub active_connection: ActiveConnection,

    // Connection dialog state
    pub show_add_connection_dialog: bool,
    pub show_edit_connection_dialog: bool,
    pub editing_connection_index: Option<usize>,
    pub conn_form_name: String,
    pub conn_form_host: String,
    pub conn_form_port: String,
    pub conn_form_database: String,
    pub conn_form_username: String,
    pub conn_form_password: String,
    pub conn_form_ssl_mode: dbctl::SslMode,
    pub test_connection_result: Option<Result<String, String>>,
    pub testing_connection: bool,

    // Tab state
    pub current_tab: Tab,

    // Database state
    pub databases: Vec<String>,
    pub selected_database: Option<usize>,
    pub new_db_name: String,

    // Table browser state
    pub table_browser: TableBrowserState,

    // Schema designer state
    pub schema_designer: SchemaDesignerState,
    pub layout_cache: schema_designer::SchemaLayoutCache,

    // User state
    pub users: Vec<dbctl::UserInfo>,
    pub new_user_name: String,
    pub new_user_password: String,
    pub new_user_superuser: bool,

    // Logs state
    pub logs_content: String,
    pub logs_lines: usize,

    // SQL state (enhanced with tabs, history, saved queries)
    pub sql_state: SqlState,

    // Install state
    pub show_install_dialog: bool,
    pub install_version: String,
    pub installed_versions: Vec<String>,

    // Create cluster state
    pub show_create_cluster_dialog: bool,
    pub new_cluster_name: String,
    pub new_cluster_version: String,
    pub new_cluster_port: String,
    pub new_cluster_username: String,

    // Backup/restore state
    pub backup_path: String,
    pub restore_path: String,

    // Confirmation dialogs
    pub show_remove_cluster_dialog: bool,
    pub show_drop_db_dialog: bool,
    pub pending_drop_db: Option<String>,

    // Status
    pub status_message: String,

    // Async handling
    runtime: tokio::runtime::Runtime,
    result_rx: mpsc::Receiver<AsyncResult>,
    result_tx: mpsc::Sender<AsyncResult>,

    // Command palette
    pub command_palette: command_palette::CommandPaletteState,

    // Theme
    pub theme_mode: ThemeMode,
    pub theme: Theme,
    theme_applied: bool,

    // GUI config (for session persistence)
    pub gui_config: GuiConfig,
    session_restored: bool,

    // AI Chat
    pub ai_chat: ai_chat::AiChatState,
    pub ai_config: ai_chat::AiConfig,
    pub show_api_key_dialog: bool,
    pub api_key_input: String,
}

impl DbctlApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        let (result_tx, result_rx) = mpsc::channel();

        // Load saved connections
        let connections = Self::load_connections();

        // Load GUI config (theme, session state)
        let gui_config = GuiConfig::load();
        let theme_mode = gui_config.theme_mode;
        let theme = Theme::from_mode(theme_mode);
        let initial_tab = gui_config.last_tab.clone().unwrap_or_default();

        // Load AI config
        let ai_config = ai_chat::AiConfig::load();

        // Load schema layout cache
        let layout_cache = schema_designer::SchemaLayoutCache::load();

        let mut app = Self {
            clusters: Vec::new(),
            selected_cluster: None,

            // Remote connections
            connections,
            active_connection: ActiveConnection::None,

            // Connection dialog
            show_add_connection_dialog: false,
            show_edit_connection_dialog: false,
            editing_connection_index: None,
            conn_form_name: String::new(),
            conn_form_host: String::new(),
            conn_form_port: String::from("5432"),
            conn_form_database: String::from("postgres"),
            conn_form_username: String::new(),
            conn_form_password: String::new(),
            conn_form_ssl_mode: dbctl::SslMode::Disable,
            test_connection_result: None,
            testing_connection: false,

            current_tab: initial_tab,
            databases: Vec::new(),
            selected_database: None,
            new_db_name: String::new(),
            table_browser: TableBrowserState::new(),
            schema_designer: SchemaDesignerState::new(),
            layout_cache: layout_cache.clone(),
            users: Vec::new(),
            new_user_name: String::new(),
            new_user_password: String::new(),
            new_user_superuser: false,
            logs_content: String::new(),
            logs_lines: 100,
            sql_state: SqlState::default(),
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
            command_palette: command_palette::CommandPaletteState::default(),
            theme_mode,
            theme,
            theme_applied: false,
            gui_config,
            session_restored: false,
            ai_chat: ai_chat::AiChatState::new(),
            ai_config,
            show_api_key_dialog: false,
            api_key_input: String::new(),
        };

        // Populate database_layouts from the loaded cache
        app.schema_designer.database_layouts = layout_cache.to_layouts();

        app.refresh_clusters();
        app.refresh_installed_versions();
        app
    }

    /// Load saved connections from disk
    fn load_connections() -> Vec<ConnectionInfo> {
        match dbctl::ConnectionStore::load() {
            Ok(store) => store
                .connections
                .into_iter()
                .map(|profile| ConnectionInfo {
                    profile,
                    connected: false,
                    databases: Vec::new(),
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Save connections to disk
    pub fn save_connections(&self) {
        let store = dbctl::ConnectionStore {
            connections: self
                .connections
                .iter()
                .map(|c| c.profile.clone())
                .collect(),
        };
        let _ = store.save();
    }

    /// Reset connection form to defaults
    pub fn reset_connection_form(&mut self) {
        self.conn_form_name = String::new();
        self.conn_form_host = String::new();
        self.conn_form_port = String::from("5432");
        self.conn_form_database = String::from("postgres");
        self.conn_form_username = String::new();
        self.conn_form_password = String::new();
        self.conn_form_ssl_mode = dbctl::SslMode::Disable;
        self.test_connection_result = None;
        self.testing_connection = false;
    }

    /// Load connection into form for editing
    pub fn load_connection_to_form(&mut self, conn: &ConnectionInfo) {
        self.conn_form_name = conn.profile.name.clone();
        self.conn_form_host = conn.profile.host.clone();
        self.conn_form_port = conn.profile.port.to_string();
        self.conn_form_database = conn.profile.database.clone();
        self.conn_form_username = conn.profile.username.clone();
        self.conn_form_password = conn.profile.password.clone();
        self.conn_form_ssl_mode = conn.profile.ssl_mode;
        self.test_connection_result = None;
        self.testing_connection = false;
    }

    /// Build ConnectionProfile from form
    pub fn build_connection_from_form(&self) -> Result<dbctl::ConnectionProfile, String> {
        if self.conn_form_name.trim().is_empty() {
            return Err("Name is required".to_string());
        }
        if self.conn_form_host.trim().is_empty() {
            return Err("Host is required".to_string());
        }
        let port: u16 = self
            .conn_form_port
            .parse()
            .map_err(|_| "Invalid port".to_string())?;

        Ok(dbctl::ConnectionProfile {
            name: self.conn_form_name.trim().to_string(),
            host: self.conn_form_host.trim().to_string(),
            port,
            database: self.conn_form_database.trim().to_string(),
            username: self.conn_form_username.trim().to_string(),
            password: self.conn_form_password.clone(),
            ssl_mode: self.conn_form_ssl_mode,
        })
    }

    pub fn refresh_clusters(&mut self) {
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
                let running_clusters: Vec<String> = self
                    .clusters
                    .iter()
                    .filter(|c| c.running)
                    .map(|c| c.name.clone())
                    .collect();
                for name in running_clusters {
                    self.fetch_cluster_stats(&name);
                }

                // Restore session on first load
                if !self.session_restored {
                    self.restore_session();
                }
            }
            Err(e) => {
                self.status_message = format!("Error: {}", e);
            }
        }
    }

    /// Restore session state from saved config (cluster, tab, database)
    fn restore_session(&mut self) {
        self.session_restored = true;

        // Restore last cluster by name
        if let Some(ref cluster_name) = self.gui_config.last_cluster {
            if let Some(i) = self.clusters.iter().position(|c| &c.name == cluster_name) {
                self.selected_cluster = Some(i);
                self.active_connection = ActiveConnection::Cluster(i);

                // Fetch data for the cluster if running
                if let Some(cluster) = self.clusters.get(i).cloned() {
                    if cluster.running {
                        self.fetch_databases(&cluster.name);
                        self.fetch_users(&cluster.name);
                    }
                    self.refresh_logs(&cluster.name);
                }
            }
        }

        // Restore last tab (already set in new() but may have been overridden)
        if let Some(ref tab) = self.gui_config.last_tab {
            self.current_tab = tab.clone();
        }

        // Note: Database restoration happens after databases are loaded
        // via process_async_results -> AsyncResult::Databases
    }

    pub fn refresh_installed_versions(&mut self) {
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

    pub fn selected_cluster(&self) -> Option<&ClusterInfo> {
        self.selected_cluster.and_then(|i| self.clusters.get(i))
    }

    /// Set the theme mode and persist preference
    pub fn set_theme(&mut self, mode: ThemeMode) {
        self.theme_mode = mode;
        self.theme = Theme::from_mode(mode);
        self.theme_applied = false; // Trigger re-apply on next frame

        // Persist the preference
        self.gui_config.theme_mode = mode;
        self.gui_config.save();
    }

    /// Save current session state (cluster, tab, database) for restoration on next launch
    pub fn save_session(&mut self) {
        // Save current cluster name
        self.gui_config.last_cluster = self.selected_cluster().map(|c| c.name.clone());

        // Save current tab
        self.gui_config.last_tab = Some(self.current_tab.clone());

        // Save current database (prefer schema designer, then table browser)
        self.gui_config.last_database = self
            .schema_designer
            .selected_database
            .clone()
            .or_else(|| self.table_browser.selected_database.clone());

        self.gui_config.save();
    }

    pub fn fetch_databases(&mut self, cluster: &str) {
        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        self.runtime.spawn(async move {
            match dbctl::postgres::list_databases(&cluster).await {
                Ok(dbs) => {
                    let _ = tx.send(AsyncResult::Databases(cluster, dbs));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(e.to_string()));
                }
            }
        });
    }

    pub fn fetch_users(&mut self, cluster: &str) {
        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        self.runtime.spawn(async move {
            match dbctl::postgres::list_users(&cluster).await {
                Ok(users) => {
                    let _ = tx.send(AsyncResult::Users(cluster, users));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(e.to_string()));
                }
            }
        });
    }

    fn fetch_cluster_stats(&mut self, cluster: &str) {
        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        self.runtime.spawn(async move {
            match dbctl::postgres::get_cluster_stats(&cluster).await {
                Ok(stats) => {
                    let _ = tx.send(AsyncResult::Stats(cluster, stats));
                }
                Err(_) => {} // Ignore stats errors
            }
        });
    }

    // Table browser async methods
    pub fn fetch_schemas(&mut self, cluster: &str, database: &str) {
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

    pub fn fetch_schema_tables(&mut self, cluster: &str, database: &str, schema: &str) {
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

    pub fn fetch_table_structure(
        &mut self,
        cluster: &str,
        database: &str,
        schema: &str,
        table: &str,
    ) {
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

    pub fn fetch_table_data(
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

    pub fn refresh_table_data(&mut self, cluster: &str) {
        if let (Some(db), Some(schema), Some(table)) = (
            self.table_browser.selected_database.clone(),
            self.table_browser.selected_schema.clone(),
            self.table_browser.selected_table.clone(),
        ) {
            self.fetch_table_data(cluster, &db, &schema, &table, self.table_browser.current_page);
        }
    }

    // Schema designer methods

    /// Save the current schema layout to memory cache and persist to disk
    fn save_and_persist_layout(&mut self) {
        self.schema_designer.save_layout_for_database();
        // Sync the in-memory cache to the persistent cache and save to disk
        self.layout_cache = schema_designer::SchemaLayoutCache::from_layouts(&self.schema_designer.database_layouts);
        self.layout_cache.save();
    }

    pub fn load_schema_for_designer(&mut self, cluster: &str, database: &str, schema: &str) {
        // Save the current layout before switching databases
        self.save_and_persist_layout();

        self.schema_designer.loading_schema = true;
        self.schema_designer.reset();

        // Restore any cached layout for the target database
        self.schema_designer.restore_layout_for_database(database);

        self.schema_designer.selected_database = Some(database.to_string());
        self.schema_designer.selected_schema = schema.to_string();

        self.spawn_schema_load(cluster, database, schema);

        // Save session with new database selection
        self.save_session();
    }

    /// Reload schema after migration - preserves action history
    pub fn reload_schema_for_designer(&mut self, cluster: &str, database: &str, schema: &str) {
        self.schema_designer.loading_schema = true;
        self.schema_designer.reset_for_reload();
        self.schema_designer.selected_database = Some(database.to_string());
        self.schema_designer.selected_schema = schema.to_string();

        self.spawn_schema_load(cluster, database, schema);
    }

    fn spawn_schema_load(&mut self, cluster: &str, database: &str, schema: &str) {
        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        let database = database.to_string();
        let schema = schema.to_string();

        self.runtime.spawn(async move {
            // Get list of tables first
            match dbctl::postgres::list_tables(&cluster, &database, &schema).await {
                Ok(tables) => {
                    let mut structures = Vec::new();
                    for table in tables {
                        match dbctl::postgres::get_table_structure(&cluster, &database, &schema, &table.name).await {
                            Ok(structure) => structures.push(structure),
                            Err(e) => {
                                let _ = tx.send(AsyncResult::SchemaLoadError(format!(
                                    "Failed to load table '{}': {}",
                                    table.name, e
                                )));
                                return;
                            }
                        }
                    }
                    let _ = tx.send(AsyncResult::SchemaLoaded(cluster, database, structures));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::SchemaLoadError(e.to_string()));
                }
            }
        });
    }

    pub fn apply_migration(&mut self, cluster: &str, database: &str, statements: Vec<String>) {
        self.schema_designer.applying_migration = true;
        self.schema_designer.migration_error = None;

        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        let database = database.to_string();

        self.runtime.spawn(async move {
            match dbctl::postgres::execute_migration(&cluster, &database, &statements).await {
                Ok(()) => {
                    let _ = tx.send(AsyncResult::MigrationApplied(format!(
                        "Successfully applied {} statement(s)",
                        statements.len()
                    )));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::MigrationFailed(e.to_string()));
                }
            }
        });
    }

    pub fn insert_table_row(&mut self, cluster: &str, database: &str, schema: &str, table: &str) {
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

    pub fn delete_table_row(
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
            match dbctl::postgres::delete_row(
                &cluster, &database, &schema, &table, &pk_column, &pk_value,
            )
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

    pub fn create_database(&mut self, cluster: &str, database: &str) {
        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        let database = database.to_string();
        self.runtime.spawn(async move {
            match dbctl::postgres::create_database(&cluster, &database).await {
                Ok(()) => {
                    let _ = tx.send(AsyncResult::Message(format!(
                        "Created database '{}'",
                        database
                    )));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(e.to_string()));
                }
            }
        });
    }

    pub fn drop_database(&mut self, cluster: &str, database: &str) {
        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        let database = database.to_string();
        self.runtime.spawn(async move {
            match dbctl::postgres::drop_database(&cluster, &database).await {
                Ok(()) => {
                    let _ = tx.send(AsyncResult::Message(format!(
                        "Dropped database '{}'",
                        database
                    )));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(e.to_string()));
                }
            }
        });
    }

    pub fn create_user(
        &mut self,
        cluster: &str,
        username: &str,
        password: Option<&str>,
        superuser: bool,
    ) {
        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        let username = username.to_string();
        let password = password.map(|s| s.to_string());
        self.runtime.spawn(async move {
            match dbctl::postgres::create_user(&cluster, &username, password.as_deref(), superuser)
                .await
            {
                Ok(()) => {
                    let _ = tx.send(AsyncResult::Message(format!("Created user '{}'", username)));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(e.to_string()));
                }
            }
        });
    }

    pub fn install_postgres(&mut self, version: &str) {
        let tx = self.result_tx.clone();
        let version = version.to_string();
        self.runtime.spawn(async move {
            match dbctl::server::install_postgres(&version).await {
                Ok(()) => {
                    let _ = tx.send(AsyncResult::Message(format!(
                        "Installed PostgreSQL {}",
                        version
                    )));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(e.to_string()));
                }
            }
        });
    }

    // Remote connection methods
    pub fn test_connection(&mut self, profile: dbctl::ConnectionProfile) {
        self.testing_connection = true;
        self.test_connection_result = None;
        let tx = self.result_tx.clone();
        let name = profile.name.clone();
        let source = dbctl::ConnectionSource::Remote(profile);
        self.runtime.spawn(async move {
            match dbctl::test_connection(&source).await {
                Ok(version) => {
                    let _ = tx.send(AsyncResult::ConnectionTest(name, Ok(version)));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::ConnectionTest(name, Err(e.to_string())));
                }
            }
        });
    }

    fn fetch_remote_databases(&mut self, profile: &dbctl::ConnectionProfile) {
        let tx = self.result_tx.clone();
        let name = profile.name.clone();
        let source = dbctl::ConnectionSource::Remote(profile.clone());
        self.runtime.spawn(async move {
            match dbctl::list_databases_with_source(&source).await {
                Ok(dbs) => {
                    let _ = tx.send(AsyncResult::RemoteDatabases(name, dbs));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(e.to_string()));
                }
            }
        });
    }

    fn fetch_remote_users(&mut self, profile: &dbctl::ConnectionProfile) {
        let tx = self.result_tx.clone();
        let name = profile.name.clone();
        let source = dbctl::ConnectionSource::Remote(profile.clone());
        self.runtime.spawn(async move {
            match dbctl::list_users_with_source(&source).await {
                Ok(users) => {
                    let _ = tx.send(AsyncResult::RemoteUsers(name, users));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(e.to_string()));
                }
            }
        });
    }

    fn fetch_remote_schema_tables(&mut self, conn_name: &str, database: &str, schema: &str) {
        self.table_browser.loading_tables = true;
        let tx = self.result_tx.clone();
        let conn_name = conn_name.to_string();
        let database = database.to_string();
        let schema = schema.to_string();

        // Find the connection profile
        let profile = self
            .connections
            .iter()
            .find(|c| c.profile.name == conn_name)
            .map(|c| c.profile.clone());

        if let Some(profile) = profile {
            let source = dbctl::ConnectionSource::Remote(profile);
            self.runtime.spawn(async move {
                match dbctl::list_tables_with_source(&source, &database, &schema).await {
                    Ok(tables) => {
                        let _ = tx.send(AsyncResult::RemoteTables(
                            conn_name, database, schema, tables,
                        ));
                    }
                    Err(e) => {
                        let _ = tx.send(AsyncResult::Error(e.to_string()));
                    }
                }
            });
        }
    }

    fn process_async_results(&mut self) {
        while let Ok(result) = self.result_rx.try_recv() {
            match result {
                AsyncResult::Databases(cluster, dbs) => {
                    if self.selected_cluster().map(|c| &c.name) == Some(&cluster) {
                        self.databases = dbs;
                        // Update current SQL tab database if it's empty
                        if !self.databases.is_empty() {
                            let tab = self.sql_state.current_tab_mut();
                            if tab.database.is_empty() {
                                tab.database = self.databases[0].clone();
                            }
                        }

                        // Restore last database on session restore
                        if self.schema_designer.selected_database.is_none() {
                            if let Some(db_name) = self.gui_config.last_database.clone() {
                                if self.databases.contains(&db_name) {
                                    // Restore any cached layout for this database
                                    self.schema_designer.restore_layout_for_database(&db_name);
                                    self.schema_designer.selected_database = Some(db_name.clone());
                                    self.schema_designer.selected_schema = "public".to_string();
                                    self.spawn_schema_load(&cluster, &db_name, "public");
                                }
                            }
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
                AsyncResult::SqlTabResult(tab_id, result) => {
                    // Find the tab by ID and update its result
                    if let Some(tab) = self.sql_state.tabs.iter_mut().find(|t| t.id == tab_id) {
                        tab.result = result;
                        tab.is_executing = false;
                    }
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
                            self.table_browser
                                .expanded_schemas
                                .insert("public".to_string());
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
                // Remote connection results
                AsyncResult::ConnectionTest(_conn_name, result) => {
                    self.testing_connection = false;
                    self.test_connection_result = Some(result);
                }
                AsyncResult::RemoteDatabases(conn_name, dbs) => {
                    if let Some(conn) = self
                        .connections
                        .iter_mut()
                        .find(|c| c.profile.name == conn_name)
                    {
                        conn.databases = dbs.clone();
                        conn.connected = true;
                    }
                    // If this is the active connection, update databases list
                    if let ActiveConnection::Remote(idx) = &self.active_connection {
                        if self.connections.get(*idx).map(|c| &c.profile.name) == Some(&conn_name) {
                            self.databases = dbs;
                            if !self.databases.is_empty() {
                                let tab = self.sql_state.current_tab_mut();
                                if tab.database.is_empty() {
                                    tab.database = self.databases[0].clone();
                                }
                            }
                        }
                    }
                }
                AsyncResult::RemoteUsers(conn_name, users) => {
                    if let ActiveConnection::Remote(idx) = &self.active_connection {
                        if self.connections.get(*idx).map(|c| &c.profile.name) == Some(&conn_name) {
                            self.users = users;
                        }
                    }
                }
                AsyncResult::RemoteStats(_conn_name, _stats) => {
                    // Stats for remote connections - currently not used in UI
                }
                AsyncResult::RemoteSchemas(conn_name, database, schemas) => {
                    if let ActiveConnection::Remote(idx) = &self.active_connection {
                        if self.connections.get(*idx).map(|c| &c.profile.name) == Some(&conn_name)
                            && self.table_browser.selected_database.as_ref() == Some(&database)
                        {
                            self.table_browser.schemas = schemas;
                            self.table_browser.loading_schemas = false;
                            if self.table_browser.schemas.iter().any(|s| s.name == "public") {
                                self.table_browser
                                    .expanded_schemas
                                    .insert("public".to_string());
                                self.table_browser.selected_schema = Some("public".to_string());
                                // Fetch tables for public schema
                                self.fetch_remote_schema_tables(&conn_name, &database, "public");
                            }
                        }
                    }
                }
                AsyncResult::RemoteTables(conn_name, database, schema, tables) => {
                    if let ActiveConnection::Remote(idx) = &self.active_connection {
                        if self.connections.get(*idx).map(|c| &c.profile.name) == Some(&conn_name)
                            && self.table_browser.selected_database.as_ref() == Some(&database)
                            && self.table_browser.selected_schema.as_ref() == Some(&schema)
                        {
                            self.table_browser.tables = tables;
                            self.table_browser.loading_tables = false;
                        }
                    }
                }
                AsyncResult::RemoteTableStructure(conn_name, database, schema, table, structure) => {
                    if let ActiveConnection::Remote(idx) = &self.active_connection {
                        if self.connections.get(*idx).map(|c| &c.profile.name) == Some(&conn_name)
                            && self.table_browser.selected_database.as_ref() == Some(&database)
                            && self.table_browser.selected_schema.as_ref() == Some(&schema)
                            && self.table_browser.selected_table.as_ref() == Some(&table)
                        {
                            self.table_browser.table_structure = Some(structure);
                            self.table_browser.loading_structure = false;
                        }
                    }
                }
                AsyncResult::RemoteTableData(conn_name, database, schema, table, data) => {
                    if let ActiveConnection::Remote(idx) = &self.active_connection {
                        if self.connections.get(*idx).map(|c| &c.profile.name) == Some(&conn_name)
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
                // Schema designer results
                AsyncResult::SchemaLoaded(_cluster, database, structures) => {
                    if self.schema_designer.selected_database.as_ref() == Some(&database) {
                        self.schema_designer.loading_schema = false;

                        // Convert TableStructure to our models
                        use schema_designer::{
                            ColumnModel, ConstraintModel, ConstraintType, IndexModel, SchemaModel, TableModel,
                        };
                        use schema_designer::canvas::auto_layout_tables;

                        let mut schema_model = SchemaModel::default();

                        for structure in structures {
                            let columns: Vec<ColumnModel> = structure
                                .columns
                                .iter()
                                .map(|c| ColumnModel {
                                    name: c.name.clone(),
                                    data_type: c.data_type.clone(),
                                    is_nullable: c.is_nullable,
                                    default_value: c.column_default.clone(),
                                    is_primary_key: c.is_primary_key,
                                })
                                .collect();

                            let constraints: Vec<ConstraintModel> = structure
                                .constraints
                                .iter()
                                .map(|c| {
                                    let constraint_type = match c.constraint_type.as_str() {
                                        "PRIMARY KEY" => ConstraintType::PrimaryKey,
                                        "FOREIGN KEY" => ConstraintType::ForeignKey,
                                        "UNIQUE" => ConstraintType::Unique,
                                        _ => ConstraintType::Check,
                                    };
                                    ConstraintModel {
                                        name: c.name.clone(),
                                        constraint_type,
                                        columns: c.columns.clone(),
                                        foreign_table: c.foreign_table.clone(),
                                        foreign_columns: c.foreign_columns.clone(),
                                        on_delete: None,
                                        on_update: None,
                                    }
                                })
                                .collect();

                            let indexes: Vec<IndexModel> = structure
                                .indexes
                                .iter()
                                .map(|i| IndexModel {
                                    name: i.name.clone(),
                                    columns: i.columns.clone(),
                                    is_unique: i.is_unique,
                                    index_type: i.index_type.clone(),
                                    is_primary: i.is_primary,
                                })
                                .collect();

                            // Restore preserved position if available, otherwise use ZERO
                            let position = self
                                .schema_designer
                                .preserved_positions
                                .get(&structure.table)
                                .copied()
                                .unwrap_or(egui::Pos2::ZERO);

                            let table = TableModel {
                                name: structure.table.clone(),
                                columns,
                                constraints,
                                indexes,
                                position,
                                collapsed: false,
                            };

                            schema_model.tables.insert(structure.table.clone(), table);
                        }

                        // Only auto-layout if no positions were preserved (fresh load)
                        // For reloads after migration, positions are restored from preserved_positions
                        if self.schema_designer.preserved_positions.is_empty() {
                            auto_layout_tables(&mut schema_model);
                        } else {
                            // Position any new tables that weren't in preserved_positions
                            use crate::schema_designer::canvas::find_non_overlapping_position;
                            let new_tables: Vec<String> = schema_model
                                .tables
                                .iter()
                                .filter(|(_, t)| t.position == egui::Pos2::ZERO)
                                .map(|(name, _)| name.clone())
                                .collect();

                            for table_name in new_tables {
                                // Find a non-overlapping position for this new table
                                let pos = find_non_overlapping_position(&schema_model, 200.0, 150.0);
                                if let Some(table) = schema_model.tables.get_mut(&table_name) {
                                    table.position = pos;
                                }
                            }
                            // Clear preserved positions after restoring
                            self.schema_designer.preserved_positions.clear();
                        }

                        // Set both original and working schemas
                        self.schema_designer.original_schema = schema_model.clone();
                        self.schema_designer.working_schema = schema_model;

                        // Initialize visual column order with semantic ordering
                        // (PK first, FK second, timestamps last)
                        self.schema_designer.visual_column_order.clear();
                        let table_names: Vec<String> = self
                            .schema_designer
                            .working_schema
                            .tables
                            .keys()
                            .cloned()
                            .collect();
                        for table_name in table_names {
                            self.schema_designer.ensure_visual_column_order(&table_name);
                        }
                    }
                }
                AsyncResult::SchemaLoadError(error) => {
                    self.schema_designer.loading_schema = false;
                    self.status_message = format!("Schema load error: {}", error);
                }
                AsyncResult::MigrationApplied(msg) => {
                    self.schema_designer.applying_migration = false;
                    self.schema_designer.show_apply_migration_dialog = false;
                    self.status_message = msg;

                    // Reload the schema to get the updated state (preserve action history)
                    if let (Some(cluster), Some(db)) = (
                        self.selected_cluster().map(|c| c.name.clone()),
                        self.schema_designer.selected_database.clone(),
                    ) {
                        let schema = self.schema_designer.selected_schema.clone();
                        self.reload_schema_for_designer(&cluster, &db, &schema);
                    }
                }
                AsyncResult::MigrationFailed(error) => {
                    self.schema_designer.applying_migration = false;
                    self.schema_designer.migration_error = Some(error.clone());
                    self.status_message = format!("Migration failed: {}", error);
                }
                // AI Chat results
                AsyncResult::AiChatResponse(response, proposal) => {
                    self.ai_chat.is_loading = false;
                    self.ai_chat.add_assistant_message(&response);
                    self.ai_chat.pending_changes = proposal;
                }
                AsyncResult::AiChatError(error) => {
                    self.ai_chat.is_loading = false;
                    self.ai_chat.error_message = Some(error);
                }
            }
        }
    }

    /// Send a message to the AI chat assistant
    pub fn send_ai_chat_message(&mut self) {
        if self.ai_chat.input_text.trim().is_empty() || self.ai_chat.is_loading {
            return;
        }

        let message = self.ai_chat.input_text.clone();
        self.ai_chat.add_user_message(&message);
        self.ai_chat.input_text.clear();
        self.ai_chat.is_loading = true;
        self.ai_chat.error_message = None;

        if !self.ai_config.has_api_key() {
            self.ai_chat.is_loading = false;
            self.ai_chat.error_message = Some("API key not configured. Go to Settings > AI API Key.".to_string());
            return;
        }

        let api_key = self.ai_config.api_key.clone();
        let model = self.ai_config.model.clone();
        let max_tokens = self.ai_config.max_tokens;

        let tx = self.result_tx.clone();
        let current_schema = self.schema_designer.working_schema.clone();
        let history: Vec<ai_chat::ChatMessage> = self.ai_chat.messages[..self.ai_chat.messages.len().saturating_sub(1)]
            .to_vec();

        self.runtime.spawn(async move {
            match ai_chat::claude_client::generate_schema_changes(
                &api_key,
                &model,
                max_tokens,
                &current_schema,
                &history,
                &message,
            )
            .await
            {
                Ok((response, proposal)) => {
                    let _ = tx.send(AsyncResult::AiChatResponse(response, proposal));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::AiChatError(e));
                }
            }
        });
    }

    pub fn start_cluster(&mut self, name: &str) {
        if let Err(e) = dbctl::server::ensure_cluster_running(name) {
            self.status_message = format!("Failed to start: {}", e);
        } else {
            self.status_message = format!("Started cluster '{}'", name);
            self.refresh_clusters();
        }
    }

    pub fn stop_cluster(&mut self, name: &str) {
        if let Err(e) = dbctl::server::stop_cluster(name) {
            self.status_message = format!("Failed to stop: {}", e);
        } else {
            self.status_message = format!("Stopped cluster '{}'", name);
            self.refresh_clusters();
        }
    }

    pub fn restart_cluster(&mut self, name: &str) {
        if let Err(e) = dbctl::server::restart_cluster(name) {
            self.status_message = format!("Failed to restart: {}", e);
        } else {
            self.status_message = format!("Restarted cluster '{}'", name);
            self.refresh_clusters();
        }
    }

    pub fn remove_cluster(&mut self, name: &str) {
        if let Err(e) = dbctl::server::remove_cluster(name) {
            self.status_message = format!("Failed to remove: {}", e);
        } else {
            self.status_message = format!("Removed cluster '{}'", name);
            self.selected_cluster = None;
            self.refresh_clusters();
        }
    }

    pub fn create_cluster(&mut self) {
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

    pub fn refresh_logs(&mut self, cluster: &str) {
        match dbctl::server::get_logs(cluster, self.logs_lines) {
            Ok(logs) => self.logs_content = logs,
            Err(e) => self.logs_content = format!("Error: {}", e),
        }
    }

    pub fn execute_sql(&mut self, cluster: &str) {
        let tab = self.sql_state.current_tab_mut();
        if tab.query.trim().is_empty() || tab.is_executing {
            return;
        }

        let query = tab.query.clone();
        let database = tab.database.clone();
        let tab_id = tab.id;
        tab.is_executing = true;

        // Add to history
        self.sql_state.add_to_history(&query, &database);
        self.sql_state.history_index = None;

        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        self.runtime.spawn(async move {
            // Use spawn_blocking since exec_sql is synchronous
            let result = tokio::task::spawn_blocking(move || {
                dbctl::postgres::exec_sql(&cluster, &database, &query)
            })
            .await;

            match result {
                Ok(Ok(output)) => {
                    let _ = tx.send(AsyncResult::SqlTabResult(tab_id, output));
                }
                Ok(Err(e)) => {
                    let _ = tx.send(AsyncResult::SqlTabResult(tab_id, format!("Error: {}", e)));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::SqlTabResult(
                        tab_id,
                        format!("Execution error: {}", e),
                    ));
                }
            }
        });
        self.status_message = "Executing query...".to_string();
    }

    pub fn execute_explain(&mut self, cluster: &str, analyze: bool) {
        let tab = self.sql_state.current_tab_mut();
        if tab.query.trim().is_empty() || tab.is_executing {
            return;
        }

        // Check if this is a DDL statement (EXPLAIN only works with SELECT, INSERT, UPDATE, DELETE, VALUES)
        let query_upper = tab.query.trim().to_uppercase();
        let first_word = query_upper.split_whitespace().next().unwrap_or("");
        let is_ddl = matches!(
            first_word,
            "CREATE" | "ALTER" | "DROP" | "TRUNCATE" | "GRANT" | "REVOKE" | "COMMENT"
        );
        // CREATE TABLE AS and CREATE MATERIALIZED VIEW AS are allowed
        let is_create_as = query_upper.contains("CREATE TABLE") && query_upper.contains(" AS ")
            || query_upper.contains("CREATE MATERIALIZED VIEW");

        if is_ddl && !is_create_as {
            let cmd = if analyze { "EXPLAIN ANALYZE" } else { "EXPLAIN" };
            tab.result = format!("{} only works with SELECT, INSERT, UPDATE, DELETE, and VALUES statements.\n\nDDL statements like CREATE, ALTER, DROP do not have query plans.", cmd);
            return;
        }

        let prefix = if analyze {
            "EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT) "
        } else {
            "EXPLAIN (FORMAT TEXT) "
        };
        let explain_query = format!("{}{}", prefix, tab.query);
        let database = tab.database.clone();
        let tab_id = tab.id;
        tab.is_executing = true;

        let tx = self.result_tx.clone();
        let cluster = cluster.to_string();
        self.runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                dbctl::postgres::exec_sql(&cluster, &database, &explain_query)
            })
            .await;

            match result {
                Ok(Ok(output)) => {
                    let _ = tx.send(AsyncResult::SqlTabResult(tab_id, output));
                }
                Ok(Err(e)) => {
                    let _ = tx.send(AsyncResult::SqlTabResult(tab_id, format!("Error: {}", e)));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::SqlTabResult(
                        tab_id,
                        format!("Execution error: {}", e),
                    ));
                }
            }
        });
        self.status_message = if analyze {
            "Running EXPLAIN ANALYZE...".to_string()
        } else {
            "Running EXPLAIN...".to_string()
        };
    }

    pub fn backup_database(&mut self, cluster: &str, database: &str) {
        match dbctl::postgres::backup(cluster, database, &self.backup_path) {
            Ok(()) => {
                self.status_message = format!("Backed up '{}' to '{}'", database, self.backup_path);
            }
            Err(e) => {
                self.status_message = format!("Backup failed: {}", e);
            }
        }
    }

    pub fn restore_database(&mut self, cluster: &str, database: &str) {
        match dbctl::postgres::restore(cluster, database, &self.restore_path) {
            Ok(()) => {
                self.status_message =
                    format!("Restored '{}' from '{}'", database, self.restore_path);
            }
            Err(e) => {
                self.status_message = format!("Restore failed: {}", e);
            }
        }
    }
}

impl eframe::App for DbctlApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Track tab at start of frame for session saving
        let tab_at_frame_start = self.current_tab.clone();

        // Apply theme visuals (only when changed or on first frame)
        if !self.theme_applied {
            let visuals = if self.theme.is_dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            };
            ctx.set_visuals(visuals);
            self.theme_applied = true;
        }

        self.process_async_results();

        // Global keyboard shortcuts
        let cmd_k = ctx.input(|i| i.key_pressed(egui::Key::K) && i.modifiers.command);
        if cmd_k && !self.command_palette.visible {
            self.command_palette.toggle();
        }

        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Install PostgreSQL...").clicked() {
                        self.show_install_dialog = true;
                        ui.close();
                    }
                    if ui.button("Create Cluster...").clicked() {
                        self.show_create_cluster_dialog = true;
                        // Auto-find port
                        if let Ok(port) = dbctl::server::find_unused_port() {
                            self.new_cluster_port = port.to_string();
                        }
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Refresh").clicked() {
                        self.refresh_clusters();
                        self.refresh_installed_versions();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("View", |ui| {
                    ui.menu_button("Theme", |ui| {
                        if ui.radio_value(&mut self.theme_mode, ThemeMode::Light, "Light").clicked() {
                            self.set_theme(ThemeMode::Light);
                            ui.close();
                        }
                        if ui.radio_value(&mut self.theme_mode, ThemeMode::Dark, "Dark").clicked() {
                            self.set_theme(ThemeMode::Dark);
                            ui.close();
                        }
                        if ui.radio_value(&mut self.theme_mode, ThemeMode::System, "System").clicked() {
                            self.set_theme(ThemeMode::System);
                            ui.close();
                        }
                    });
                });

                ui.menu_button("Settings", |ui| {
                    if ui.button("AI Assistant...").clicked() {
                        self.show_api_key_dialog = true;
                        ui.close();
                    }
                });
            });
        });

        // Left panel - cluster and connections list
        egui::SidePanel::left("clusters_panel")
            .resizable(true)
            .default_width(220.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Clusters Section
                    ui.heading("Clusters");
                    ui.separator();

                    if ui.button("+ New Cluster").clicked() {
                        self.show_create_cluster_dialog = true;
                        if let Ok(port) = dbctl::server::find_unused_port() {
                            self.new_cluster_port = port.to_string();
                        }
                    }

                    ui.separator();

                    let mut cluster_selection = None;
                    for (i, cluster) in self.clusters.iter().enumerate() {
                        let status = if cluster.running { "[R]" } else { "[S]" };
                        let label = format!(
                            "{} {} (v{}, :{})",
                            status, cluster.name, cluster.metadata.version, cluster.metadata.port
                        );

                        let selected = matches!(&self.active_connection, ActiveConnection::Cluster(idx) if *idx == i);
                        if ui.selectable_label(selected, label).clicked() {
                            cluster_selection = Some(i);
                        }
                    }

                    if let Some(i) = cluster_selection {
                        self.selected_cluster = Some(i);
                        self.active_connection = ActiveConnection::Cluster(i);
                        self.current_tab = Tab::Overview;
                        self.databases.clear();
                        self.users.clear();
                        self.logs_content.clear();
                        self.table_browser.reset();
                        // Save current layout before resetting (also persists to disk)
                        self.save_and_persist_layout();
                        self.schema_designer.reset();

                        if let Some(cluster) = self.clusters.get(i).cloned() {
                            if cluster.running {
                                self.fetch_databases(&cluster.name);
                                self.fetch_users(&cluster.name);
                            }
                            self.refresh_logs(&cluster.name);
                        }

                        // Save session state
                        self.save_session();
                    }

                    if self.clusters.is_empty() {
                        ui.label("  (no clusters)");
                    }

                    ui.add_space(10.0);

                    // Connections Section (Remote PostgreSQL)
                    ui.heading("Connections");
                    ui.separator();

                    if ui.button("+ Add Connection").clicked() {
                        self.reset_connection_form();
                        self.show_add_connection_dialog = true;
                    }

                    ui.separator();

                    let mut connection_selection = None;
                    let mut connection_to_edit = None;
                    let mut connection_to_remove = None;

                    for (i, conn) in self.connections.iter().enumerate() {
                        let status = if conn.connected { "[C]" } else { "[D]" };
                        let label = format!(
                            "{} {} ({}:{})",
                            status, conn.profile.name, conn.profile.host, conn.profile.port
                        );

                        let selected = matches!(&self.active_connection, ActiveConnection::Remote(idx) if *idx == i);

                        ui.horizontal(|ui| {
                            if ui.selectable_label(selected, &label).clicked() {
                                connection_selection = Some(i);
                            }

                            // Context menu for edit/remove
                            ui.menu_button("...", |ui| {
                                if ui.button("Edit").clicked() {
                                    connection_to_edit = Some(i);
                                    ui.close();
                                }
                                if ui.button("Remove").clicked() {
                                    connection_to_remove = Some(i);
                                    ui.close();
                                }
                            });
                        });
                    }

                    // Handle connection selection
                    if let Some(i) = connection_selection {
                        self.selected_cluster = None;
                        self.active_connection = ActiveConnection::Remote(i);
                        self.current_tab = Tab::Overview;
                        self.databases.clear();
                        self.users.clear();
                        self.logs_content.clear();
                        self.table_browser.reset();
                        // Save current layout before resetting (also persists to disk)
                        self.save_and_persist_layout();
                        self.schema_designer.reset();

                        // Fetch databases for this connection (clone to avoid borrow issues)
                        if let Some(profile) = self.connections.get(i).map(|c| c.profile.clone()) {
                            self.fetch_remote_databases(&profile);
                            self.fetch_remote_users(&profile);
                        }
                    }

                    // Handle edit (clone to avoid borrow issues)
                    if let Some(i) = connection_to_edit {
                        if let Some(conn) = self.connections.get(i).cloned() {
                            self.load_connection_to_form(&conn);
                            self.editing_connection_index = Some(i);
                            self.show_edit_connection_dialog = true;
                        }
                    }

                    // Handle remove
                    if let Some(i) = connection_to_remove {
                        self.connections.remove(i);
                        self.save_connections();
                        // Clear selection if we removed the active connection
                        if matches!(&self.active_connection, ActiveConnection::Remote(idx) if *idx == i)
                        {
                            self.active_connection = ActiveConnection::None;
                        }
                        self.status_message = "Connection removed".to_string();
                    }

                    if self.connections.is_empty() {
                        ui.label("  (no connections)");
                    }

                    ui.add_space(10.0);

                    // Installed Versions Section
                    ui.separator();
                    ui.heading("Installed");
                    for v in &self.installed_versions {
                        ui.label(format!("  PostgreSQL {}", v));
                    }
                    if self.installed_versions.is_empty() {
                        ui.label("  (none)");
                    }
                });
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
                            ui.label(egui::RichText::new("Running").color(self.theme.status_running));
                        } else {
                            ui.label(egui::RichText::new("Stopped").color(self.theme.status_stopped));
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
                            } else if ui.button("Start").clicked() {
                                self.start_cluster(&cluster.name);
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
                            ui.selectable_value(&mut self.current_tab, Tab::SchemaDesigner, "Schema");
                            ui.selectable_value(&mut self.current_tab, Tab::Users, "Users");
                            ui.selectable_value(&mut self.current_tab, Tab::Sql, "SQL");
                        }
                        ui.selectable_value(&mut self.current_tab, Tab::Logs, "Logs");
                    });

                    ui.separator();

                    // Tab content
                    match self.current_tab {
                        Tab::Overview => tabs::render_overview_tab(self, ui, &cluster),
                        Tab::Databases => tabs::render_databases_tab(self, ui, &cluster),
                        Tab::Tables => tabs::render_tables_tab(self, ui, &cluster),
                        Tab::SchemaDesigner => tabs::render_schema_designer_tab(self, ui, &cluster),
                        Tab::Users => tabs::render_users_tab(self, ui, &cluster),
                        Tab::Logs => tabs::render_logs_tab(self, ui, &cluster),
                        Tab::Sql => tabs::render_sql_tab(self, ui, &cluster),
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
        dialogs::render_install_dialog(self, ctx);
        dialogs::render_create_cluster_dialog(self, ctx);
        dialogs::render_remove_cluster_dialog(self, ctx);
        dialogs::render_drop_db_dialog(self, ctx);
        dialogs::render_add_connection_dialog(self, ctx);
        dialogs::render_edit_connection_dialog(self, ctx);
        dialogs::render_api_key_dialog(self, ctx);

        // Command palette (render last to appear on top)
        command_palette::render_command_palette(self, ctx);

        // Save session if tab changed
        if self.current_tab != tab_at_frame_start {
            self.save_session();
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Save the current layout before exiting
        self.save_and_persist_layout();
    }
}
