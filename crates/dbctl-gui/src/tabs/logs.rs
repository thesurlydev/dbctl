use eframe::egui;

use crate::state::ClusterInfo;
use crate::DbctlApp;

pub fn render_logs_tab(app: &mut DbctlApp, ui: &mut egui::Ui, cluster: &ClusterInfo) {
    ui.horizontal(|ui| {
        ui.label("Lines:");
        ui.add(egui::DragValue::new(&mut app.logs_lines).range(10..=1000));
        if ui.button("Refresh").clicked() {
            app.refresh_logs(&cluster.name);
        }
    });

    ui.separator();

    egui::ScrollArea::vertical()
        .id_salt("logs_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut app.logs_content.as_str())
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY),
            );
        });
}
