use eframe::egui;

use crate::state::Tab;
use crate::DbctlApp;

/// Command categories for organization and display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCategory {
    Cluster,
    Database,
    Table,
    Schema,
    Sql,
    Users,
    Connections,
    Navigation,
}

impl CommandCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Cluster => "Cluster",
            Self::Database => "Database",
            Self::Table => "Table",
            Self::Schema => "Schema",
            Self::Sql => "SQL",
            Self::Users => "Users",
            Self::Connections => "Connections",
            Self::Navigation => "Navigation",
        }
    }
}

/// Unique identifier for each command
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandId {
    // Cluster operations
    InstallPostgres,
    CreateCluster,
    StartCluster,
    StopCluster,
    RestartCluster,
    RemoveCluster,
    RefreshClusters,

    // Database operations
    CreateDatabase,
    DropDatabase,
    BackupDatabase,
    RestoreDatabase,
    RefreshDatabases,

    // Table operations
    InsertRow,
    DeleteRow,
    ExportCsv,
    ExportJson,
    RefreshTableData,

    // Schema Designer operations
    CreateTable,
    ApplyMigration,
    DuplicateTable,
    RenameTable,
    DeleteTable,
    AddColumn,
    AddIndex,
    AddForeignKey,
    Undo,
    Redo,
    OpenTemplates,
    ToggleAiChat,
    ToggleHistory,
    JumpToHistoryStart,
    JumpToHistoryEnd,
    ConfigureApiKey,

    // SQL operations
    ExecuteSql,
    ExplainSql,
    AnalyzeSql,
    ClearSql,
    SaveQuery,
    NewSqlTab,
    ShowHistory,

    // User operations
    CreateUser,
    RefreshUsers,

    // Connection operations
    AddConnection,
    EditConnection,
    RemoveConnection,

    // Tab navigation
    GoToOverview,
    GoToDatabases,
    GoToTables,
    GoToSchema,
    GoToUsers,
    GoToLogs,
    GoToSql,
}

/// Represents a single command in the palette
#[derive(Debug, Clone)]
pub struct Command {
    pub id: CommandId,
    pub label: String,
    pub category: CommandCategory,
    pub shortcut: Option<String>,
    pub keywords: Vec<String>,
}

/// State for the command palette
#[derive(Default)]
pub struct CommandPaletteState {
    pub visible: bool,
    pub search_query: String,
    pub selected_index: usize,
}

impl CommandPaletteState {
    pub fn open(&mut self) {
        self.visible = true;
        self.search_query.clear();
        self.selected_index = 0;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.search_query.clear();
        self.selected_index = 0;
    }

    pub fn toggle(&mut self) {
        if self.visible {
            self.close();
        } else {
            self.open();
        }
    }
}

/// Build the complete list of available commands
pub fn build_command_registry() -> Vec<Command> {
    vec![
        // Cluster operations
        Command {
            id: CommandId::InstallPostgres,
            label: "Install PostgreSQL".to_string(),
            category: CommandCategory::Cluster,
            shortcut: None,
            keywords: vec!["download".to_string(), "setup".to_string()],
        },
        Command {
            id: CommandId::CreateCluster,
            label: "Create Cluster".to_string(),
            category: CommandCategory::Cluster,
            shortcut: None,
            keywords: vec!["new".to_string(), "init".to_string()],
        },
        Command {
            id: CommandId::StartCluster,
            label: "Start Cluster".to_string(),
            category: CommandCategory::Cluster,
            shortcut: None,
            keywords: vec!["run".to_string(), "launch".to_string()],
        },
        Command {
            id: CommandId::StopCluster,
            label: "Stop Cluster".to_string(),
            category: CommandCategory::Cluster,
            shortcut: None,
            keywords: vec!["halt".to_string(), "shutdown".to_string()],
        },
        Command {
            id: CommandId::RestartCluster,
            label: "Restart Cluster".to_string(),
            category: CommandCategory::Cluster,
            shortcut: None,
            keywords: vec!["reboot".to_string()],
        },
        Command {
            id: CommandId::RemoveCluster,
            label: "Remove Cluster".to_string(),
            category: CommandCategory::Cluster,
            shortcut: None,
            keywords: vec!["delete".to_string(), "destroy".to_string()],
        },
        Command {
            id: CommandId::RefreshClusters,
            label: "Refresh Clusters".to_string(),
            category: CommandCategory::Cluster,
            shortcut: None,
            keywords: vec!["reload".to_string()],
        },
        // Database operations
        Command {
            id: CommandId::CreateDatabase,
            label: "Create Database".to_string(),
            category: CommandCategory::Database,
            shortcut: None,
            keywords: vec!["new".to_string(), "add".to_string()],
        },
        Command {
            id: CommandId::DropDatabase,
            label: "Drop Database".to_string(),
            category: CommandCategory::Database,
            shortcut: None,
            keywords: vec!["delete".to_string(), "remove".to_string()],
        },
        Command {
            id: CommandId::BackupDatabase,
            label: "Backup Database".to_string(),
            category: CommandCategory::Database,
            shortcut: None,
            keywords: vec!["export".to_string(), "dump".to_string()],
        },
        Command {
            id: CommandId::RestoreDatabase,
            label: "Restore Database".to_string(),
            category: CommandCategory::Database,
            shortcut: None,
            keywords: vec!["import".to_string(), "load".to_string()],
        },
        Command {
            id: CommandId::RefreshDatabases,
            label: "Refresh Databases".to_string(),
            category: CommandCategory::Database,
            shortcut: None,
            keywords: vec!["reload".to_string()],
        },
        // Table operations
        Command {
            id: CommandId::InsertRow,
            label: "Insert Row".to_string(),
            category: CommandCategory::Table,
            shortcut: None,
            keywords: vec!["add".to_string(), "new".to_string(), "create".to_string()],
        },
        Command {
            id: CommandId::DeleteRow,
            label: "Delete Row".to_string(),
            category: CommandCategory::Table,
            shortcut: None,
            keywords: vec!["remove".to_string()],
        },
        Command {
            id: CommandId::ExportCsv,
            label: "Export to CSV".to_string(),
            category: CommandCategory::Table,
            shortcut: None,
            keywords: vec!["download".to_string(), "save".to_string()],
        },
        Command {
            id: CommandId::ExportJson,
            label: "Export to JSON".to_string(),
            category: CommandCategory::Table,
            shortcut: None,
            keywords: vec!["download".to_string(), "save".to_string()],
        },
        Command {
            id: CommandId::RefreshTableData,
            label: "Refresh Table Data".to_string(),
            category: CommandCategory::Table,
            shortcut: None,
            keywords: vec!["reload".to_string()],
        },
        // Schema Designer operations
        Command {
            id: CommandId::CreateTable,
            label: "Create Table".to_string(),
            category: CommandCategory::Schema,
            shortcut: Some("Cmd+N".to_string()),
            keywords: vec!["new".to_string(), "add".to_string()],
        },
        Command {
            id: CommandId::ApplyMigration,
            label: "Apply Migration".to_string(),
            category: CommandCategory::Schema,
            shortcut: Some("Cmd+S".to_string()),
            keywords: vec!["save".to_string(), "execute".to_string(), "run".to_string()],
        },
        Command {
            id: CommandId::DuplicateTable,
            label: "Duplicate Table".to_string(),
            category: CommandCategory::Schema,
            shortcut: Some("Cmd+D".to_string()),
            keywords: vec!["copy".to_string(), "clone".to_string()],
        },
        Command {
            id: CommandId::RenameTable,
            label: "Rename Table".to_string(),
            category: CommandCategory::Schema,
            shortcut: Some("F2".to_string()),
            keywords: vec![],
        },
        Command {
            id: CommandId::DeleteTable,
            label: "Delete Table".to_string(),
            category: CommandCategory::Schema,
            shortcut: Some("Delete".to_string()),
            keywords: vec!["drop".to_string(), "remove".to_string()],
        },
        Command {
            id: CommandId::AddColumn,
            label: "Add Column".to_string(),
            category: CommandCategory::Schema,
            shortcut: None,
            keywords: vec!["new".to_string(), "create".to_string()],
        },
        Command {
            id: CommandId::AddIndex,
            label: "Add Index".to_string(),
            category: CommandCategory::Schema,
            shortcut: None,
            keywords: vec!["new".to_string(), "create".to_string()],
        },
        Command {
            id: CommandId::AddForeignKey,
            label: "Add Foreign Key".to_string(),
            category: CommandCategory::Schema,
            shortcut: None,
            keywords: vec![
                "fk".to_string(),
                "relationship".to_string(),
                "reference".to_string(),
            ],
        },
        Command {
            id: CommandId::Undo,
            label: "Undo".to_string(),
            category: CommandCategory::Schema,
            shortcut: Some("Cmd+Z".to_string()),
            keywords: vec!["revert".to_string()],
        },
        Command {
            id: CommandId::Redo,
            label: "Redo".to_string(),
            category: CommandCategory::Schema,
            shortcut: Some("Cmd+Shift+Z".to_string()),
            keywords: vec![],
        },
        Command {
            id: CommandId::OpenTemplates,
            label: "Open Schema Templates".to_string(),
            category: CommandCategory::Schema,
            shortcut: None,
            keywords: vec![
                "template".to_string(),
                "starter".to_string(),
                "pattern".to_string(),
                "ecommerce".to_string(),
                "blog".to_string(),
                "saas".to_string(),
            ],
        },
        Command {
            id: CommandId::ToggleAiChat,
            label: "Toggle AI Assistant".to_string(),
            category: CommandCategory::Schema,
            shortcut: Some("Cmd+Shift+I".to_string()),
            keywords: vec![
                "ai".to_string(),
                "chat".to_string(),
                "claude".to_string(),
                "generate".to_string(),
                "assistant".to_string(),
            ],
        },
        Command {
            id: CommandId::ToggleHistory,
            label: "Toggle History Panel".to_string(),
            category: CommandCategory::Schema,
            shortcut: Some("Cmd+Shift+H".to_string()),
            keywords: vec![
                "history".to_string(),
                "timeline".to_string(),
                "audit".to_string(),
                "log".to_string(),
                "changes".to_string(),
            ],
        },
        Command {
            id: CommandId::JumpToHistoryStart,
            label: "Jump to Initial State".to_string(),
            category: CommandCategory::Schema,
            shortcut: None,
            keywords: vec![
                "history".to_string(),
                "start".to_string(),
                "beginning".to_string(),
                "reset".to_string(),
            ],
        },
        Command {
            id: CommandId::JumpToHistoryEnd,
            label: "Jump to Latest State".to_string(),
            category: CommandCategory::Schema,
            shortcut: None,
            keywords: vec![
                "history".to_string(),
                "end".to_string(),
                "latest".to_string(),
                "current".to_string(),
            ],
        },
        Command {
            id: CommandId::ConfigureApiKey,
            label: "Configure API Key".to_string(),
            category: CommandCategory::Schema,
            shortcut: None,
            keywords: vec![
                "api".to_string(),
                "key".to_string(),
                "anthropic".to_string(),
                "settings".to_string(),
            ],
        },
        // SQL operations
        Command {
            id: CommandId::ExecuteSql,
            label: "Execute SQL".to_string(),
            category: CommandCategory::Sql,
            shortcut: Some("Cmd+Enter".to_string()),
            keywords: vec!["run".to_string(), "query".to_string()],
        },
        Command {
            id: CommandId::ExplainSql,
            label: "Explain Query".to_string(),
            category: CommandCategory::Sql,
            shortcut: None,
            keywords: vec!["plan".to_string()],
        },
        Command {
            id: CommandId::AnalyzeSql,
            label: "Explain Analyze".to_string(),
            category: CommandCategory::Sql,
            shortcut: None,
            keywords: vec!["plan".to_string(), "profile".to_string()],
        },
        Command {
            id: CommandId::ClearSql,
            label: "Clear SQL".to_string(),
            category: CommandCategory::Sql,
            shortcut: None,
            keywords: vec!["reset".to_string()],
        },
        Command {
            id: CommandId::SaveQuery,
            label: "Save Query".to_string(),
            category: CommandCategory::Sql,
            shortcut: Some("Cmd+S".to_string()),
            keywords: vec!["store".to_string()],
        },
        Command {
            id: CommandId::NewSqlTab,
            label: "New SQL Tab".to_string(),
            category: CommandCategory::Sql,
            shortcut: None,
            keywords: vec!["add".to_string(), "create".to_string()],
        },
        Command {
            id: CommandId::ShowHistory,
            label: "Show Query History".to_string(),
            category: CommandCategory::Sql,
            shortcut: None,
            keywords: vec!["previous".to_string(), "past".to_string()],
        },
        // User operations
        Command {
            id: CommandId::CreateUser,
            label: "Create User".to_string(),
            category: CommandCategory::Users,
            shortcut: None,
            keywords: vec!["new".to_string(), "add".to_string(), "role".to_string()],
        },
        Command {
            id: CommandId::RefreshUsers,
            label: "Refresh Users".to_string(),
            category: CommandCategory::Users,
            shortcut: None,
            keywords: vec!["reload".to_string()],
        },
        // Connection operations
        Command {
            id: CommandId::AddConnection,
            label: "Add Connection".to_string(),
            category: CommandCategory::Connections,
            shortcut: None,
            keywords: vec!["new".to_string(), "remote".to_string()],
        },
        Command {
            id: CommandId::EditConnection,
            label: "Edit Connection".to_string(),
            category: CommandCategory::Connections,
            shortcut: None,
            keywords: vec!["modify".to_string(), "update".to_string()],
        },
        Command {
            id: CommandId::RemoveConnection,
            label: "Remove Connection".to_string(),
            category: CommandCategory::Connections,
            shortcut: None,
            keywords: vec!["delete".to_string()],
        },
        // Tab navigation
        Command {
            id: CommandId::GoToOverview,
            label: "Go to Overview".to_string(),
            category: CommandCategory::Navigation,
            shortcut: None,
            keywords: vec!["tab".to_string(), "switch".to_string()],
        },
        Command {
            id: CommandId::GoToDatabases,
            label: "Go to Databases".to_string(),
            category: CommandCategory::Navigation,
            shortcut: None,
            keywords: vec!["tab".to_string(), "switch".to_string()],
        },
        Command {
            id: CommandId::GoToTables,
            label: "Go to Tables".to_string(),
            category: CommandCategory::Navigation,
            shortcut: None,
            keywords: vec!["tab".to_string(), "switch".to_string(), "browser".to_string()],
        },
        Command {
            id: CommandId::GoToSchema,
            label: "Go to Schema Designer".to_string(),
            category: CommandCategory::Navigation,
            shortcut: None,
            keywords: vec!["tab".to_string(), "switch".to_string(), "design".to_string()],
        },
        Command {
            id: CommandId::GoToUsers,
            label: "Go to Users".to_string(),
            category: CommandCategory::Navigation,
            shortcut: None,
            keywords: vec!["tab".to_string(), "switch".to_string(), "roles".to_string()],
        },
        Command {
            id: CommandId::GoToLogs,
            label: "Go to Logs".to_string(),
            category: CommandCategory::Navigation,
            shortcut: None,
            keywords: vec!["tab".to_string(), "switch".to_string()],
        },
        Command {
            id: CommandId::GoToSql,
            label: "Go to SQL".to_string(),
            category: CommandCategory::Navigation,
            shortcut: None,
            keywords: vec!["tab".to_string(), "switch".to_string(), "query".to_string()],
        },
    ]
}

/// Fuzzy matching score - returns Some(score) if match, None if no match
/// Higher score = better match
pub fn fuzzy_match(query: &str, text: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }

    let query = query.to_lowercase();
    let text = text.to_lowercase();

    // Exact match gets highest score
    if text == query {
        return Some(1000);
    }

    // Starts with query
    if text.starts_with(&query) {
        return Some(900 + (100 - text.len() as i32).max(0));
    }

    // Contains query as substring
    if text.contains(&query) {
        return Some(700 + (100 - text.len() as i32).max(0));
    }

    // Fuzzy character matching
    let mut query_chars = query.chars().peekable();
    let mut score = 0i32;
    let mut consecutive = 0i32;
    let mut last_match_idx: Option<usize> = None;
    let text_chars: Vec<char> = text.chars().collect();

    for (idx, c) in text_chars.iter().enumerate() {
        if let Some(&q) = query_chars.peek()
            && *c == q
        {
            query_chars.next();
            score += 10;

            // Bonus for consecutive matches
            if let Some(last) = last_match_idx {
                if idx == last + 1 {
                    consecutive += 1;
                    score += consecutive * 5;
                } else {
                    consecutive = 0;
                }
            }

            // Bonus for matching at word boundaries
            if idx == 0
                || text_chars
                    .get(idx - 1)
                    .is_some_and(|c| *c == ' ' || *c == '_')
            {
                score += 15;
            }

            last_match_idx = Some(idx);
        }
    }

    // All query characters must be found
    if query_chars.peek().is_some() {
        return None;
    }

    Some(score)
}

/// Filter and sort commands based on search query
pub fn filter_commands(commands: &[Command], query: &str) -> Vec<Command> {
    let mut scored: Vec<(i32, &Command)> = commands
        .iter()
        .filter_map(|cmd| {
            // Try matching against label
            let label_score = fuzzy_match(query, &cmd.label);

            // Try matching against keywords
            let keyword_score = cmd
                .keywords
                .iter()
                .filter_map(|kw| fuzzy_match(query, kw))
                .max()
                .map(|s| s - 100); // Keywords score slightly lower

            // Try matching against category
            let category_score = fuzzy_match(query, cmd.category.label()).map(|s| s - 200);

            // Take the best score
            let best_score = [label_score, keyword_score, category_score]
                .into_iter()
                .flatten()
                .max();

            best_score.map(|score| (score, cmd))
        })
        .collect();

    // Sort by score descending
    scored.sort_by(|a, b| b.0.cmp(&a.0));

    scored.into_iter().map(|(_, cmd)| cmd.clone()).collect()
}

/// Render the command palette overlay
pub fn render_command_palette(app: &mut DbctlApp, ctx: &egui::Context) {
    if !app.command_palette.visible {
        return;
    }

    // Handle escape key
    let escape_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    if escape_pressed {
        app.command_palette.close();
        return;
    }

    // Semi-transparent backdrop
    let screen_rect = ctx.input(|i| i.viewport_rect());
    let backdrop_color = app.theme.backdrop;
    let backdrop_response = egui::Area::new(egui::Id::new("command_palette_backdrop"))
        .fixed_pos(screen_rect.min)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let response = ui.allocate_response(screen_rect.size(), egui::Sense::click());
            ui.painter().rect_filled(
                screen_rect,
                0.0,
                backdrop_color,
            );
            response
        });

    // Close if backdrop clicked
    if backdrop_response.inner.clicked() {
        app.command_palette.close();
        return;
    }

    // Filter commands based on query
    let all_commands = build_command_registry();
    let filtered = filter_commands(&all_commands, &app.command_palette.search_query);

    // Handle keyboard navigation before rendering
    let up_pressed = ctx.input(|i| i.key_pressed(egui::Key::ArrowUp));
    let down_pressed = ctx.input(|i| i.key_pressed(egui::Key::ArrowDown));
    let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));

    if up_pressed && app.command_palette.selected_index > 0 {
        app.command_palette.selected_index -= 1;
    }

    if down_pressed && app.command_palette.selected_index < filtered.len().saturating_sub(1) {
        app.command_palette.selected_index += 1;
    }

    // Clamp selection to valid range
    if !filtered.is_empty() {
        app.command_palette.selected_index = app
            .command_palette
            .selected_index
            .min(filtered.len() - 1);
    } else {
        app.command_palette.selected_index = 0;
    }

    // Command palette window
    egui::Window::new("Command Palette")
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .anchor(egui::Align2::CENTER_TOP, [0.0, 100.0])
        .default_width(500.0)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            // Search input
            let response = ui.add(
                egui::TextEdit::singleline(&mut app.command_palette.search_query)
                    .hint_text("Type a command...")
                    .desired_width(f32::INFINITY)
                    .font(egui::FontId::proportional(16.0)),
            );

            // Auto-focus on the search input
            response.request_focus();

            ui.separator();

            // Command list
            let mut command_to_execute = None;

            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    for (idx, cmd) in filtered.iter().enumerate() {
                        let is_selected = idx == app.command_palette.selected_index;

                        let bg_color = if is_selected {
                            app.theme.selection_bg
                        } else {
                            egui::Color32::TRANSPARENT
                        };

                        let text_color = if is_selected {
                            app.theme.text_on_accent
                        } else {
                            ui.visuals().text_color()
                        };

                        let frame_response = egui::Frame::new()
                            .fill(bg_color)
                            .inner_margin(egui::Margin::symmetric(8, 4))
                            .corner_radius(4.0)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());

                                ui.horizontal(|ui| {
                                    // Category label (fixed width)
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(80.0, 20.0),
                                        egui::Layout::left_to_right(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(cmd.category.label())
                                                    .small()
                                                    .color(egui::Color32::GRAY),
                                            );
                                        },
                                    );

                                    // Command label
                                    ui.label(egui::RichText::new(&cmd.label).color(text_color));

                                    // Keyboard shortcut (right-aligned)
                                    if let Some(shortcut) = &cmd.shortcut {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    egui::RichText::new(shortcut)
                                                        .small()
                                                        .color(egui::Color32::GRAY)
                                                        .monospace(),
                                                );
                                            },
                                        );
                                    }
                                });
                            });

                        // Handle click
                        let click_response =
                            ui.interact(frame_response.response.rect, ui.id().with(idx), egui::Sense::click());

                        if click_response.clicked() {
                            command_to_execute = Some(cmd.id);
                        }

                        // Handle hover - update selection
                        if click_response.hovered() {
                            app.command_palette.selected_index = idx;
                        }

                        // Scroll selected item into view
                        if is_selected {
                            click_response.scroll_to_me(Some(egui::Align::Center));
                        }
                    }
                });

            // Handle enter key
            if enter_pressed && !filtered.is_empty() {
                command_to_execute = Some(filtered[app.command_palette.selected_index].id);
            }

            // Execute command if selected
            if let Some(cmd_id) = command_to_execute {
                execute_command(app, cmd_id);
                app.command_palette.close();
            }

            // Show help text if no matches
            if filtered.is_empty() && !app.command_palette.search_query.is_empty() {
                ui.label(
                    egui::RichText::new("No matching commands")
                        .color(egui::Color32::GRAY)
                        .italics(),
                );
            }
        });
}

/// Execute a command by its ID
pub fn execute_command(app: &mut DbctlApp, cmd_id: CommandId) {
    match cmd_id {
        // Cluster operations
        CommandId::InstallPostgres => {
            app.show_install_dialog = true;
        }
        CommandId::CreateCluster => {
            app.show_create_cluster_dialog = true;
            if let Ok(port) = dbctl::server::find_unused_port() {
                app.new_cluster_port = port.to_string();
            }
        }
        CommandId::StartCluster => {
            if let Some(cluster) = app.selected_cluster().map(|c| c.name.clone()) {
                app.start_cluster(&cluster);
            } else {
                app.status_message = "No cluster selected".to_string();
            }
        }
        CommandId::StopCluster => {
            if let Some(cluster) = app.selected_cluster().map(|c| c.name.clone()) {
                app.stop_cluster(&cluster);
            } else {
                app.status_message = "No cluster selected".to_string();
            }
        }
        CommandId::RestartCluster => {
            if let Some(cluster) = app.selected_cluster().map(|c| c.name.clone()) {
                app.restart_cluster(&cluster);
            } else {
                app.status_message = "No cluster selected".to_string();
            }
        }
        CommandId::RemoveCluster => {
            if app.selected_cluster().is_some() {
                app.show_remove_cluster_dialog = true;
            } else {
                app.status_message = "No cluster selected".to_string();
            }
        }
        CommandId::RefreshClusters => {
            app.refresh_clusters();
            app.refresh_installed_versions();
        }

        // Database operations
        CommandId::CreateDatabase => {
            if let Some(cluster) = app.selected_cluster() {
                if cluster.running {
                    app.current_tab = Tab::Databases;
                } else {
                    app.status_message = "Cluster is not running".to_string();
                }
            } else {
                app.status_message = "No cluster selected".to_string();
            }
        }
        CommandId::DropDatabase => {
            if let Some(db) = app
                .selected_database
                .and_then(|i| app.databases.get(i).cloned())
            {
                if db != "postgres" && db != "template0" && db != "template1" {
                    app.pending_drop_db = Some(db);
                    app.show_drop_db_dialog = true;
                } else {
                    app.status_message = "Cannot drop system database".to_string();
                }
            } else {
                app.status_message = "No database selected".to_string();
            }
        }
        CommandId::BackupDatabase => {
            app.current_tab = Tab::Databases;
        }
        CommandId::RestoreDatabase => {
            app.current_tab = Tab::Databases;
        }
        CommandId::RefreshDatabases => {
            if let Some(cluster) = app.selected_cluster().map(|c| c.name.clone()) {
                app.fetch_databases(&cluster);
            }
        }

        // Table operations
        CommandId::InsertRow => {
            if app.table_browser.selected_table.is_some() {
                app.table_browser.show_insert_dialog = true;
                app.table_browser.new_row_values.clear();
            } else {
                app.status_message = "No table selected".to_string();
            }
        }
        CommandId::DeleteRow => {
            app.status_message = "Select a row to delete".to_string();
        }
        CommandId::ExportCsv => {
            app.current_tab = Tab::Tables;
        }
        CommandId::ExportJson => {
            app.current_tab = Tab::Tables;
        }
        CommandId::RefreshTableData => {
            app.current_tab = Tab::Tables;
        }

        // Schema Designer operations
        CommandId::CreateTable => {
            app.current_tab = Tab::SchemaDesigner;
            app.schema_designer.reset_new_table_form();
            app.schema_designer.show_new_table_dialog = true;
        }
        CommandId::ApplyMigration => {
            if app.schema_designer.has_changes() {
                app.schema_designer.show_apply_migration_dialog = true;
            } else {
                app.status_message = "No pending changes".to_string();
            }
        }
        CommandId::DuplicateTable => {
            app.current_tab = Tab::SchemaDesigner;
        }
        CommandId::RenameTable => {
            app.current_tab = Tab::SchemaDesigner;
        }
        CommandId::DeleteTable => {
            app.current_tab = Tab::SchemaDesigner;
        }
        CommandId::AddColumn => {
            if app.schema_designer.selected_table.is_some() {
                app.schema_designer.reset_new_column_form();
                app.schema_designer.show_new_column_dialog = true;
            } else {
                app.status_message = "No table selected in schema designer".to_string();
            }
        }
        CommandId::AddIndex => {
            if app.schema_designer.selected_table.is_some() {
                app.schema_designer.reset_new_index_form();
                app.schema_designer.show_new_index_dialog = true;
            } else {
                app.status_message = "No table selected in schema designer".to_string();
            }
        }
        CommandId::AddForeignKey => {
            if app.schema_designer.selected_table.is_some() {
                app.schema_designer.reset_new_fk_form();
                app.schema_designer.show_new_fk_dialog = true;
            } else {
                app.status_message = "No table selected in schema designer".to_string();
            }
        }
        CommandId::Undo => {
            app.schema_designer.undo();
        }
        CommandId::Redo => {
            app.schema_designer.redo();
        }
        CommandId::OpenTemplates => {
            app.current_tab = Tab::SchemaDesigner;
            app.schema_designer.show_template_dialog = true;
            app.schema_designer.template_search_query.clear();
            app.schema_designer.selected_template = None;
            app.schema_designer.template_preview = None;
        }
        CommandId::ToggleAiChat => {
            app.current_tab = Tab::SchemaDesigner;
            app.ai_chat.toggle();
        }
        CommandId::ToggleHistory => {
            app.current_tab = Tab::SchemaDesigner;
            app.schema_designer.history_panel_visible = !app.schema_designer.history_panel_visible;
        }
        CommandId::JumpToHistoryStart => {
            app.current_tab = Tab::SchemaDesigner;
            app.schema_designer.jump_to_action(0);
        }
        CommandId::JumpToHistoryEnd => {
            app.current_tab = Tab::SchemaDesigner;
            let len = app.schema_designer.action_history.len();
            app.schema_designer.jump_to_action(len);
        }
        CommandId::ConfigureApiKey => {
            app.show_api_key_dialog = true;
        }

        // SQL operations
        CommandId::ExecuteSql => {
            if let Some(cluster) = app.selected_cluster().map(|c| c.name.clone()) {
                app.execute_sql(&cluster);
            }
        }
        CommandId::ExplainSql => {
            if let Some(cluster) = app.selected_cluster().map(|c| c.name.clone()) {
                app.execute_explain(&cluster, false);
            }
        }
        CommandId::AnalyzeSql => {
            if let Some(cluster) = app.selected_cluster().map(|c| c.name.clone()) {
                app.execute_explain(&cluster, true);
            }
        }
        CommandId::ClearSql => {
            let tab = app.sql_state.current_tab_mut();
            tab.query.clear();
            tab.result.clear();
        }
        CommandId::SaveQuery => {
            app.sql_state.show_save_dialog = true;
        }
        CommandId::NewSqlTab => {
            let db = app.sql_state.current_tab().database.clone();
            app.sql_state.add_tab(db);
        }
        CommandId::ShowHistory => {
            app.sql_state.show_history_popup = !app.sql_state.show_history_popup;
        }

        // User operations
        CommandId::CreateUser => {
            app.current_tab = Tab::Users;
        }
        CommandId::RefreshUsers => {
            if let Some(cluster) = app.selected_cluster().map(|c| c.name.clone()) {
                app.fetch_users(&cluster);
            }
        }

        // Connection operations
        CommandId::AddConnection => {
            app.reset_connection_form();
            app.show_add_connection_dialog = true;
        }
        CommandId::EditConnection => {
            app.status_message = "Select a connection to edit".to_string();
        }
        CommandId::RemoveConnection => {
            app.status_message = "Select a connection to remove".to_string();
        }

        // Tab navigation
        CommandId::GoToOverview => {
            app.current_tab = Tab::Overview;
        }
        CommandId::GoToDatabases => {
            app.current_tab = Tab::Databases;
        }
        CommandId::GoToTables => {
            app.current_tab = Tab::Tables;
        }
        CommandId::GoToSchema => {
            app.current_tab = Tab::SchemaDesigner;
        }
        CommandId::GoToUsers => {
            app.current_tab = Tab::Users;
        }
        CommandId::GoToLogs => {
            app.current_tab = Tab::Logs;
        }
        CommandId::GoToSql => {
            app.current_tab = Tab::Sql;
        }
    }
}
