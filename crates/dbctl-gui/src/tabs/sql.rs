use eframe::egui;

use crate::state::ClusterInfo;
use crate::syntax::highlight_sql;
use crate::DbctlApp;

pub fn render_sql_tab(app: &mut DbctlApp, ui: &mut egui::Ui, cluster: &ClusterInfo) {
    let cluster_name = cluster.name.clone();

    // Keyboard shortcuts
    let execute_shortcut = ui.ctx().input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.command);
    let save_shortcut = ui.ctx().input(|i| i.key_pressed(egui::Key::S) && i.modifiers.command);
    let history_up = ui.ctx().input(|i| i.key_pressed(egui::Key::ArrowUp) && i.modifiers.command);
    let history_down =
        ui.ctx().input(|i| i.key_pressed(egui::Key::ArrowDown) && i.modifiers.command);

    // Tab bar
    ui.horizontal(|ui| {
        let tabs_len = app.sql_state.tabs.len();
        let mut tab_to_close: Option<usize> = None;

        for (idx, tab) in app.sql_state.tabs.iter().enumerate() {
            let is_active = idx == app.sql_state.active_tab;
            let tab_text = if tab.is_executing {
                format!("{} ...", tab.name)
            } else {
                tab.name.clone()
            };

            ui.horizontal(|ui| {
                if ui.selectable_label(is_active, &tab_text).clicked() {
                    app.sql_state.active_tab = idx;
                }
                // Close button (only show if more than 1 tab)
                if tabs_len > 1 && ui.small_button("x").clicked() {
                    tab_to_close = Some(idx);
                }
            });
        }

        // Add new tab button
        if app.sql_state.tabs.len() < 10 && ui.button("+").clicked() {
            let db = app.sql_state.current_tab().database.clone();
            app.sql_state.add_tab(db);
        }

        // Close tab if requested
        if let Some(idx) = tab_to_close {
            app.sql_state.close_tab(idx);
        }
    });

    ui.separator();

    // Get current tab info (need to work around borrow checker)
    let active_idx = app.sql_state.active_tab;
    let current_db = app.sql_state.tabs[active_idx].database.clone();
    let is_executing = app.sql_state.tabs[active_idx].is_executing;

    // Controls row
    ui.horizontal(|ui| {
        ui.label("Database:");
        egui::ComboBox::from_id_salt("sql_db")
            .selected_text(&current_db)
            .show_ui(ui, |ui| {
                for db in &app.databases.clone() {
                    if ui.selectable_label(&current_db == db, db).clicked() {
                        app.sql_state.tabs[active_idx].database = db.clone();
                    }
                }
            });

        ui.separator();

        // Load saved query dropdown
        if !app.sql_state.saved_queries.is_empty() {
            egui::ComboBox::from_id_salt("load_query")
                .selected_text("Load...")
                .show_ui(ui, |ui| {
                    let queries = app.sql_state.saved_queries.clone();
                    for sq in &queries {
                        if ui.selectable_label(false, &sq.name).clicked() {
                            app.sql_state.tabs[active_idx].query = sq.query.clone();
                        }
                    }
                });
        }

        if ui.button("Save").clicked() {
            app.sql_state.show_save_dialog = true;
        }

        ui.separator();

        let execute_enabled =
            !app.sql_state.tabs[active_idx].query.trim().is_empty() && !is_executing;

        if ui
            .add_enabled(execute_enabled, egui::Button::new("Execute"))
            .clicked()
        {
            app.execute_sql(&cluster_name);
        }

        if ui
            .add_enabled(execute_enabled, egui::Button::new("Explain"))
            .clicked()
        {
            app.execute_explain(&cluster_name, false);
        }

        if ui
            .add_enabled(execute_enabled, egui::Button::new("Analyze"))
            .clicked()
        {
            app.execute_explain(&cluster_name, true);
        }

        if ui.button("Clear").clicked() {
            app.sql_state.tabs[active_idx].query.clear();
            app.sql_state.tabs[active_idx].result.clear();
        }

        if is_executing {
            ui.spinner();
        }
    });

    ui.separator();

    // Query input
    ui.horizontal(|ui| {
        ui.label("SQL Query:");
        if ui.small_button("History").clicked() {
            app.sql_state.show_history_popup = !app.sql_state.show_history_popup;
        }
    });

    // Query text area with syntax highlighting
    {
        let font_id = egui::FontId::monospace(14.0);
        let theme = &app.theme;
        let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
            let mut job = highlight_sql(text.as_str(), font_id.clone(), theme);
            job.wrap.max_width = wrap_width;
            ui.fonts_mut(|f| f.layout_job(job))
        };

        let tab = &mut app.sql_state.tabs[active_idx];
        ui.add(
            egui::TextEdit::multiline(&mut tab.query)
                .font(egui::FontId::monospace(14.0))
                .desired_width(f32::INFINITY)
                .desired_rows(8)
                .layouter(&mut layouter),
        );
    }

    // Handle keyboard shortcuts after the text edit
    if execute_shortcut
        && !app.sql_state.tabs[active_idx].query.trim().is_empty()
        && !is_executing
    {
        app.execute_sql(&cluster_name);
    }
    if save_shortcut {
        app.sql_state.show_save_dialog = true;
    }
    if history_up {
        navigate_history(app, -1);
    }
    if history_down {
        navigate_history(app, 1);
    }

    // History popup
    if app.sql_state.show_history_popup && !app.sql_state.history.is_empty() {
        egui::Window::new("Query History")
            .collapsible(false)
            .resizable(true)
            .default_width(400.0)
            .default_height(300.0)
            .show(ui.ctx(), |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("history_scroll")
                    .max_height(250.0)
                    .show(ui, |ui| {
                        let history = app.sql_state.history.clone();
                        for entry in history.iter() {
                            ui.horizontal(|ui| {
                                let preview: String = entry.query.chars().take(60).collect();
                                let preview = if entry.query.len() > 60 {
                                    format!("{}...", preview)
                                } else {
                                    preview
                                };
                                if ui.button(&preview).clicked() {
                                    app.sql_state.tabs[active_idx].query = entry.query.clone();
                                    app.sql_state.tabs[active_idx].database =
                                        entry.database.clone();
                                    app.sql_state.show_history_popup = false;
                                }
                                ui.label(format!("[{}]", entry.database));
                            });
                        }
                    });

                if ui.button("Close").clicked() {
                    app.sql_state.show_history_popup = false;
                }
            });
    }

    // Save query dialog
    if app.sql_state.show_save_dialog {
        egui::Window::new("Save Query")
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut app.sql_state.save_query_name);
                });

                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        let name = app.sql_state.save_query_name.clone();
                        let query = app.sql_state.tabs[active_idx].query.clone();
                        if !name.is_empty() && !query.trim().is_empty() {
                            match app.sql_state.save_query(&name, &query) {
                                Ok(()) => {
                                    app.status_message = format!("Saved query '{}'", name);
                                }
                                Err(e) => {
                                    app.status_message = e;
                                }
                            }
                        }
                        app.sql_state.show_save_dialog = false;
                        app.sql_state.save_query_name.clear();
                    }
                    if ui.button("Cancel").clicked() {
                        app.sql_state.show_save_dialog = false;
                        app.sql_state.save_query_name.clear();
                    }
                });
            });
    }

    ui.separator();

    // Result display
    ui.label("Result:");
    egui::ScrollArea::vertical()
        .id_salt("sql_result_scroll")
        .max_height(200.0)
        .show(ui, |ui| {
            let result = &app.sql_state.tabs[active_idx].result;
            ui.add(
                egui::TextEdit::multiline(&mut result.as_str())
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY),
            );
        });

    // Shortcuts help
    ui.separator();
    ui.horizontal(|ui| {
        ui.small("Cmd+Enter: Execute | Cmd+S: Save | Cmd+Up/Down: History");
    });
}

pub fn navigate_history(app: &mut DbctlApp, direction: i32) {
    if app.sql_state.history.is_empty() {
        return;
    }

    let active_idx = app.sql_state.active_tab;
    let new_index = match app.sql_state.history_index {
        None => {
            if direction < 0 {
                Some(0)
            } else {
                None
            }
        }
        Some(idx) => {
            let new_idx = idx as i32 + direction;
            if new_idx < 0 {
                None
            } else if new_idx >= app.sql_state.history.len() as i32 {
                Some(app.sql_state.history.len() - 1)
            } else {
                Some(new_idx as usize)
            }
        }
    };

    app.sql_state.history_index = new_index;

    if let Some(idx) = new_index {
        if let Some(entry) = app.sql_state.history.get(idx) {
            app.sql_state.tabs[active_idx].query = entry.query.clone();
        }
    }
}
