use eframe::egui;

use crate::state::ClusterInfo;
use crate::DbctlApp;

pub fn render_tables_tab(app: &mut DbctlApp, ui: &mut egui::Ui, cluster: &ClusterInfo) {
    let cluster_name = cluster.name.clone();

    // Database selector at top
    ui.horizontal(|ui| {
        ui.label("Database:");
        egui::ComboBox::from_id_salt("table_browser_db")
            .selected_text(
                app.table_browser
                    .selected_database
                    .as_deref()
                    .unwrap_or("Select database..."),
            )
            .show_ui(ui, |ui| {
                for db in &app.databases.clone() {
                    let selected = app.table_browser.selected_database.as_ref() == Some(db);
                    if ui.selectable_label(selected, db).clicked() {
                        // Clear previous state when switching databases
                        app.table_browser.schemas.clear();
                        app.table_browser.tables.clear();
                        app.table_browser.selected_schema = None;
                        app.table_browser.selected_table = None;
                        app.table_browser.table_structure = None;
                        app.table_browser.table_data = None;
                        app.table_browser.expanded_schemas.clear();
                        app.table_browser.selected_database = Some(db.clone());
                        app.fetch_schemas(&cluster_name, db);
                    }
                }
            });

        if ui.button("Refresh").clicked() {
            if let Some(db) = &app.table_browser.selected_database.clone() {
                app.fetch_schemas(&cluster_name, db);
            }
        }
    });

    ui.separator();

    if app.table_browser.selected_database.is_none() {
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
                render_schema_tree(app, ui, &cluster_name);
            },
        );

        ui.separator();

        // Right panel - Table details
        ui.allocate_ui_with_layout(
            egui::vec2(available_width - 210.0, ui.available_height()),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                if app.table_browser.selected_table.is_some() {
                    // Structure panel (collapsible)
                    egui::CollapsingHeader::new("Table Structure")
                        .default_open(true)
                        .show(ui, |ui| {
                            render_table_structure(app, ui);
                        });

                    ui.separator();

                    // Data panel
                    render_table_data(app, ui, &cluster_name);
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("Select a table from the schema tree to view its data");
                    });
                }
            },
        );
    });
}

fn render_schema_tree(app: &mut DbctlApp, ui: &mut egui::Ui, cluster: &str) {
    ui.heading("Schemas");

    if app.table_browser.loading_schemas {
        ui.spinner();
        return;
    }

    egui::ScrollArea::vertical()
        .id_salt("schema_tree_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let schemas: Vec<_> = app.table_browser.schemas.iter().cloned().collect();
            for schema in schemas {
                let is_expanded = app.table_browser.expanded_schemas.contains(&schema.name);

                let header_response = egui::CollapsingHeader::new(&schema.name)
                    .default_open(schema.name == "public")
                    .show(ui, |ui| {
                        // Show loading indicator or tables
                        if app.table_browser.loading_tables
                            && app.table_browser.selected_schema.as_ref() == Some(&schema.name)
                        {
                            ui.spinner();
                        } else if app.table_browser.selected_schema.as_ref() == Some(&schema.name) {
                            for table in &app.table_browser.tables.clone() {
                                let selected =
                                    app.table_browser.selected_table.as_ref() == Some(&table.name);
                                let icon = if table.table_type == "VIEW" {
                                    "V"
                                } else {
                                    "T"
                                };
                                let label = format!("[{}] {}", icon, table.name);
                                if ui.selectable_label(selected, label).clicked() {
                                    app.table_browser.selected_table = Some(table.name.clone());
                                    app.table_browser.current_page = 0;
                                    if let Some(db) =
                                        &app.table_browser.selected_database.clone()
                                    {
                                        app.fetch_table_structure(
                                            cluster,
                                            db,
                                            &schema.name,
                                            &table.name,
                                        );
                                        app.fetch_table_data(
                                            cluster,
                                            db,
                                            &schema.name,
                                            &table.name,
                                            0,
                                        );
                                    }
                                }
                            }
                            if app.table_browser.tables.is_empty() {
                                ui.label("(no tables)");
                            }
                        }
                    });

                // Handle schema expansion
                if header_response.fully_open() && !is_expanded {
                    app.table_browser
                        .expanded_schemas
                        .insert(schema.name.clone());
                    app.table_browser.selected_schema = Some(schema.name.clone());
                    if let Some(db) = &app.table_browser.selected_database.clone() {
                        app.fetch_schema_tables(cluster, db, &schema.name);
                    }
                } else if !header_response.fully_open() && is_expanded {
                    app.table_browser.expanded_schemas.remove(&schema.name);
                }
            }
        });
}

fn render_table_structure(app: &mut DbctlApp, ui: &mut egui::Ui) {
    if app.table_browser.loading_structure {
        ui.spinner();
        return;
    }

    if let Some(structure) = &app.table_browser.table_structure {
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
            egui::CollapsingHeader::new(format!(
                "Constraints ({})",
                structure.constraints.len()
            ))
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

fn render_table_data(app: &mut DbctlApp, ui: &mut egui::Ui, cluster: &str) {
    if app.table_browser.loading_data {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Loading data...");
        });
        return;
    }

    let has_pk = app
        .table_browser
        .table_structure
        .as_ref()
        .is_some_and(|s| !s.primary_key_columns.is_empty());

    // Toolbar
    ui.horizontal(|ui| {
        if has_pk {
            if ui.button("+ Insert Row").clicked() {
                app.table_browser.show_insert_dialog = true;
                app.table_browser.new_row_values.clear();
            }
        } else {
            ui.add_enabled(false, egui::Button::new("+ Insert Row"))
                .on_disabled_hover_text("Table has no primary key - inline editing disabled");
        }

        if ui.button("Export CSV").clicked() {
            if let Some(data) = &app.table_browser.table_data {
                let csv = dbctl::postgres::export_to_csv(data);
                ui.ctx().copy_text(csv);
                app.status_message = "CSV copied to clipboard".to_string();
            }
        }

        if ui.button("Export JSON").clicked() {
            if let Some(data) = &app.table_browser.table_data {
                if let Ok(json) = dbctl::postgres::export_to_json(data) {
                    ui.ctx().copy_text(json);
                    app.status_message = "JSON copied to clipboard".to_string();
                }
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(format!("{} rows total", app.table_browser.total_rows));
        });
    });

    ui.separator();

    if let Some(data) = &app.table_browser.table_data.clone() {
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
                                    if let Some(structure) = &app.table_browser.table_structure {
                                        if let Some(pk_col) =
                                            structure.primary_key_columns.first()
                                        {
                                            if let Some(pk_val) =
                                                row.get(pk_col).and_then(|v| v.as_ref())
                                            {
                                                if ui.small_button("Delete").clicked() {
                                                    app.table_browser.show_delete_dialog = true;
                                                    app.table_browser.pending_delete_pk =
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
            let total_pages =
                ((data.total_rows as usize).saturating_sub(1)) / data.page_size + 1;

            if ui
                .add_enabled(data.page > 0, egui::Button::new("<"))
                .clicked()
            {
                app.table_browser.current_page = data.page.saturating_sub(1);
                app.refresh_table_data(cluster);
            }

            ui.label(format!("Page {} of {}", data.page + 1, total_pages.max(1)));

            if ui
                .add_enabled(data.page + 1 < total_pages, egui::Button::new(">"))
                .clicked()
            {
                app.table_browser.current_page = data.page + 1;
                app.refresh_table_data(cluster);
            }

            ui.separator();

            ui.label("Page size:");
            egui::ComboBox::from_id_salt("page_size")
                .selected_text(format!("{}", app.table_browser.page_size))
                .show_ui(ui, |ui| {
                    for size in [25, 50, 100, 250] {
                        if ui
                            .selectable_value(
                                &mut app.table_browser.page_size,
                                size,
                                format!("{}", size),
                            )
                            .clicked()
                        {
                            app.table_browser.current_page = 0;
                            app.refresh_table_data(cluster);
                        }
                    }
                });
        });
    }

    // Render dialogs
    render_insert_row_dialog(app, ui.ctx());
    render_delete_row_dialog(app, ui.ctx());
}

fn render_insert_row_dialog(app: &mut DbctlApp, ctx: &egui::Context) {
    if !app.table_browser.show_insert_dialog {
        return;
    }

    let structure = match &app.table_browser.table_structure {
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

                        let value = app
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
                        app.selected_cluster().map(|c| c.name.clone()),
                        app.table_browser.selected_database.clone(),
                        app.table_browser.selected_schema.clone(),
                        app.table_browser.selected_table.clone(),
                    ) {
                        // Filter out empty values
                        app.table_browser
                            .new_row_values
                            .retain(|_, v| !v.is_empty());
                        app.insert_table_row(&cluster, &db, &schema, &table);
                        app.table_browser.show_insert_dialog = false;
                        // Refresh data after a short delay
                        app.refresh_table_data(&cluster);
                    }
                }
                if ui.button("Cancel").clicked() {
                    app.table_browser.show_insert_dialog = false;
                    app.table_browser.new_row_values.clear();
                }
            });
        });
}

fn render_delete_row_dialog(app: &mut DbctlApp, ctx: &egui::Context) {
    if !app.table_browser.show_delete_dialog {
        return;
    }

    let pk_value = match &app.table_browser.pending_delete_pk {
        Some(v) => v.clone(),
        None => return,
    };

    egui::Window::new("Confirm Delete")
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(format!(
                "Delete row with primary key = '{}'?",
                pk_value
            ));
            ui.colored_label(egui::Color32::RED, "This cannot be undone!");

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Delete").clicked() {
                    if let (
                        Some(cluster),
                        Some(db),
                        Some(schema),
                        Some(table),
                        Some(structure),
                    ) = (
                        app.selected_cluster().map(|c| c.name.clone()),
                        app.table_browser.selected_database.clone(),
                        app.table_browser.selected_schema.clone(),
                        app.table_browser.selected_table.clone(),
                        app.table_browser.table_structure.clone(),
                    ) {
                        if let Some(pk_col) = structure.primary_key_columns.first() {
                            app.delete_table_row(
                                &cluster, &db, &schema, &table, pk_col, &pk_value,
                            );
                            app.table_browser.show_delete_dialog = false;
                            app.table_browser.pending_delete_pk = None;
                            // Refresh data
                            app.refresh_table_data(&cluster);
                        }
                    }
                }
                if ui.button("Cancel").clicked() {
                    app.table_browser.show_delete_dialog = false;
                    app.table_browser.pending_delete_pk = None;
                }
            });
        });
}
