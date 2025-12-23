use eframe::egui;

use crate::ai_chat::{AiConfig, AVAILABLE_MODELS};
use crate::state::ConnectionInfo;
use crate::DbctlApp;

pub fn render_install_dialog(app: &mut DbctlApp, ctx: &egui::Context) {
    if !app.show_install_dialog {
        return;
    }

    egui::Window::new("Install PostgreSQL")
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Version:");
                ui.text_edit_singleline(&mut app.install_version);
            });

            ui.label("Examples: 17, 16, 15.8, 14.12");

            ui.separator();

            let version = app.install_version.clone();
            ui.horizontal(|ui| {
                if ui.button("Install").clicked() {
                    app.install_postgres(&version);
                    app.show_install_dialog = false;
                    app.status_message = format!("Installing PostgreSQL {}...", version);
                }
                if ui.button("Cancel").clicked() {
                    app.show_install_dialog = false;
                }
            });
        });
}

pub fn render_create_cluster_dialog(app: &mut DbctlApp, ctx: &egui::Context) {
    if !app.show_create_cluster_dialog {
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
                    ui.text_edit_singleline(&mut app.new_cluster_name);
                    ui.end_row();

                    ui.label("Version:");
                    egui::ComboBox::from_id_salt("version_select")
                        .selected_text(&app.new_cluster_version)
                        .show_ui(ui, |ui| {
                            for v in &app.installed_versions {
                                ui.selectable_value(&mut app.new_cluster_version, v.clone(), v);
                            }
                        });
                    ui.end_row();

                    ui.label("Port:");
                    ui.text_edit_singleline(&mut app.new_cluster_port);
                    ui.end_row();

                    ui.label("Superuser:");
                    ui.text_edit_singleline(&mut app.new_cluster_username);
                    ui.end_row();
                });

            if app.installed_versions.is_empty() {
                ui.colored_label(app.theme.warning, "No PostgreSQL versions installed!");
            }

            ui.separator();

            ui.horizontal(|ui| {
                let can_create = !app.new_cluster_name.is_empty()
                    && !app.new_cluster_version.is_empty()
                    && !app.new_cluster_port.is_empty();

                if ui
                    .add_enabled(can_create, egui::Button::new("Create"))
                    .clicked()
                {
                    app.create_cluster();
                }
                if ui.button("Cancel").clicked() {
                    app.show_create_cluster_dialog = false;
                }
            });
        });
}

pub fn render_remove_cluster_dialog(app: &mut DbctlApp, ctx: &egui::Context) {
    if !app.show_remove_cluster_dialog {
        return;
    }

    let cluster_name = app
        .selected_cluster()
        .map(|c| c.name.clone())
        .unwrap_or_default();

    egui::Window::new("Confirm Remove")
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(format!(
                "Are you sure you want to remove cluster '{}'?",
                cluster_name
            ));
            ui.colored_label(app.theme.warning, "This will delete all data!");

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Remove").clicked() {
                    app.remove_cluster(&cluster_name);
                    app.show_remove_cluster_dialog = false;
                }
                if ui.button("Cancel").clicked() {
                    app.show_remove_cluster_dialog = false;
                }
            });
        });
}

pub fn render_drop_db_dialog(app: &mut DbctlApp, ctx: &egui::Context) {
    if !app.show_drop_db_dialog {
        return;
    }

    let db_name = app.pending_drop_db.clone().unwrap_or_default();
    let cluster_name = app
        .selected_cluster()
        .map(|c| c.name.clone())
        .unwrap_or_default();

    egui::Window::new("Confirm Drop Database")
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(format!(
                "Are you sure you want to drop database '{}'?",
                db_name
            ));
            ui.colored_label(app.theme.warning, "This cannot be undone!");

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Drop").clicked() {
                    app.drop_database(&cluster_name, &db_name);
                    app.show_drop_db_dialog = false;
                    app.pending_drop_db = None;
                }
                if ui.button("Cancel").clicked() {
                    app.show_drop_db_dialog = false;
                    app.pending_drop_db = None;
                }
            });
        });
}

pub fn render_add_connection_dialog(app: &mut DbctlApp, ctx: &egui::Context) {
    if !app.show_add_connection_dialog {
        return;
    }

    egui::Window::new("Add Connection")
        .collapsible(false)
        .resizable(false)
        .default_width(350.0)
        .show(ctx, |ui| {
            render_connection_form(app, ui);

            ui.separator();

            // Test connection result
            if let Some(ref result) = app.test_connection_result {
                match result {
                    Ok(version) => {
                        ui.colored_label(
                            app.theme.success,
                            format!("Connected: {}", version),
                        );
                    }
                    Err(e) => {
                        ui.colored_label(app.theme.warning, format!("Failed: {}", e));
                    }
                }
                ui.separator();
            }

            ui.horizontal(|ui| {
                // Test button
                let can_test = !app.conn_form_host.is_empty() && !app.testing_connection;

                if ui
                    .add_enabled(can_test, egui::Button::new("Test"))
                    .clicked()
                {
                    if let Ok(profile) = app.build_connection_from_form() {
                        app.test_connection(profile);
                    }
                }

                if app.testing_connection {
                    ui.spinner();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        app.show_add_connection_dialog = false;
                        app.reset_connection_form();
                    }

                    let can_save =
                        !app.conn_form_name.is_empty() && !app.conn_form_host.is_empty();

                    if ui
                        .add_enabled(can_save, egui::Button::new("Save"))
                        .clicked()
                    {
                        match app.build_connection_from_form() {
                            Ok(profile) => {
                                // Check for duplicate name
                                if app
                                    .connections
                                    .iter()
                                    .any(|c| c.profile.name == profile.name)
                                {
                                    app.status_message = format!(
                                        "Connection '{}' already exists",
                                        profile.name
                                    );
                                } else {
                                    app.connections.push(ConnectionInfo {
                                        profile,
                                        connected: false,
                                        databases: Vec::new(),
                                    });
                                    app.save_connections();
                                    app.show_add_connection_dialog = false;
                                    app.reset_connection_form();
                                    app.status_message = "Connection saved".to_string();
                                }
                            }
                            Err(e) => {
                                app.status_message = e;
                            }
                        }
                    }
                });
            });
        });
}

pub fn render_edit_connection_dialog(app: &mut DbctlApp, ctx: &egui::Context) {
    if !app.show_edit_connection_dialog {
        return;
    }

    egui::Window::new("Edit Connection")
        .collapsible(false)
        .resizable(false)
        .default_width(350.0)
        .show(ctx, |ui| {
            render_connection_form(app, ui);

            ui.separator();

            // Test connection result
            if let Some(ref result) = app.test_connection_result {
                match result {
                    Ok(version) => {
                        ui.colored_label(
                            app.theme.success,
                            format!("Connected: {}", version),
                        );
                    }
                    Err(e) => {
                        ui.colored_label(app.theme.warning, format!("Failed: {}", e));
                    }
                }
                ui.separator();
            }

            ui.horizontal(|ui| {
                // Test button
                let can_test = !app.conn_form_host.is_empty() && !app.testing_connection;

                if ui
                    .add_enabled(can_test, egui::Button::new("Test"))
                    .clicked()
                {
                    if let Ok(profile) = app.build_connection_from_form() {
                        app.test_connection(profile);
                    }
                }

                if app.testing_connection {
                    ui.spinner();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        app.show_edit_connection_dialog = false;
                        app.editing_connection_index = None;
                        app.reset_connection_form();
                    }

                    let can_save =
                        !app.conn_form_name.is_empty() && !app.conn_form_host.is_empty();

                    if ui
                        .add_enabled(can_save, egui::Button::new("Save"))
                        .clicked()
                    {
                        match app.build_connection_from_form() {
                            Ok(profile) => {
                                if let Some(idx) = app.editing_connection_index {
                                    // Check for duplicate name (excluding current)
                                    let duplicate = app
                                        .connections
                                        .iter()
                                        .enumerate()
                                        .any(|(i, c)| i != idx && c.profile.name == profile.name);

                                    if duplicate {
                                        app.status_message = format!(
                                            "Connection '{}' already exists",
                                            profile.name
                                        );
                                    } else {
                                        if let Some(conn) = app.connections.get_mut(idx) {
                                            conn.profile = profile;
                                            conn.connected = false;
                                            conn.databases.clear();
                                        }
                                        app.save_connections();
                                        app.show_edit_connection_dialog = false;
                                        app.editing_connection_index = None;
                                        app.reset_connection_form();
                                        app.status_message = "Connection updated".to_string();
                                    }
                                }
                            }
                            Err(e) => {
                                app.status_message = e;
                            }
                        }
                    }
                });
            });
        });
}

fn render_connection_form(app: &mut DbctlApp, ui: &mut egui::Ui) {
    egui::Grid::new("connection_form_grid")
        .num_columns(2)
        .spacing([10.0, 5.0])
        .show(ui, |ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut app.conn_form_name);
            ui.end_row();

            ui.label("Host:");
            ui.text_edit_singleline(&mut app.conn_form_host);
            ui.end_row();

            ui.label("Port:");
            ui.text_edit_singleline(&mut app.conn_form_port);
            ui.end_row();

            ui.label("Database:");
            ui.text_edit_singleline(&mut app.conn_form_database);
            ui.end_row();

            ui.label("Username:");
            ui.text_edit_singleline(&mut app.conn_form_username);
            ui.end_row();

            ui.label("Password:");
            ui.add(egui::TextEdit::singleline(&mut app.conn_form_password).password(true));
            ui.end_row();

            ui.label("SSL Mode:");
            egui::ComboBox::from_id_salt("ssl_mode_select")
                .selected_text(app.conn_form_ssl_mode.as_str())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut app.conn_form_ssl_mode,
                        dbctl::SslMode::Disable,
                        "disable",
                    );
                    ui.selectable_value(
                        &mut app.conn_form_ssl_mode,
                        dbctl::SslMode::Require,
                        "require",
                    );
                });
            ui.end_row();
        });
}

pub fn render_api_key_dialog(app: &mut DbctlApp, ctx: &egui::Context) {
    if !app.show_api_key_dialog {
        return;
    }

    egui::Window::new("AI Schema Assistant Settings")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(420.0)
        .show(ctx, |ui| {
            ui.label("Configure the AI Schema Assistant for natural language schema design.");
            ui.add_space(5.0);
            ui.label(
                egui::RichText::new("Settings are saved to ~/.dbctl/ai-config.json")
                    .size(11.0)
                    .color(app.theme.text_muted),
            );

            ui.add_space(15.0);

            // API Key section
            ui.heading("API Key");
            ui.add_space(5.0);

            let has_key = !app.ai_config.api_key.is_empty();
            ui.horizontal(|ui| {
                ui.label("Status:");
                if has_key {
                    ui.label(egui::RichText::new("Configured").color(app.theme.success));
                } else {
                    ui.label(egui::RichText::new("Not configured").color(app.theme.warning));
                }
            });

            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("API Key:");
                ui.add(
                    egui::TextEdit::singleline(&mut app.api_key_input)
                        .password(true)
                        .desired_width(280.0)
                        .hint_text(if has_key { "(unchanged)" } else { "sk-ant-..." }),
                );
            });

            ui.add_space(10.0);

            ui.hyperlink_to(
                "Get an API key from Anthropic Console",
                "https://console.anthropic.com/settings/keys",
            );

            ui.add_space(15.0);
            ui.separator();
            ui.add_space(10.0);

            // Model section
            ui.heading("Model");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Model:");
                egui::ComboBox::from_id_salt("ai_model_select")
                    .selected_text(app.ai_config.model_display_name())
                    .width(280.0)
                    .show_ui(ui, |ui| {
                        for (model_id, model_name) in AVAILABLE_MODELS {
                            ui.selectable_value(
                                &mut app.ai_config.model,
                                model_id.to_string(),
                                *model_name,
                            );
                        }
                    });
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Max Tokens:");
                ui.add(
                    egui::DragValue::new(&mut app.ai_config.max_tokens)
                        .range(256..=16384)
                        .speed(100),
                );
                ui.label(
                    egui::RichText::new("(response length limit)")
                        .size(11.0)
                        .color(app.theme.text_muted),
                );
            });

            ui.add_space(20.0);
            ui.separator();
            ui.add_space(10.0);

            // Action buttons
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    // Reload config to discard changes
                    app.ai_config = AiConfig::load();
                    app.show_api_key_dialog = false;
                    app.api_key_input.clear();
                }

                // Reset to defaults button
                if ui.button("Reset to Defaults").clicked() {
                    app.ai_config = AiConfig::default();
                    app.api_key_input.clear();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Only require valid key format if user is entering a new key
                    let new_key_valid = app.api_key_input.is_empty()
                        || app.api_key_input.starts_with("sk-ant-")
                        || app.api_key_input.starts_with("sk-");

                    if ui
                        .add_enabled(new_key_valid, egui::Button::new("Save"))
                        .clicked()
                    {
                        // Update API key if user entered a new one
                        if !app.api_key_input.is_empty() {
                            app.ai_config.api_key = app.api_key_input.clone();
                        }

                        match app.ai_config.save() {
                            Ok(()) => {
                                app.status_message = "AI settings saved".to_string();
                                app.show_api_key_dialog = false;
                                app.api_key_input.clear();
                            }
                            Err(e) => {
                                app.status_message = format!("Failed to save settings: {}", e);
                            }
                        }
                    }

                    // Remove key button (only if key exists)
                    if has_key {
                        if ui.button("Remove Key").clicked() {
                            app.ai_config.api_key.clear();
                            app.api_key_input.clear();
                            match app.ai_config.save() {
                                Ok(()) => {
                                    app.status_message = "API key removed".to_string();
                                }
                                Err(e) => {
                                    app.status_message = format!("Failed to save: {}", e);
                                }
                            }
                        }
                    }
                });
            });
        });
}
