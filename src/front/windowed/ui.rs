use crate::config::TracerConfig;
use crate::front::windowed::free_cam::FreeCamera;
use crate::tracer::{Bundle, TracerProfile};
use egui::Widget;
use gpu_allocator::vulkan::AllocatorVisualizer;
use log::info;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::keyboard::{Key, NamedKey};

pub struct UICompositor {
    config: TracerConfig,
    free_camera: FreeCamera,
    visible: bool,

    pub egui: egui_winit::State,
    pub allocator_visualizer: AllocatorVisualizer,
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
        let initial_camera = config.0.borrow().camera.clone();
        Self {
            egui,
            allocator_visualizer: AllocatorVisualizer::new(),
            config,
            fps: 0.0,
            tracer_profile: None,
            visible: true,
            free_camera: FreeCamera::new(initial_camera),
        }
    }

    pub fn set_fps(&mut self, fps: f32) {
        self.fps = fps;
    }

    pub fn set_tracer_profile(&mut self, profile: TracerProfile) {
        self.tracer_profile = Some(profile);
    }

    pub fn on_window_event(&mut self, event: &WindowEvent) {
        self.free_camera.on_window_event(event);
        match event {
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    logical_key, state, ..
                },
                ..
            } => match (logical_key, state) {
                (Key::Named(NamedKey::F1), ElementState::Released) => {
                    info!("Toggling UI visibility");
                    self.visible = !self.visible;
                }
                _ => {}
            },

            _ => {}
        }
    }

    pub(crate) fn render(&mut self, bundle: Bundle, ctx: &egui::Context) {
        let mut changed = false;
        let cfg = &mut self.config.0.borrow_mut();

        if let Some(camera_data) = self.free_camera.tick_handler() {
            cfg.camera.position = camera_data.position;
            cfg.camera.direction = camera_data.as_direction();
            cfg.updated = true;
        }

        if !self.visible {
            return;
        }

        egui::SidePanel::left("side_panel")
            .resizable(true)
            .show(ctx, |ui| {
                ui.label(format!("FPS: {:.2}", self.fps));
                if let Some(profile) = &self.tracer_profile {
                    ui.label(format!("Traces per sec: {:.2}", profile.fps.fps()));
                    ui.label(format!("Render time: {:.2}", profile.render_time));
                }

                ui.separator();
                ui.label("Press F1 to toggle UI visibility");
                ui.label("Use WASD + Space/Shift to move camera");
                ui.separator();

                ui.collapsing("Tracer Controls", |ui| {
                    const PI: f32 = std::f32::consts::PI;
                    float_slider!(&mut cfg.camera.fov, 0.0..=PI, "FOV", ui, changed);
                    float_slider!(&mut cfg.samples_count, 1..=64, "Samples Count", ui, changed);
                    float_slider!(
                        &mut cfg.jitter_strength,
                        0.0..=1.0,
                        "Jitter Strength",
                        ui,
                        changed
                    );
                    float_slider!(
                        &mut cfg.temporal_accumulation,
                        0.0..=1.0,
                        "Temporal Accumulation",
                        ui,
                        changed
                    );
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
