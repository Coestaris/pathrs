use crate::tracer::config::TracerConfig;
use crate::tracer::front::windowed::TracerWindowedFront;
use crate::tracer::Tracer;
use build_info::BuildInfo;
use glam::UVec2;
use log::info;
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalSize, Size};
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::platform::x11::WindowAttributesExtX11;
use winit::window::{Window, WindowAttributes, WindowId};

struct Context {
    window: Window,
    tracer: Tracer<TracerWindowedFront>,
}

pub struct App {
    build_info: BuildInfo,
    viewport: UVec2,
    config: TracerConfig,
    context: Option<Context>,
}

impl App {
    pub fn new(config: TracerConfig, initial_viewport: UVec2, bi: BuildInfo) -> Self {
        Self {
            viewport: initial_viewport,
            build_info: bi,
            context: None,
            config,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let title = format!(
            "{} (v{})",
            self.build_info.crate_info.name,
            self.build_info.crate_info.version.to_string()
        );
        let size = Size::Physical(PhysicalSize::new(self.viewport.x, self.viewport.y));
        let attributes = WindowAttributes::default()
            .with_title(title)
            .with_base_size(size);

        #[cfg(target_os = "linux")]
        // Make my I3 happy
        let attributes = attributes.with_x11_window_type(vec![
            winit::platform::x11::WindowType::Normal,
            winit::platform::x11::WindowType::Dialog,
        ]);

        unsafe {
            let window = event_loop.create_window(attributes).unwrap();
            let tracer = Tracer::<TracerWindowedFront>::new_windowed(
                self.config.clone(),
                self.viewport,
                self.build_info.clone(),
                &window,
            )
            .unwrap();
            self.context = Some(Context { window, tracer });
        }

        info!("Initialized windowed tracer");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let context = self.context.as_mut().unwrap();
        match event {
            WindowEvent::Resized(physical_size) => unsafe {
                info!("Window resized to {:?}", physical_size);
                self.viewport = UVec2::new(physical_size.width, physical_size.height);
                context.tracer.resize(self.viewport).unwrap();
            },
            WindowEvent::RedrawRequested => unsafe {
                context.tracer.trace().unwrap();
            },
            WindowEvent::CloseRequested => {
                info!("Close requested, exiting event loop");
                event_loop.exit();
            }
            // Close on escape
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::Escape),
                        ..
                    },
                ..
            } => {
                info!("Escape pressed, exiting event loop");
                event_loop.exit();
            }

            _ => {
                // Redraw on any other event
                context.window.request_redraw();
            }
        }
    }
}
