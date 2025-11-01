pub struct UICompositor {}

impl UICompositor {
    pub(crate) fn new_context() -> egui::Context {
        let egui = egui::Context::default();
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = egui::Color32::from_rgba_unmultiplied(
            visuals.window_fill.r(),
            visuals.window_fill.g(),
            visuals.window_fill.b(),
            200,
        );

        egui.set_visuals(visuals);
        egui.set_zoom_factor(1.1);
        egui
    }

    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) fn render(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Window::new("Tracer Controls")
                .resizable(true)
                .default_width(300.0)
                .show(ui.ctx(), |ui| {
                    ui.label("Tracer Controls go here");
                });
        });
    }
}
