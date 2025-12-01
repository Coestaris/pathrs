use crate::config::TracerConfig;
use crate::tracer::{Bundle, TracerProfile};
use egui::Widget;
use gpu_allocator::vulkan::AllocatorVisualizer;

pub struct UICompositor {
    pub egui: egui_winit::State,
    pub allocator_visualizer: AllocatorVisualizer,
    config: TracerConfig,
    pub fps: f32,
    pub tracer_profile: Option<TracerProfile>,
}

macro_rules! float_slider {
    ($val:expr, $range:expr, $text:expr, $ui:expr, $changed:expr) => {
        if egui::Slider::new($val, $range)
            .text($text)
            .step_by(0.01)
            .ui($ui)
            .changed()
        {
            $changed = true;
        }
    };
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

    pub(crate) fn new(egui: egui_winit::State, config: TracerConfig) -> Self {
        Self {
            egui,
            allocator_visualizer: AllocatorVisualizer::new(),
            config,
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

    pub(crate) fn render(&mut self, bundle: Bundle, ctx: &egui::Context) {
        let mut changed = false;
        let cfg = &mut self.config.0.borrow_mut();

        egui::SidePanel::left("side_panel")
            .resizable(true)
            .show(ctx, |ui| {
                ui.label(format!("FPS: {:.2}", self.fps));
                if let Some(profile) = &self.tracer_profile {
                    ui.label(format!("Traces per sec: {:.2}", profile.fps.fps()));
                    ui.label(format!("Render time: {:.2}", profile.render_time));
                }

                ui.collapsing("Tracer Controls", |ui| {
                    const PI: f32 = std::f32::consts::PI;
                    float_slider!(&mut cfg.camera.fov, 0.0..=PI, "FOV", ui, changed);
                    float_slider!(&mut cfg.camera.position.x, -4.0..=4.0, "X", ui, changed);
                    float_slider!(&mut cfg.camera.position.y, -4.0..=4.0, "Y", ui, changed);
                    float_slider!(&mut cfg.camera.position.z, -4.0..=4.0, "Z", ui, changed);
                    float_slider!(&mut cfg.camera.direction.x, -PI..=PI, "Dir X", ui, changed);
                    float_slider!(&mut cfg.camera.direction.y, -PI..=PI, "Dir Y", ui, changed);
                    float_slider!(&mut cfg.camera.direction.z, -PI..=PI, "Dir Z", ui, changed);
                });

                ui.collapsing("Allocator Breakdown", |ui| {
                    self.allocator_visualizer
                        .render_breakdown_ui(ui, &bundle.allocator());
                });
            });

        if changed {
            cfg.updated = true;
        }
    }
}
