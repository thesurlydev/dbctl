use eframe::egui;

use crate::ai_chat::{ApiKeyManager, MessageRole, SchemaChangeProposal};
use crate::schema_designer::{
    canvas::{auto_layout_tables, layout_new_tables, render_canvas},
    get_all_templates, ColumnModel, ConstraintModel, FkAction, IndexModel, SchemaAction,
    SchemaModel, TableModel, TemplateCategory,
};
use crate::state::ClusterInfo;
use crate::DbctlApp;

/// Keyboard shortcuts state
struct Shortcuts {
    new_table: bool,
    save: bool,
    duplicate: bool,
    rename: bool,
    delete: bool,
    undo: bool,
    redo: bool,
    escape: bool,
    toggle_history: bool,
}

fn capture_shortcuts(ctx: &egui::Context) -> Shortcuts {
    ctx.input(|i| Shortcuts {
        new_table: i.key_pressed(egui::Key::N) && i.modifiers.command,
        save: i.key_pressed(egui::Key::S) && i.modifiers.command,
        duplicate: i.key_pressed(egui::Key::D) && i.modifiers.command,
        rename: i.key_pressed(egui::Key::F2),
        delete: i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace),
        undo: i.key_pressed(egui::Key::Z) && i.modifiers.command && !i.modifiers.shift,
        redo: i.key_pressed(egui::Key::Z) && i.modifiers.command && i.modifiers.shift,
        escape: i.key_pressed(egui::Key::Escape),
        toggle_history: i.key_pressed(egui::Key::H) && i.modifiers.command && i.modifiers.shift,
    })
}

pub fn render_schema_designer_tab(app: &mut DbctlApp, ui: &mut egui::Ui, cluster: &ClusterInfo) {
    let ctx = ui.ctx().clone();

    // Capture keyboard shortcuts
    let shortcuts = capture_shortcuts(&ctx);

    // Process shortcuts
    if shortcuts.new_table && !any_dialog_open(app) {
        app.schema_designer.reset_new_table_form();
        app.schema_designer.show_new_table_dialog = true;
    }
    if shortcuts.save && app.schema_designer.has_changes() {
        app.schema_designer.show_apply_migration_dialog = true;
    }
    if shortcuts.duplicate {
        duplicate_selected(app);
    }
    if shortcuts.rename && !any_dialog_open(app) {
        start_rename(app);
    }
    if shortcuts.delete && !any_dialog_open(app) {
        delete_selected(app);
    }
    if shortcuts.undo {
        app.schema_designer.undo();
    }
    if shortcuts.redo {
        app.schema_designer.redo();
    }
    if shortcuts.escape {
        cancel_current_operation(app);
    }
    if shortcuts.toggle_history {
        app.schema_designer.history_panel_visible = !app.schema_designer.history_panel_visible;
    }

    // Top toolbar
    render_toolbar(app, ui, cluster);

    ui.separator();

    // Main content - split between canvas, AI sidebar, and SQL preview
    let available_height = ui.available_height();
    let sql_preview_height = if app.schema_designer.sql_preview_expanded { 150.0 } else { 30.0 };
    let canvas_height = available_height - sql_preview_height - 10.0;

    // Horizontal layout for canvas and sidebars
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), canvas_height),
        egui::Layout::left_to_right(egui::Align::TOP),
        |ui| {
            // Calculate widths for sidebars
            let history_width = if app.schema_designer.history_panel_visible { 280.0 } else { 0.0 };
            let ai_width = if app.ai_chat.visible { app.ai_chat.width } else { 0.0 };
            let spacing = if app.schema_designer.history_panel_visible || app.ai_chat.visible { 8.0 } else { 0.0 };
            let canvas_width = ui.available_width() - history_width - ai_width - spacing;

            // Canvas area
            ui.allocate_ui_with_layout(
                egui::vec2(canvas_width, ui.available_height()),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
            if app.schema_designer.loading_schema {
                ui.centered_and_justified(|ui| {
                    ui.spinner();
                    ui.label("Loading schema...");
                });
            } else if app.schema_designer.selected_database.is_none() {
                ui.centered_and_justified(|ui| {
                    ui.label("Select a database to start designing");
                });
            } else if app.schema_designer.working_schema.tables.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.label("No tables in schema");
                        ui.add_space(10.0);
                        if ui.button("Create Table (Cmd+N)").clicked() {
                            app.schema_designer.reset_new_table_form();
                            app.schema_designer.show_new_table_dialog = true;
                        }
                    });
                });
            } else {
                // Render canvas
                let canvas_response = render_canvas(
                    ui,
                    &app.schema_designer.working_schema,
                    &mut app.schema_designer.canvas,
                    &app.schema_designer.selected_table,
                    &app.schema_designer.selected_column,
                    &app.schema_designer.visual_column_order,
                    &app.theme,
                );

                // Handle canvas responses
                if let Some(table_name) = canvas_response.table_clicked {
                    app.schema_designer.selected_table = Some(table_name);
                    app.schema_designer.selected_column = None;
                }
                if let Some((table, column)) = canvas_response.column_clicked {
                    app.schema_designer.selected_table = Some(table.clone());
                    app.schema_designer.selected_column = Some((table, column));
                }
                if canvas_response.deselect {
                    app.schema_designer.selected_table = None;
                    app.schema_designer.selected_column = None;
                }
                if let Some(table_name) = canvas_response.table_double_clicked {
                    // Toggle collapse
                    if let Some(table) = app.schema_designer.working_schema.tables.get_mut(&table_name) {
                        table.collapsed = !table.collapsed;
                    }
                }
                if let Some((table, column)) = canvas_response.column_double_clicked {
                    // Open edit column dialog
                    if let Some(t) = app.schema_designer.working_schema.tables.get(&table) {
                        if let Some(col) = t.columns.iter().find(|c| c.name == column) {
                            app.schema_designer.selected_table = Some(table.clone());
                            app.schema_designer.selected_column = Some((table, column));
                            app.schema_designer.edit_column_name = col.name.clone();
                            // Parse type and size from the data_type
                            let (base_type, size) = parse_type_and_size(&col.data_type);
                            app.schema_designer.edit_column_type = base_type;
                            app.schema_designer.edit_column_size = size;
                            app.schema_designer.edit_column_nullable = col.is_nullable;
                            app.schema_designer.edit_column_default = col.default_value.clone().unwrap_or_default();
                            app.schema_designer.show_edit_column_dialog = true;
                        }
                    }
                }
                if let Some((table_name, _old_pos, new_pos)) = canvas_response.table_moved {
                    // Update position during drag (without recording action)
                    if let Some(table) = app.schema_designer.working_schema.tables.get_mut(&table_name) {
                        table.position = new_pos;
                    }
                }
                if let Some((table_name, old_pos, new_pos)) = canvas_response.table_move_completed {
                    // Record the move action for undo
                    app.schema_designer.push_action(SchemaAction::MoveTable {
                        table: table_name,
                        old_pos,
                        new_pos,
                    });
                }
                // Handle column reordering (visual only, doesn't affect schema)
                if let Some((table_name, from_idx, to_idx)) = canvas_response.column_reordered {
                    app.schema_designer.reorder_column_visually(&table_name, from_idx, to_idx);
                }
            }
                },
            );

            // History panel (right sidebar)
            if app.schema_designer.history_panel_visible {
                ui.separator();
                render_history_panel(app, ui);
            }

            // AI Chat sidebar
            if app.ai_chat.visible {
                ui.separator();
                render_ai_chat_sidebar(app, ui);
            }
        },
    );

    // SQL Preview panel
    render_sql_preview(app, ui);

    // Dialogs
    render_new_table_dialog(app, &ctx);
    render_new_column_dialog(app, &ctx);
    render_new_index_dialog(app, &ctx);
    render_new_fk_dialog(app, &ctx);
    render_edit_column_dialog(app, &ctx);
    render_apply_migration_dialog(app, &ctx, cluster);
    render_template_dialog(app, &ctx);
    render_template_confirm_dialog(app, &ctx);
    render_delete_table_dialog(app, &ctx);
    render_api_key_dialog(app, &ctx);
}

fn render_toolbar(app: &mut DbctlApp, ui: &mut egui::Ui, cluster: &ClusterInfo) {
    // Row 1: Database/schema selection and table actions
    ui.horizontal(|ui| {
        // Database selector
        ui.label("Database:");
        let current_db = app.schema_designer.selected_database.clone().unwrap_or_default();
        let databases = app.databases.clone();
        let cluster_name = cluster.name.clone();
        let mut selected_db: Option<String> = None;

        egui::ComboBox::from_id_salt("schema_designer_db")
            .selected_text(&current_db)
            .width(100.0)
            .show_ui(ui, |ui| {
                for db in &databases {
                    if ui.selectable_label(Some(db) == app.schema_designer.selected_database.as_ref(), db).clicked() {
                        selected_db = Some(db.clone());
                    }
                }
            });

        if let Some(db) = selected_db {
            app.schema_designer.selected_database = Some(db.clone());
            app.schema_designer.selected_schema = "public".to_string();
            app.load_schema_for_designer(&cluster_name, &db, "public");
        }

        ui.label("Schema:");
        ui.add(egui::TextEdit::singleline(&mut app.schema_designer.selected_schema).desired_width(60.0));

        ui.separator();

        if ui.button("+ Table").on_hover_text("New Table (Cmd+N)").clicked() {
            app.schema_designer.reset_new_table_form();
            app.schema_designer.show_new_table_dialog = true;
        }

        if ui.button("Templates").clicked() {
            app.schema_designer.show_template_dialog = true;
            app.schema_designer.template_search_query.clear();
            app.schema_designer.selected_template = None;
            app.schema_designer.template_preview = None;
        }

        // Only show these if a table is selected
        if app.schema_designer.selected_table.is_some() {
            if ui.button("+ Col").on_hover_text("Add Column").clicked() {
                app.schema_designer.reset_new_column_form();
                app.schema_designer.show_new_column_dialog = true;
            }
            if ui.button("+ Idx").on_hover_text("Add Index").clicked() {
                app.schema_designer.reset_new_index_form();
                app.schema_designer.show_new_index_dialog = true;
            }
            if ui.button("+ FK").on_hover_text("Add Foreign Key").clicked() {
                app.schema_designer.reset_new_fk_form();
                app.schema_designer.show_new_fk_dialog = true;
            }
        }

        // Right side actions
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let has_changes = app.schema_designer.has_changes();
            if ui.add_enabled(has_changes, egui::Button::new("Apply"))
                .on_hover_text("Apply Changes (Cmd+S)")
                .clicked()
            {
                app.schema_designer.show_apply_migration_dialog = true;
            }
            if has_changes {
                ui.label(format!("({})", app.schema_designer.pending_sql.len()));
            }
        });
    });

    // Row 2: View controls and tools
    ui.horizontal(|ui| {
        let can_undo = app.schema_designer.action_index > 0;
        let can_redo = app.schema_designer.action_index < app.schema_designer.action_history.len();

        if ui.add_enabled(can_undo, egui::Button::new("Undo")).on_hover_text("Cmd+Z").clicked() {
            app.schema_designer.undo();
        }
        if ui.add_enabled(can_redo, egui::Button::new("Redo")).on_hover_text("Cmd+Shift+Z").clicked() {
            app.schema_designer.redo();
        }

        ui.separator();

        if ui.button("Layout").on_hover_text("Auto Layout Tables").clicked() {
            auto_layout_tables(&mut app.schema_designer.working_schema);
        }

        ui.separator();

        ui.checkbox(&mut app.schema_designer.canvas.show_grid, "Grid");
        ui.checkbox(&mut app.schema_designer.canvas.snap_to_grid, "Snap");

        // Right side - AI toggle and History
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let ai_button_text = if app.ai_chat.visible { "Hide AI" } else { "AI Assistant" };
            if ui.button(ai_button_text).on_hover_text("Cmd+Shift+I").clicked() {
                app.ai_chat.toggle();
            }

            let history_button_text = if app.schema_designer.history_panel_visible { "Hide History" } else { "History" };
            if ui.button(history_button_text).on_hover_text("Cmd+Shift+H").clicked() {
                app.schema_designer.history_panel_visible = !app.schema_designer.history_panel_visible;
            }
        });
    });
}

fn render_sql_preview(app: &mut DbctlApp, ui: &mut egui::Ui) {
    ui.separator();

    ui.horizontal(|ui| {
        let arrow = if app.schema_designer.sql_preview_expanded { "v" } else { ">" };
        if ui.button(format!("{} SQL Preview", arrow)).clicked() {
            app.schema_designer.sql_preview_expanded = !app.schema_designer.sql_preview_expanded;
        }

        if !app.schema_designer.pending_sql.is_empty() {
            ui.label(format!("({} statements)", app.schema_designer.pending_sql.len()));
        }
    });

    if app.schema_designer.sql_preview_expanded {
        let sql_text = app.schema_designer.pending_sql.join("\n\n");
        egui::ScrollArea::vertical()
            .max_height(120.0)
            .show(ui, |ui| {
                if sql_text.is_empty() {
                    ui.label(egui::RichText::new("No pending changes").italics().color(egui::Color32::GRAY));
                } else {
                    ui.add(
                        egui::TextEdit::multiline(&mut sql_text.as_str())
                            .font(egui::FontId::monospace(12.0))
                            .desired_width(f32::INFINITY)
                            .interactive(false),
                    );
                }
            });
    }
}

fn render_new_table_dialog(app: &mut DbctlApp, ctx: &egui::Context) {
    if !app.schema_designer.show_new_table_dialog {
        return;
    }

    egui::Window::new("New Table")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Table Name:");
                ui.text_edit_singleline(&mut app.schema_designer.new_table_name);
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    app.schema_designer.show_new_table_dialog = false;
                }

                let name_valid = !app.schema_designer.new_table_name.trim().is_empty();
                let name_unique = !app.schema_designer.working_schema.tables.contains_key(&app.schema_designer.new_table_name);

                if ui.add_enabled(name_valid && name_unique, egui::Button::new("Create")).clicked() {
                    let name = app.schema_designer.new_table_name.trim().to_string();

                    // Find a non-overlapping position for the new table
                    let new_table_width = 150.0; // MIN_TABLE_WIDTH
                    let new_table_height = 44.0; // Header + padding for empty table
                    let pos = crate::schema_designer::canvas::find_non_overlapping_position(
                        &app.schema_designer.working_schema,
                        new_table_width,
                        new_table_height,
                    );

                    let table = TableModel {
                        name: name.clone(),
                        position: pos,
                        ..Default::default()
                    };

                    app.schema_designer.push_action(SchemaAction::CreateTable(table.clone()));
                    app.schema_designer.working_schema.tables.insert(name.clone(), table);
                    app.schema_designer.selected_table = Some(name);
                    app.schema_designer.show_new_table_dialog = false;
                    app.schema_designer.refresh_pending_sql();
                }

                if !name_unique && name_valid {
                    ui.label(egui::RichText::new("Table already exists").color(egui::Color32::RED));
                }
            });
        });
}

/// Types that support a size/length parameter
const SIZED_TYPES: &[&str] = &["varchar", "char", "numeric", "decimal", "bit", "varbit"];

/// Check if a type supports size parameter
fn type_supports_size(type_name: &str) -> bool {
    let base = type_name.split('(').next().unwrap_or(type_name);
    SIZED_TYPES.contains(&base)
}

/// Combine base type with size (e.g., "varchar" + "100" -> "varchar(100)")
fn combine_type_with_size(base_type: &str, size: &str) -> String {
    if size.is_empty() || !type_supports_size(base_type) {
        base_type.to_string()
    } else {
        format!("{}({})", base_type, size)
    }
}

/// Parse a type into base type and size (e.g., "varchar(100)" -> ("varchar", "100"))
fn parse_type_and_size(full_type: &str) -> (String, String) {
    if let Some(paren_pos) = full_type.find('(') {
        let base = full_type[..paren_pos].to_string();
        let size = full_type[paren_pos + 1..]
            .trim_end_matches(')')
            .to_string();
        (base, size)
    } else {
        (full_type.to_string(), String::new())
    }
}

fn render_new_column_dialog(app: &mut DbctlApp, ctx: &egui::Context) {
    if !app.schema_designer.show_new_column_dialog {
        return;
    }

    let table_name = match &app.schema_designer.selected_table {
        Some(t) => t.clone(),
        None => {
            app.schema_designer.show_new_column_dialog = false;
            return;
        }
    };

    egui::Window::new(format!("Add Column to '{}'", table_name))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Grid::new("new_col_grid").num_columns(2).show(ui, |ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut app.schema_designer.new_column_name);
                ui.end_row();

                ui.label("Type:");
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("col_type")
                        .selected_text(&app.schema_designer.new_column_type)
                        .show_ui(ui, |ui| {
                            for t in &["integer", "bigint", "smallint", "serial", "bigserial", "text", "varchar", "char", "boolean", "timestamp", "timestamptz", "date", "time", "uuid", "jsonb", "json", "numeric", "decimal", "real", "double precision", "bytea"] {
                                ui.selectable_value(&mut app.schema_designer.new_column_type, t.to_string(), *t);
                            }
                        });

                    // Show size input for types that support it
                    if type_supports_size(&app.schema_designer.new_column_type) {
                        ui.label("(");
                        ui.add(egui::TextEdit::singleline(&mut app.schema_designer.new_column_size)
                            .desired_width(50.0)
                            .hint_text("size"));
                        ui.label(")");
                    }
                });
                ui.end_row();

                ui.label("Nullable:");
                ui.checkbox(&mut app.schema_designer.new_column_nullable, "");
                ui.end_row();

                ui.label("Default:");
                ui.text_edit_singleline(&mut app.schema_designer.new_column_default);
                ui.end_row();

                ui.label("Primary Key:");
                ui.checkbox(&mut app.schema_designer.new_column_pk, "");
                ui.end_row();
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    app.schema_designer.show_new_column_dialog = false;
                }

                let name_valid = !app.schema_designer.new_column_name.trim().is_empty();
                let name_unique = app.schema_designer.working_schema.tables
                    .get(&table_name)
                    .map(|t| !t.columns.iter().any(|c| c.name == app.schema_designer.new_column_name))
                    .unwrap_or(true);

                if ui.add_enabled(name_valid && name_unique, egui::Button::new("Add")).clicked() {
                    let full_type = combine_type_with_size(
                        &app.schema_designer.new_column_type,
                        &app.schema_designer.new_column_size,
                    );

                    let column = ColumnModel {
                        name: app.schema_designer.new_column_name.trim().to_string(),
                        data_type: full_type,
                        is_nullable: app.schema_designer.new_column_nullable,
                        default_value: if app.schema_designer.new_column_default.is_empty() {
                            None
                        } else {
                            Some(app.schema_designer.new_column_default.clone())
                        },
                        is_primary_key: app.schema_designer.new_column_pk,
                    };

                    app.schema_designer.push_action(SchemaAction::AddColumn {
                        table: table_name.clone(),
                        column: column.clone(),
                    });

                    if let Some(table) = app.schema_designer.working_schema.tables.get_mut(&table_name) {
                        table.columns.push(column);
                    }

                    app.schema_designer.show_new_column_dialog = false;
                    app.schema_designer.refresh_pending_sql();
                }

                if !name_unique && name_valid {
                    ui.label(egui::RichText::new("Column already exists").color(egui::Color32::RED));
                }
            });
        });
}

fn render_new_index_dialog(app: &mut DbctlApp, ctx: &egui::Context) {
    if !app.schema_designer.show_new_index_dialog {
        return;
    }

    let table_name = match &app.schema_designer.selected_table {
        Some(t) => t.clone(),
        None => {
            app.schema_designer.show_new_index_dialog = false;
            return;
        }
    };

    let columns: Vec<String> = app.schema_designer.working_schema.tables
        .get(&table_name)
        .map(|t| t.columns.iter().map(|c| c.name.clone()).collect())
        .unwrap_or_default();

    egui::Window::new(format!("Add Index to '{}'", table_name))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Index Name:");
                ui.text_edit_singleline(&mut app.schema_designer.new_index_name);
            });

            ui.checkbox(&mut app.schema_designer.new_index_unique, "Unique");

            ui.label("Columns:");
            for col in &columns {
                let mut selected = app.schema_designer.new_index_columns.contains(col);
                if ui.checkbox(&mut selected, col).changed() {
                    if selected {
                        app.schema_designer.new_index_columns.insert(col.clone());
                    } else {
                        app.schema_designer.new_index_columns.remove(col);
                    }
                }
            }

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    app.schema_designer.show_new_index_dialog = false;
                }

                let name_valid = !app.schema_designer.new_index_name.trim().is_empty();
                let has_columns = !app.schema_designer.new_index_columns.is_empty();

                if ui.add_enabled(name_valid && has_columns, egui::Button::new("Create")).clicked() {
                    let index = IndexModel {
                        name: app.schema_designer.new_index_name.trim().to_string(),
                        columns: app.schema_designer.new_index_columns.iter().cloned().collect(),
                        is_unique: app.schema_designer.new_index_unique,
                        ..Default::default()
                    };

                    app.schema_designer.push_action(SchemaAction::CreateIndex {
                        table: table_name.clone(),
                        index: index.clone(),
                    });

                    if let Some(table) = app.schema_designer.working_schema.tables.get_mut(&table_name) {
                        table.indexes.push(index);
                    }

                    app.schema_designer.show_new_index_dialog = false;
                    app.schema_designer.refresh_pending_sql();
                }
            });
        });
}

fn render_new_fk_dialog(app: &mut DbctlApp, ctx: &egui::Context) {
    if !app.schema_designer.show_new_fk_dialog {
        return;
    }

    let table_name = match &app.schema_designer.selected_table {
        Some(t) => t.clone(),
        None => {
            app.schema_designer.show_new_fk_dialog = false;
            return;
        }
    };

    let columns: Vec<String> = app.schema_designer.working_schema.tables
        .get(&table_name)
        .map(|t| t.columns.iter().map(|c| c.name.clone()).collect())
        .unwrap_or_default();

    let other_tables: Vec<String> = app.schema_designer.working_schema.tables.keys()
        .filter(|t| *t != &table_name)
        .cloned()
        .collect();

    egui::Window::new(format!("Add Foreign Key to '{}'", table_name))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Grid::new("fk_grid").num_columns(2).show(ui, |ui| {
                ui.label("Column:");
                egui::ComboBox::from_id_salt("fk_col")
                    .selected_text(&app.schema_designer.new_fk_column)
                    .show_ui(ui, |ui| {
                        for col in &columns {
                            ui.selectable_value(&mut app.schema_designer.new_fk_column, col.clone(), col);
                        }
                    });
                ui.end_row();

                ui.label("References Table:");
                egui::ComboBox::from_id_salt("fk_ref_table")
                    .selected_text(&app.schema_designer.new_fk_ref_table)
                    .show_ui(ui, |ui| {
                        for tbl in &other_tables {
                            ui.selectable_value(&mut app.schema_designer.new_fk_ref_table, tbl.clone(), tbl);
                        }
                    });
                ui.end_row();

                ui.label("References Column:");
                let ref_cols: Vec<String> = app.schema_designer.working_schema.tables
                    .get(&app.schema_designer.new_fk_ref_table)
                    .map(|t| t.columns.iter().map(|c| c.name.clone()).collect())
                    .unwrap_or_default();
                egui::ComboBox::from_id_salt("fk_ref_col")
                    .selected_text(&app.schema_designer.new_fk_ref_column)
                    .show_ui(ui, |ui| {
                        for col in &ref_cols {
                            ui.selectable_value(&mut app.schema_designer.new_fk_ref_column, col.clone(), col);
                        }
                    });
                ui.end_row();

                ui.label("On Delete:");
                egui::ComboBox::from_id_salt("fk_on_delete")
                    .selected_text(format!("{}", app.schema_designer.new_fk_on_delete))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut app.schema_designer.new_fk_on_delete, FkAction::NoAction, "NO ACTION");
                        ui.selectable_value(&mut app.schema_designer.new_fk_on_delete, FkAction::Restrict, "RESTRICT");
                        ui.selectable_value(&mut app.schema_designer.new_fk_on_delete, FkAction::Cascade, "CASCADE");
                        ui.selectable_value(&mut app.schema_designer.new_fk_on_delete, FkAction::SetNull, "SET NULL");
                    });
                ui.end_row();
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    app.schema_designer.show_new_fk_dialog = false;
                }

                let valid = !app.schema_designer.new_fk_column.is_empty()
                    && !app.schema_designer.new_fk_ref_table.is_empty()
                    && !app.schema_designer.new_fk_ref_column.is_empty();

                if ui.add_enabled(valid, egui::Button::new("Create")).clicked() {
                    let fk_name = format!(
                        "fk_{}_{}_{}",
                        table_name,
                        app.schema_designer.new_fk_column,
                        app.schema_designer.new_fk_ref_table
                    );

                    let constraint = ConstraintModel::foreign_key(
                        &fk_name,
                        &app.schema_designer.new_fk_column,
                        &app.schema_designer.new_fk_ref_table,
                        &app.schema_designer.new_fk_ref_column,
                        app.schema_designer.new_fk_on_delete,
                    );

                    app.schema_designer.push_action(SchemaAction::AddConstraint {
                        table: table_name.clone(),
                        constraint: constraint.clone(),
                    });

                    if let Some(table) = app.schema_designer.working_schema.tables.get_mut(&table_name) {
                        table.constraints.push(constraint);
                    }

                    app.schema_designer.show_new_fk_dialog = false;
                    app.schema_designer.refresh_pending_sql();
                }
            });
        });
}

fn render_edit_column_dialog(app: &mut DbctlApp, ctx: &egui::Context) {
    if !app.schema_designer.show_edit_column_dialog {
        return;
    }

    let (table_name, column_name) = match &app.schema_designer.selected_column {
        Some((t, c)) => (t.clone(), c.clone()),
        None => {
            app.schema_designer.show_edit_column_dialog = false;
            return;
        }
    };

    egui::Window::new(format!("Edit Column '{}.{}'", table_name, column_name))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Grid::new("edit_col_grid").num_columns(2).show(ui, |ui| {
                ui.label("Type:");
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("edit_col_type")
                        .selected_text(&app.schema_designer.edit_column_type)
                        .show_ui(ui, |ui| {
                            for t in &["integer", "bigint", "smallint", "serial", "bigserial", "text", "varchar", "char", "boolean", "timestamp", "timestamptz", "date", "time", "uuid", "jsonb", "json", "numeric", "decimal", "real", "double precision", "bytea"] {
                                ui.selectable_value(&mut app.schema_designer.edit_column_type, t.to_string(), *t);
                            }
                        });

                    // Show size input for types that support it
                    if type_supports_size(&app.schema_designer.edit_column_type) {
                        ui.label("(");
                        ui.add(egui::TextEdit::singleline(&mut app.schema_designer.edit_column_size)
                            .desired_width(50.0)
                            .hint_text("size"));
                        ui.label(")");
                    }
                });
                ui.end_row();

                ui.label("Nullable:");
                ui.checkbox(&mut app.schema_designer.edit_column_nullable, "");
                ui.end_row();

                ui.label("Default:");
                ui.text_edit_singleline(&mut app.schema_designer.edit_column_default);
                ui.end_row();
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    app.schema_designer.show_edit_column_dialog = false;
                }

                if ui.button("Save").clicked() {
                    // Combine type with size
                    let full_type = combine_type_with_size(
                        &app.schema_designer.edit_column_type,
                        &app.schema_designer.edit_column_size,
                    );

                    // Gather data first to avoid borrow issues
                    let action_and_col = {
                        let table = app.schema_designer.working_schema.tables.get(&table_name);
                        if let Some(table) = table {
                            if let Some(col) = table.columns.iter().find(|c| c.name == column_name) {
                                let old_col = col.clone();
                                let new_col = ColumnModel {
                                    name: col.name.clone(),
                                    data_type: full_type.clone(),
                                    is_nullable: app.schema_designer.edit_column_nullable,
                                    default_value: if app.schema_designer.edit_column_default.is_empty() {
                                        None
                                    } else {
                                        Some(app.schema_designer.edit_column_default.clone())
                                    },
                                    is_primary_key: col.is_primary_key,
                                };

                                if old_col.data_type != new_col.data_type
                                    || old_col.is_nullable != new_col.is_nullable
                                    || old_col.default_value != new_col.default_value
                                {
                                    Some((old_col, new_col))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };

                    if let Some((old_col, new_col)) = action_and_col {
                        app.schema_designer.push_action(SchemaAction::AlterColumn {
                            table: table_name.clone(),
                            old: old_col,
                            new: new_col.clone(),
                        });
                        if let Some(table) = app.schema_designer.working_schema.tables.get_mut(&table_name) {
                            if let Some(col) = table.columns.iter_mut().find(|c| c.name == column_name) {
                                *col = new_col;
                            }
                        }
                        app.schema_designer.refresh_pending_sql();
                    }
                    app.schema_designer.show_edit_column_dialog = false;
                }

                // Delete button
                if ui.button(egui::RichText::new("Delete Column").color(egui::Color32::RED)).clicked() {
                    // Gather data first
                    let col_to_drop = app.schema_designer.working_schema.tables
                        .get(&table_name)
                        .and_then(|t| t.columns.iter().find(|c| c.name == column_name).cloned());

                    if let Some(col) = col_to_drop {
                        app.schema_designer.push_action(SchemaAction::DropColumn {
                            table: table_name.clone(),
                            column: col,
                        });
                        if let Some(table) = app.schema_designer.working_schema.tables.get_mut(&table_name) {
                            table.columns.retain(|c| c.name != column_name);
                        }
                        app.schema_designer.selected_column = None;
                        app.schema_designer.refresh_pending_sql();
                    }
                    app.schema_designer.show_edit_column_dialog = false;
                }
            });
        });
}

fn render_apply_migration_dialog(app: &mut DbctlApp, ctx: &egui::Context, cluster: &ClusterInfo) {
    if !app.schema_designer.show_apply_migration_dialog {
        return;
    }

    egui::Window::new("Apply Migration")
        .collapsible(false)
        .resizable(true)
        .default_width(500.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label("The following SQL will be executed:");
            ui.add_space(5.0);

            let sql_text = app.schema_designer.pending_sql.join("\n\n");
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut sql_text.as_str())
                            .font(egui::FontId::monospace(12.0))
                            .desired_width(f32::INFINITY)
                            .interactive(false),
                    );
                });

            if let Some(ref error) = app.schema_designer.migration_error {
                ui.add_space(5.0);
                ui.label(egui::RichText::new(format!("Error: {}", error)).color(egui::Color32::RED));
            }

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    app.schema_designer.show_apply_migration_dialog = false;
                    app.schema_designer.migration_error = None;
                }

                if app.schema_designer.applying_migration {
                    ui.spinner();
                    ui.label("Applying...");
                } else {
                    if ui.button("Apply").clicked() {
                        let db = app.schema_designer.selected_database.clone();
                        if let Some(db) = db {
                            let statements = app.schema_designer.pending_sql.clone();
                            app.apply_migration(&cluster.name, &db, statements);
                        }
                    }
                }
            });
        });
}

fn any_dialog_open(app: &DbctlApp) -> bool {
    app.schema_designer.show_new_table_dialog
        || app.schema_designer.show_new_column_dialog
        || app.schema_designer.show_new_index_dialog
        || app.schema_designer.show_new_fk_dialog
        || app.schema_designer.show_edit_column_dialog
        || app.schema_designer.show_apply_migration_dialog
        || app.schema_designer.show_template_dialog
        || app.schema_designer.show_template_confirm_dialog
}

fn duplicate_selected(app: &mut DbctlApp) {
    if let Some(table_name) = &app.schema_designer.selected_table.clone() {
        if let Some(table) = app.schema_designer.working_schema.tables.get(table_name) {
            let mut new_table = table.clone();

            // Find unique name
            let mut i = 1;
            loop {
                let new_name = format!("{}_copy{}", table_name, i);
                if !app.schema_designer.working_schema.tables.contains_key(&new_name) {
                    new_table.name = new_name;
                    break;
                }
                i += 1;
            }

            // Offset position
            new_table.position = table.position + egui::vec2(30.0, 30.0);

            app.schema_designer.push_action(SchemaAction::CreateTable(new_table.clone()));
            app.schema_designer.working_schema.tables.insert(new_table.name.clone(), new_table.clone());
            app.schema_designer.selected_table = Some(new_table.name);
            app.schema_designer.refresh_pending_sql();
        }
    }
}

fn start_rename(_app: &mut DbctlApp) {
    // Currently not implemented - would need inline text editing
    // For now, tables/columns must be recreated
}

fn delete_selected(app: &mut DbctlApp) {
    if let Some((table_name, column_name)) = app.schema_designer.selected_column.clone() {
        // Delete column - no confirmation needed
        let col_to_drop = app.schema_designer.working_schema.tables
            .get(&table_name)
            .and_then(|t| t.columns.iter().find(|c| c.name == column_name).cloned());

        if let Some(col) = col_to_drop {
            app.schema_designer.push_action(SchemaAction::DropColumn {
                table: table_name.clone(),
                column: col,
            });
            if let Some(table) = app.schema_designer.working_schema.tables.get_mut(&table_name) {
                table.columns.retain(|c| c.name != column_name);
            }
            app.schema_designer.selected_column = None;
            app.schema_designer.refresh_pending_sql();
        }
    } else if let Some(table_name) = app.schema_designer.selected_table.clone() {
        // Delete table - show confirmation dialog
        app.schema_designer.pending_delete_table = Some(table_name);
        app.schema_designer.show_delete_table_dialog = true;
    }
}

fn confirm_delete_table(app: &mut DbctlApp) {
    if let Some(table_name) = app.schema_designer.pending_delete_table.take() {
        if let Some(table) = app.schema_designer.working_schema.tables.remove(&table_name) {
            app.schema_designer.push_action(SchemaAction::DropTable(table));
            app.schema_designer.selected_table = None;
            app.schema_designer.refresh_pending_sql();
        }
    }
    app.schema_designer.show_delete_table_dialog = false;
}

fn cancel_current_operation(app: &mut DbctlApp) {
    app.schema_designer.show_new_table_dialog = false;
    app.schema_designer.show_new_column_dialog = false;
    app.schema_designer.show_new_index_dialog = false;
    app.schema_designer.show_new_fk_dialog = false;
    app.schema_designer.show_edit_column_dialog = false;
    app.schema_designer.show_apply_migration_dialog = false;
    app.schema_designer.show_template_dialog = false;
    app.schema_designer.show_template_confirm_dialog = false;
    app.schema_designer.show_delete_table_dialog = false;
    app.schema_designer.pending_delete_table = None;
    app.schema_designer.renaming_target = None;
}

fn render_template_dialog(app: &mut DbctlApp, ctx: &egui::Context) {
    if !app.schema_designer.show_template_dialog {
        return;
    }

    let templates = get_all_templates();

    egui::Window::new("Schema Templates")
        .collapsible(false)
        .resizable(false)
        .fixed_size([750.0, 500.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            let content_height = 430.0;

            // Two-column layout using columns
            ui.columns(2, |columns| {
                // Left column: Template list (narrower)
                columns[0].set_max_width(250.0);
                columns[0].vertical(|ui| {
                    ui.set_min_height(content_height);

                    // Search box
                    ui.horizontal(|ui| {
                        ui.label("Search:");
                        ui.add_sized(
                            [150.0, 20.0],
                            egui::TextEdit::singleline(&mut app.schema_designer.template_search_query),
                        );
                    });
                    ui.add_space(8.0);

                    // Filter templates by search
                    let search_lower = app.schema_designer.template_search_query.to_lowercase();
                    let filtered_templates: Vec<_> = templates
                        .iter()
                        .filter(|t| {
                            if search_lower.is_empty() {
                                return true;
                            }
                            t.name.to_lowercase().contains(&search_lower)
                                || t.description.to_lowercase().contains(&search_lower)
                                || t.tags.iter().any(|tag| tag.to_lowercase().contains(&search_lower))
                        })
                        .collect();

                    egui::ScrollArea::vertical()
                        .id_salt("template_list")
                        .max_height(content_height - 50.0)
                        .show(ui, |ui| {
                            ui.set_min_width(230.0);

                            // Group by category
                            for category in [TemplateCategory::Comprehensive, TemplateCategory::Starter] {
                                let category_templates: Vec<_> = filtered_templates
                                    .iter()
                                    .filter(|t| t.category == category)
                                    .collect();

                                if category_templates.is_empty() {
                                    continue;
                                }

                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new(category.label())
                                        .strong()
                                        .size(12.0)
                                        .color(app.theme.text_muted),
                                );
                                ui.add_space(4.0);

                                for template in category_templates {
                                    let is_selected = app.schema_designer.selected_template.as_ref()
                                        == Some(&template.id.to_string());

                                    let response = ui.selectable_label(
                                        is_selected,
                                        format!("{} ({} tables)", template.name, template.table_count()),
                                    );

                                    if response.clicked() {
                                        app.schema_designer.selected_template =
                                            Some(template.id.to_string());
                                        app.schema_designer.template_preview =
                                            Some(template.generate());
                                    }
                                }
                            }
                        });
                });

                // Right column: Preview (wider)
                columns[1].vertical(|ui| {
                    ui.set_min_height(content_height);

                    if let Some(preview) = &app.schema_designer.template_preview {
                        // Find the template for description
                        let template = app.schema_designer.selected_template.as_ref().and_then(|id| {
                            templates.iter().find(|t| t.id == id)
                        });

                        if let Some(template) = template {
                            ui.heading(template.name);
                            ui.label(template.description);
                            ui.add_space(8.0);

                            // Tags
                            ui.horizontal(|ui| {
                                for tag in &template.tags {
                                    ui.label(
                                        egui::RichText::new(format!("#{}", tag))
                                            .size(11.0)
                                            .color(app.theme.text_muted),
                                    );
                                }
                            });
                            ui.add_space(12.0);
                        }

                        // Table list preview
                        ui.label(
                            egui::RichText::new(format!("Tables ({})", preview.tables.len()))
                                .strong(),
                        );
                        ui.separator();

                        egui::ScrollArea::vertical()
                            .id_salt("template_preview")
                            .max_height(content_height - 100.0)
                            .show(ui, |ui| {
                                render_template_preview(ui, preview, &app.theme);
                            });
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label("Select a template to preview");
                        });
                    }
                });
            });

            ui.separator();

            // Action buttons
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    app.schema_designer.show_template_dialog = false;
                    app.schema_designer.selected_template = None;
                    app.schema_designer.template_preview = None;
                }

                let can_apply = app.schema_designer.selected_template.is_some();
                if ui
                    .add_enabled(can_apply, egui::Button::new("Apply Template"))
                    .clicked()
                {
                    // Open confirmation dialog instead of applying immediately
                    app.schema_designer.show_template_confirm_dialog = true;
                    app.schema_designer.show_template_dialog = false;
                }

                if can_apply {
                    ui.label(
                        egui::RichText::new("This will add tables to your current schema")
                            .size(11.0)
                            .color(app.theme.text_muted),
                    );
                }
            });
        });
}

fn render_template_confirm_dialog(app: &mut DbctlApp, ctx: &egui::Context) {
    if !app.schema_designer.show_template_confirm_dialog {
        return;
    }

    let templates = get_all_templates();
    let template_info = app.schema_designer.selected_template.as_ref().and_then(|id| {
        templates.iter().find(|t| t.id == *id).map(|t| (t.name, t.description))
    });

    let (template_name, template_desc) = template_info.unwrap_or(("Unknown", ""));

    // Clone the preview to avoid borrow issues
    let preview = app.schema_designer.template_preview.clone();

    // Pre-compute warning about existing tables
    let existing_tables: Vec<String> = preview
        .as_ref()
        .map(|schema| {
            schema
                .tables
                .keys()
                .filter(|name| app.schema_designer.working_schema.tables.contains_key(*name))
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    egui::Window::new("Confirm Template Application")
        .collapsible(false)
        .resizable(false)
        .fixed_size([450.0, 350.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new(format!("Apply \"{}\" template?", template_name))
                    .strong()
                    .size(14.0),
            );

            ui.add_space(8.0);
            ui.label(template_desc);
            ui.add_space(4.0);

            ui.separator();

            // Show tables that will be created
            if let Some(schema) = &preview {
                ui.label(
                    egui::RichText::new(format!("The following {} tables will be created:", schema.tables.len()))
                        .color(app.theme.text_muted),
                );

                ui.add_space(4.0);

                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .show(ui, |ui| {
                        let mut table_names: Vec<_> = schema.tables.keys().collect();
                        table_names.sort();

                        for name in table_names {
                            if let Some(table) = schema.tables.get(name) {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(name)
                                            .monospace()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("({} columns)", table.columns.len()))
                                            .size(11.0)
                                            .color(app.theme.text_muted),
                                    );
                                });
                            }
                        }
                    });
            }

            ui.add_space(8.0);
            ui.separator();

            // Warning about existing tables
            if !existing_tables.is_empty() {
                ui.colored_label(
                    app.theme.warning,
                    format!(
                        "Warning: {} table(s) already exist and will be overwritten: {}",
                        existing_tables.len(),
                        existing_tables.join(", ")
                    ),
                );
                ui.add_space(4.0);
            }

            // Action buttons
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    app.schema_designer.show_template_confirm_dialog = false;
                    app.schema_designer.selected_template = None;
                    app.schema_designer.template_preview = None;
                }

                if ui.button("Go Back").clicked() {
                    // Return to template selection
                    app.schema_designer.show_template_confirm_dialog = false;
                    app.schema_designer.show_template_dialog = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Confirm & Apply").clicked() {
                        apply_template(app);
                        app.schema_designer.show_template_confirm_dialog = false;
                    }
                });
            });
        });
}

fn render_template_preview(ui: &mut egui::Ui, schema: &SchemaModel, theme: &crate::theme::Theme) {
    // Sort tables by name for consistent display
    let mut table_names: Vec<_> = schema.tables.keys().collect();
    table_names.sort();

    for table_name in table_names {
        if let Some(table) = schema.tables.get(table_name) {
            egui::CollapsingHeader::new(
                egui::RichText::new(&table.name).strong().monospace(),
            )
            .default_open(false)
            .show(ui, |ui| {
                // Columns
                for col in &table.columns {
                    ui.horizontal(|ui| {
                        // Primary key indicator
                        if col.is_primary_key {
                            ui.label(
                                egui::RichText::new("PK")
                                    .size(10.0)
                                    .strong()
                                    .color(theme.pk_badge_text),
                            );
                        } else {
                            ui.add_space(20.0);
                        }

                        ui.label(
                            egui::RichText::new(&col.name)
                                .monospace()
                                .size(11.0),
                        );
                        ui.label(
                            egui::RichText::new(&col.data_type)
                                .monospace()
                                .size(11.0)
                                .color(theme.text_muted),
                        );

                        if !col.is_nullable {
                            ui.label(
                                egui::RichText::new("NOT NULL")
                                    .size(10.0)
                                    .color(theme.text_muted),
                            );
                        }
                    });
                }

                // Foreign keys
                let fks: Vec<_> = table
                    .constraints
                    .iter()
                    .filter(|c| c.constraint_type == crate::schema_designer::ConstraintType::ForeignKey)
                    .collect();

                if !fks.is_empty() {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Foreign Keys:")
                            .size(10.0)
                            .color(theme.text_muted),
                    );
                    for fk in fks {
                        if let (Some(ref_table), Some(ref_cols)) =
                            (&fk.foreign_table, &fk.foreign_columns)
                        {
                            ui.label(
                                egui::RichText::new(format!(
                                    "  {} -> {}.{}",
                                    fk.columns.join(", "),
                                    ref_table,
                                    ref_cols.join(", ")
                                ))
                                .monospace()
                                .size(10.0)
                                .color(theme.fk_line),
                            );
                        }
                    }
                }

                // Indexes
                if !table.indexes.is_empty() {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Indexes:")
                            .size(10.0)
                            .color(theme.text_muted),
                    );
                    for idx in &table.indexes {
                        let unique_str = if idx.is_unique { "UNIQUE " } else { "" };
                        ui.label(
                            egui::RichText::new(format!(
                                "  {}{}({})",
                                unique_str,
                                idx.name,
                                idx.columns.join(", ")
                            ))
                            .monospace()
                            .size(10.0)
                            .color(theme.text_muted),
                        );
                    }
                }
            });
        }
    }
}

fn apply_template(app: &mut DbctlApp) {
    if let Some(template_id) = &app.schema_designer.selected_template.clone() {
        let templates = get_all_templates();
        if let Some(template) = templates.iter().find(|t| t.id == *template_id) {
            let schema = template.generate();

            // Collect names of new tables being added
            let new_table_names: Vec<String> = schema.tables.keys().cloned().collect();

            // Create batch action for all tables
            let actions: Vec<SchemaAction> = schema
                .tables
                .values()
                .map(|table| SchemaAction::CreateTable(table.clone()))
                .collect();

            if !actions.is_empty() {
                app.schema_designer.push_action(SchemaAction::Batch(actions));

                // Update working schema
                for (name, table) in schema.tables {
                    app.schema_designer
                        .working_schema
                        .tables
                        .insert(name, table);
                }

                // Layout only the new tables, positioning them below existing tables
                layout_new_tables(&mut app.schema_designer.working_schema, &new_table_names);

                app.schema_designer.refresh_pending_sql();
            }

            app.schema_designer.show_template_dialog = false;
            app.schema_designer.selected_template = None;
            app.schema_designer.template_preview = None;
        }
    }
}

fn render_ai_chat_sidebar(app: &mut DbctlApp, ui: &mut egui::Ui) {
    egui::Frame::NONE
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.set_width(app.ai_chat.width);
            ui.set_min_height(ui.available_height());

            // Header
            ui.horizontal(|ui| {
                ui.heading("AI Schema Assistant");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("X").clicked() {
                        app.ai_chat.visible = false;
                    }
                    if ui.button("Clear").clicked() {
                        app.ai_chat.clear_history();
                    }
                });
            });

            ui.separator();

            // API key status
            if !ApiKeyManager::has_api_key() {
                ui.colored_label(app.theme.warning, "API key not configured");
                if ui.button("Configure API Key").clicked() {
                    app.show_api_key_dialog = true;
                }
                ui.separator();
            }

            // Chat history
            let history_height = ui.available_height() - 100.0;
            egui::ScrollArea::vertical()
                .max_height(history_height)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for message in &app.ai_chat.messages {
                        render_chat_message(ui, message, &app.theme);
                    }

                    if app.ai_chat.is_loading {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Thinking...");
                        });
                    }
                });

            // Pending changes proposal
            if let Some(proposal) = app.ai_chat.pending_changes.clone() {
                ui.separator();
                ui.label(
                    egui::RichText::new("Proposed changes:")
                        .strong()
                        .color(app.theme.success),
                );
                ui.label(&proposal.description);

                ui.horizontal(|ui| {
                    if ui.button("Apply Changes").clicked() {
                        apply_ai_changes(app, &proposal);
                    }
                    if ui.button("Reject").clicked() {
                        app.ai_chat.pending_changes = None;
                    }
                });
            }

            // Error message
            if let Some(error) = &app.ai_chat.error_message {
                ui.colored_label(app.theme.warning, error);
            }

            ui.separator();

            // Input area
            ui.horizontal(|ui| {
                let response = ui.add(
                    egui::TextEdit::multiline(&mut app.ai_chat.input_text)
                        .desired_rows(2)
                        .desired_width(ui.available_width() - 60.0)
                        .hint_text("Describe your schema..."),
                );

                // Send on Cmd+Enter
                if response.has_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.command)
                {
                    app.send_ai_chat_message();
                }

                let can_send = !app.ai_chat.input_text.is_empty() && !app.ai_chat.is_loading;
                if ui
                    .add_enabled(can_send, egui::Button::new("Send"))
                    .clicked()
                {
                    app.send_ai_chat_message();
                }
            });
        });
}

fn render_chat_message(
    ui: &mut egui::Ui,
    message: &crate::ai_chat::ChatMessage,
    theme: &crate::theme::Theme,
) {
    let bg_color = match message.role {
        MessageRole::User => theme.chat_user_bg,
        MessageRole::Assistant => theme.chat_assistant_bg,
    };

    let role_label = match message.role {
        MessageRole::User => "You",
        MessageRole::Assistant => "AI",
    };

    egui::Frame::new()
        .fill(bg_color)
        .inner_margin(egui::Margin::same(8))
        .corner_radius(4.0)
        .show(ui, |ui| {
            ui.label(egui::RichText::new(role_label).strong().size(11.0));
            ui.label(&message.content);
        });

    ui.add_space(4.0);
}

fn apply_ai_changes(app: &mut DbctlApp, proposal: &SchemaChangeProposal) {
    let mut actions = Vec::new();

    // Add new tables
    for table in &proposal.tables_to_add {
        actions.push(SchemaAction::CreateTable(table.clone()));
    }

    // Remove tables
    for table_name in &proposal.tables_to_remove {
        if let Some(table) = app
            .schema_designer
            .working_schema
            .tables
            .get(table_name)
            .cloned()
        {
            actions.push(SchemaAction::DropTable(table));
        }
    }

    // Modify tables (add columns)
    for modification in &proposal.tables_to_modify {
        for col in &modification.columns_to_add {
            actions.push(SchemaAction::AddColumn {
                table: modification.table_name.clone(),
                column: col.clone(),
            });
        }
        for col_name in &modification.columns_to_remove {
            if let Some(table) = app
                .schema_designer
                .working_schema
                .tables
                .get(&modification.table_name)
            {
                if let Some(col) = table.columns.iter().find(|c| &c.name == col_name) {
                    actions.push(SchemaAction::DropColumn {
                        table: modification.table_name.clone(),
                        column: col.clone(),
                    });
                }
            }
        }
    }

    if !actions.is_empty() {
        app.schema_designer
            .push_action(SchemaAction::Batch(actions.clone()));

        // Apply changes to working schema
        for action in actions {
            match action {
                SchemaAction::CreateTable(table) => {
                    app.schema_designer
                        .working_schema
                        .tables
                        .insert(table.name.clone(), table);
                }
                SchemaAction::DropTable(table) => {
                    app.schema_designer.working_schema.tables.remove(&table.name);
                }
                SchemaAction::AddColumn { table, column } => {
                    if let Some(t) = app.schema_designer.working_schema.tables.get_mut(&table) {
                        t.columns.push(column);
                    }
                }
                SchemaAction::DropColumn { table, column } => {
                    if let Some(t) = app.schema_designer.working_schema.tables.get_mut(&table) {
                        t.columns.retain(|c| c.name != column.name);
                    }
                }
                _ => {}
            }
        }

        app.schema_designer.refresh_pending_sql();
    }

    app.ai_chat.pending_changes = None;
}

fn render_delete_table_dialog(app: &mut DbctlApp, ctx: &egui::Context) {
    if !app.schema_designer.show_delete_table_dialog {
        return;
    }

    let table_name = app
        .schema_designer
        .pending_delete_table
        .clone()
        .unwrap_or_default();

    egui::Window::new("Delete Table")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(format!(
                "Are you sure you want to delete table '{}'?",
                table_name
            ));

            ui.add_space(5.0);

            ui.label(
                egui::RichText::new("This action can be undone using Cmd+Z.")
                    .size(11.0)
                    .color(ui.visuals().text_color().gamma_multiply(0.7)),
            );

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    app.schema_designer.show_delete_table_dialog = false;
                    app.schema_designer.pending_delete_table = None;
                }

                if ui
                    .add(egui::Button::new("Delete").fill(app.theme.warning))
                    .clicked()
                {
                    confirm_delete_table(app);
                }
            });
        });
}

fn render_api_key_dialog(app: &mut DbctlApp, ctx: &egui::Context) {
    if !app.show_api_key_dialog {
        return;
    }

    egui::Window::new("Configure API Key")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label("Enter your Anthropic API key:");
            ui.label(
                egui::RichText::new("(Key is stored securely in your system keychain)")
                    .size(11.0)
                    .color(ui.visuals().text_color().gamma_multiply(0.7)),
            );

            ui.add_space(10.0);

            ui.add(
                egui::TextEdit::singleline(&mut app.api_key_input)
                    .password(true)
                    .desired_width(350.0)
                    .hint_text("sk-ant-..."),
            );

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    app.show_api_key_dialog = false;
                    app.api_key_input.clear();
                }

                let can_save = app.api_key_input.starts_with("sk-ant-")
                    || app.api_key_input.starts_with("sk-");
                if ui
                    .add_enabled(can_save, egui::Button::new("Save"))
                    .clicked()
                {
                    match ApiKeyManager::set_api_key(&app.api_key_input) {
                        Ok(()) => {
                            app.status_message = "API key saved".to_string();
                            app.show_api_key_dialog = false;
                            app.api_key_input.clear();
                        }
                        Err(e) => {
                            app.status_message = format!("Failed to save API key: {}", e);
                        }
                    }
                }
            });

            ui.add_space(5.0);

            ui.hyperlink_to(
                "Get an API key from Anthropic Console",
                "https://console.anthropic.com/settings/keys",
            );
        });
}

fn render_history_panel(app: &mut DbctlApp, ui: &mut egui::Ui) {
    let panel_width = 280.0;

    // Collect action data upfront to avoid borrow issues
    let action_count = app.schema_designer.action_history.len();
    let current_index = app.schema_designer.action_index;
    let entries: Vec<(String, String)> = app
        .schema_designer
        .action_history
        .iter()
        .map(|a| (a.description(), a.format_timestamp()))
        .collect();

    // Allocate space for the panel
    ui.allocate_ui_with_layout(
        egui::vec2(panel_width, ui.available_height()),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
            ui.add_space(8.0);

            // Header
            ui.horizontal(|ui| {
                ui.heading("History");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("X").on_hover_text("Close").clicked() {
                        app.schema_designer.history_panel_visible = false;
                    }
                });
            });

            ui.separator();

            if action_count == 0 {
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new("No changes yet")
                        .italics()
                        .color(app.theme.text_muted),
                );
            } else {
                // Quick navigation buttons
                ui.horizontal(|ui| {
                    let can_go_start = current_index > 0;
                    let can_go_end = current_index < action_count;

                    if ui
                        .add_enabled(can_go_start, egui::Button::new("<< Start"))
                        .on_hover_text("Jump to initial state")
                        .clicked()
                    {
                        app.schema_designer.jump_to_action(0);
                    }

                    if ui
                        .add_enabled(can_go_end, egui::Button::new("End >>"))
                        .on_hover_text("Jump to latest state")
                        .clicked()
                    {
                        app.schema_designer.jump_to_action(action_count);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("{}/{}", current_index, action_count))
                                .size(11.0)
                                .color(app.theme.text_muted),
                        );
                    });
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                // Timeline entries
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Initial state entry (index 0)
                        let is_current = current_index == 0;
                        render_history_entry(
                            ui,
                            0,
                            "Initial state",
                            "",
                            is_current,
                            false,
                            &app.theme,
                            &mut app.schema_designer,
                        );

                        // Action entries (1 to action_count)
                        for (i, (description, timestamp)) in entries.iter().enumerate() {
                            let is_current = current_index == i + 1;
                            let is_future = i + 1 > current_index;

                            render_history_entry(
                                ui,
                                i + 1,
                                description,
                                timestamp,
                                is_current,
                                is_future,
                                &app.theme,
                                &mut app.schema_designer,
                            );
                        }
                    });
            }
        },
    );
}

fn render_history_entry(
    ui: &mut egui::Ui,
    index: usize,
    description: &str,
    timestamp: &str,
    is_current: bool,
    is_future: bool,
    theme: &crate::theme::Theme,
    state: &mut crate::schema_designer::SchemaDesignerState,
) {
    let bg_color = if is_current {
        theme.selection_bg
    } else {
        egui::Color32::TRANSPARENT
    };

    let text_color = if is_future {
        theme.text_muted
    } else if is_current {
        theme.selection_text
    } else {
        ui.visuals().text_color()
    };

    let response = egui::Frame::new()
        .fill(bg_color)
        .inner_margin(egui::Margin::symmetric(6, 4))
        .corner_radius(4.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.horizontal(|ui| {
                // Current position indicator
                if is_current {
                    ui.label(
                        egui::RichText::new(">")
                            .strong()
                            .color(theme.success),
                    );
                } else {
                    ui.add_space(12.0);
                }

                ui.vertical(|ui| {
                    // Description
                    let desc_text = if is_current {
                        egui::RichText::new(description).strong().color(text_color)
                    } else {
                        egui::RichText::new(description).color(text_color)
                    };
                    ui.label(desc_text);

                    // Timestamp (only for non-initial entries)
                    if !timestamp.is_empty() {
                        ui.label(
                            egui::RichText::new(timestamp)
                                .size(10.0)
                                .color(theme.text_muted),
                        );
                    }
                });
            });
        })
        .response;

    // Handle click to jump to this state
    if response.interact(egui::Sense::click()).clicked() && !is_current {
        state.jump_to_action(index);
    }

    // Show hover effect
    if response.hovered() && !is_current {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
}
