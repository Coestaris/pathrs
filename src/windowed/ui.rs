use gpu_allocator::vulkan::AllocatorVisualizer;
use gpu_allocator::AllocatorReport;

pub struct UICompositor {
    pub egui: egui_winit::State,
    pub allocator_visualizer: AllocatorVisualizer,
    pub allocator_breakdown_open: bool,
    pub tracer_controls_open: bool,
    pub fps: f32,
}

impl UICompositor {
    pub(crate) fn new_context() -> egui::Context {
        let egui = egui::Context::default();
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::TRANSPARENT;
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

    pub(crate) fn new(egui: egui_winit::State) -> Self {
        Self {
            egui,
            allocator_visualizer: AllocatorVisualizer::new(),
            allocator_breakdown_open: true,
            tracer_controls_open: true,
            fps: 0.0,
        }
    }

    pub fn set_fps(&mut self, fps: f32) {
        self.fps = fps;
    }

    pub(crate) fn render(
        &mut self,
        ctx: &egui::Context,
        allocator: &mut gpu_allocator::vulkan::Allocator,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Window::new("Tracer Controls")
                .resizable(true)
                .default_width(300.0)
                .open(&mut self.tracer_controls_open)
                .show(ui.ctx(), |ui| {
                    ui.label(format!("FPS: {:.2}", self.fps));
                    ui.label("Tracer Controls go here");
                });

            self.allocator_visualizer.render_breakdown_window(
                ctx,
                allocator,
                &mut self.allocator_breakdown_open,
            );
        });
    }
}
