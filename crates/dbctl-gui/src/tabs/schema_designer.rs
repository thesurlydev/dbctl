use eframe::egui;

use crate::schema_designer::{
    canvas::{auto_layout_tables, render_canvas},
    ColumnModel, ConstraintModel, FkAction, IndexModel, SchemaAction, TableModel,
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

    // Top toolbar
    render_toolbar(app, ui, cluster);

    ui.separator();

    // Main content - split between canvas and SQL preview
    let available_height = ui.available_height();
    let sql_preview_height = if app.schema_designer.sql_preview_expanded { 150.0 } else { 30.0 };
    let canvas_height = available_height - sql_preview_height - 10.0;

    // Canvas area
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), canvas_height),
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

    // SQL Preview panel
    render_sql_preview(app, ui);

    // Dialogs
    render_new_table_dialog(app, &ctx);
    render_new_column_dialog(app, &ctx);
    render_new_index_dialog(app, &ctx);
    render_new_fk_dialog(app, &ctx);
    render_edit_column_dialog(app, &ctx);
    render_apply_migration_dialog(app, &ctx, cluster);
}

fn render_toolbar(app: &mut DbctlApp, ui: &mut egui::Ui, cluster: &ClusterInfo) {
    ui.horizontal(|ui| {
        // Database selector
        ui.label("Database:");
        let current_db = app.schema_designer.selected_database.clone().unwrap_or_default();
        let databases = app.databases.clone();
        let cluster_name = cluster.name.clone();
        let mut selected_db: Option<String> = None;

        egui::ComboBox::from_id_salt("schema_designer_db")
            .selected_text(&current_db)
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

        // Schema selector
        ui.label("Schema:");
        ui.add(egui::TextEdit::singleline(&mut app.schema_designer.selected_schema).desired_width(80.0));

        ui.separator();

        // Action buttons
        if ui.button("+ Table").on_hover_text("Cmd+N").clicked() {
            app.schema_designer.reset_new_table_form();
            app.schema_designer.show_new_table_dialog = true;
        }

        // Only show these if a table is selected
        if app.schema_designer.selected_table.is_some() {
            if ui.button("+ Column").clicked() {
                app.schema_designer.reset_new_column_form();
                app.schema_designer.show_new_column_dialog = true;
            }
            if ui.button("+ Index").clicked() {
                app.schema_designer.reset_new_index_form();
                app.schema_designer.show_new_index_dialog = true;
            }
            if ui.button("+ FK").clicked() {
                app.schema_designer.reset_new_fk_form();
                app.schema_designer.show_new_fk_dialog = true;
            }
        }

        ui.separator();

        // Undo/Redo
        let can_undo = app.schema_designer.action_index > 0;
        let can_redo = app.schema_designer.action_index < app.schema_designer.action_history.len();

        if ui.add_enabled(can_undo, egui::Button::new("Undo")).on_hover_text("Cmd+Z").clicked() {
            app.schema_designer.undo();
        }
        if ui.add_enabled(can_redo, egui::Button::new("Redo")).on_hover_text("Cmd+Shift+Z").clicked() {
            app.schema_designer.redo();
        }

        ui.separator();

        // Auto-layout
        if ui.button("Auto Layout").clicked() {
            auto_layout_tables(&mut app.schema_designer.working_schema);
        }

        // View controls
        ui.separator();
        ui.checkbox(&mut app.schema_designer.canvas.show_grid, "Grid");
        ui.checkbox(&mut app.schema_designer.canvas.snap_to_grid, "Snap");

        // Right side - apply changes
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let has_changes = app.schema_designer.has_changes();
            if ui.add_enabled(has_changes, egui::Button::new("Apply Changes"))
                .on_hover_text("Cmd+S")
                .clicked()
            {
                app.schema_designer.show_apply_migration_dialog = true;
            }

            if has_changes {
                ui.label(format!("{} pending change(s)", app.schema_designer.pending_sql.len()));
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

                    // Create table with default position
                    let table_count = app.schema_designer.working_schema.tables.len();
                    let pos = egui::pos2(
                        50.0 + (table_count % 3) as f32 * 280.0,
                        50.0 + (table_count / 3) as f32 * 200.0,
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
        // Delete column - gather data first
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
        // Delete table
        if let Some(table) = app.schema_designer.working_schema.tables.remove(&table_name) {
            app.schema_designer.push_action(SchemaAction::DropTable(table));
            app.schema_designer.selected_table = None;
            app.schema_designer.refresh_pending_sql();
        }
    }
}

fn cancel_current_operation(app: &mut DbctlApp) {
    app.schema_designer.show_new_table_dialog = false;
    app.schema_designer.show_new_column_dialog = false;
    app.schema_designer.show_new_index_dialog = false;
    app.schema_designer.show_new_fk_dialog = false;
    app.schema_designer.show_edit_column_dialog = false;
    app.schema_designer.show_apply_migration_dialog = false;
    app.schema_designer.renaming_target = None;
}
