use crate::tracer::TracerProfile;
use gpu_allocator::vulkan::AllocatorVisualizer;

pub struct UICompositor {
    pub egui: egui_winit::State,
    pub allocator_visualizer: AllocatorVisualizer,

    pub fps: f32,
    pub tracer_profile: Option<TracerProfile>,
}

impl UICompositor {
    pub(crate) fn new_context() -> egui::Context {
        let egui = egui::Context::default();
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = egui::Color32::from_rgba_unmultiplied(
            visuals.window_fill.r(),
            visuals.window_fill.g(),
            visuals.window_fill.b(),
            255,
        );
        visuals.panel_fill = visuals.window_fill;

        egui.set_visuals(visuals);
        egui.set_zoom_factor(1.1);
        egui
    }

    pub(crate) fn new(egui: egui_winit::State) -> Self {
        Self {
            egui,
            allocator_visualizer: AllocatorVisualizer::new(),
            fps: 0.0,
            tracer_profile: None,
        }
    }

    pub fn set_fps(&mut self, fps: f32) {
        self.fps = fps;
    }

    pub fn set_tracer_profile(&mut self, profile: TracerProfile) {
        self.tracer_profile = Some(profile);
    }

    pub(crate) fn render(
        &mut self,
        ctx: &egui::Context,
        allocator: &mut gpu_allocator::vulkan::Allocator,
    ) {
        egui::SidePanel::left("side_panel")
            .resizable(true)
            .show(ctx, |ui| {
                ui.label(format!("FPS: {:.2}", self.fps));
                if let Some(profile) = &self.tracer_profile {
                    ui.label(format!("Traces per sec: {:.2}", profile.fps.fps()));
                    ui.label(format!("Render time: {:.2}", profile.render_time));
                }

                ui.collapsing("Tracer Controls", |ui| {
                    ui.label("Tracer Controls go here");
                });

                ui.collapsing("Allocator Breakdown", |ui| {
                    self.allocator_visualizer.render_breakdown_ui(ui, allocator);
                });
            });
    }
}
