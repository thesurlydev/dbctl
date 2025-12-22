use eframe::egui;

use crate::state::ClusterInfo;
use crate::DbctlApp;

pub fn render_users_tab(app: &mut DbctlApp, ui: &mut egui::Ui, cluster: &ClusterInfo) {
    let cluster_name = cluster.name.clone();
    let mut should_create = false;
    let mut should_refresh = false;

    ui.horizontal(|ui| {
        ui.label("Username:");
        ui.text_edit_singleline(&mut app.new_user_name);
        ui.label("Password:");
        ui.add(egui::TextEdit::singleline(&mut app.new_user_password).password(true));
        ui.checkbox(&mut app.new_user_superuser, "Superuser");
        if ui.button("Create").clicked() && !app.new_user_name.is_empty() {
            should_create = true;
        }

        ui.separator();

        if ui.button("Refresh").clicked() {
            should_refresh = true;
        }
    });

    if should_create {
        let username = app.new_user_name.clone();
        let pwd = if app.new_user_password.is_empty() {
            None
        } else {
            Some(app.new_user_password.clone())
        };
        let superuser = app.new_user_superuser;
        app.create_user(&cluster_name, &username, pwd.as_deref(), superuser);
        app.new_user_name.clear();
        app.new_user_password.clear();
        app.new_user_superuser = false;
    }

    if should_refresh {
        app.fetch_users(&cluster_name);
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

            for user in &app.users {
                ui.label(&user.name);
                ui.label(if user.superuser { "Yes" } else { "No" });
                ui.label(if user.create_db { "Yes" } else { "No" });
                ui.label(if user.create_role { "Yes" } else { "No" });
                ui.end_row();
            }
        });
}
