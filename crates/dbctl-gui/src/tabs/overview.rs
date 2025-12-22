use eframe::egui;

use crate::state::ClusterInfo;
use crate::DbctlApp;

pub fn render_overview_tab(app: &mut DbctlApp, ui: &mut egui::Ui, cluster: &ClusterInfo) {
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
                ui.ctx().copy_text(conn_str.clone());
                app.status_message = "Connection string copied".to_string();
            }
        });
    }
}
