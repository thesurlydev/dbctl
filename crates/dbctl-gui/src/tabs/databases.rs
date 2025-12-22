use eframe::egui;

use crate::state::ClusterInfo;
use crate::DbctlApp;

pub fn render_databases_tab(app: &mut DbctlApp, ui: &mut egui::Ui, cluster: &ClusterInfo) {
    let cluster_name = cluster.name.clone();

    ui.horizontal(|ui| {
        ui.label("New database:");
        ui.text_edit_singleline(&mut app.new_db_name);
        if ui.button("Create").clicked() && !app.new_db_name.is_empty() {
            let db = app.new_db_name.clone();
            app.create_database(&cluster_name, &db);
            app.new_db_name.clear();
        }

        ui.separator();

        if ui.button("Refresh").clicked() {
            app.fetch_databases(&cluster_name);
        }
    });

    ui.separator();

    let mut action = None;

    egui::ScrollArea::vertical()
        .id_salt("databases_list_scroll")
        .show(ui, |ui| {
            for (i, db) in app.databases.iter().enumerate() {
                ui.horizontal(|ui| {
                    let selected = app.selected_database == Some(i);
                    if ui.selectable_label(selected, db).clicked() {
                        app.selected_database = Some(i);
                        // Also update current SQL tab database
                        app.sql_state.current_tab_mut().database = db.clone();
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
            app.pending_drop_db = Some(db);
            app.show_drop_db_dialog = true;
        }
    }

    // Backup/Restore section
    if let Some(idx) = app.selected_database {
        if let Some(db) = app.databases.get(idx).cloned() {
            ui.separator();
            ui.heading("Backup / Restore");

            let cluster_name = cluster.name.clone();
            let db_clone = db.clone();

            ui.horizontal(|ui| {
                ui.label("Backup to:");
                ui.text_edit_singleline(&mut app.backup_path);
            });
            if ui.button("Backup").clicked() && !app.backup_path.is_empty() {
                app.backup_database(&cluster_name, &db_clone);
            }

            ui.horizontal(|ui| {
                ui.label("Restore from:");
                ui.text_edit_singleline(&mut app.restore_path);
            });
            if ui.button("Restore").clicked() && !app.restore_path.is_empty() {
                app.restore_database(&cluster_name, &db);
            }
        }
    }
}
