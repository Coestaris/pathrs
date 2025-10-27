use crate::tracer::front::TracerSurface;
use crate::tracer::Tracer;
use build_info::BuildInfo;
use glam::UVec2;
use log::{debug, info};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalSize, Size};
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::platform::x11::WindowAttributesExtX11;
use winit::window::{Window, WindowAttributes, WindowId};

pub struct App {
    build_info: BuildInfo,
    viewport: UVec2,
    window: Option<Window>,
    tracer: Option<Tracer>,
}

impl App {
    pub fn new(initial_viewport: UVec2, bi: BuildInfo) -> Self {
        Self {
            viewport: initial_viewport,
            build_info: bi,
            window: None,
            tracer: None,
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

        self.window = Some(event_loop.create_window(attributes).unwrap());
        info!("Created window with viewport {:?}", self.viewport);

        let surface = TracerSurface::new_from_winit(self.window.as_ref().unwrap()).unwrap();
        self.tracer = Some(
            Tracer::new_windowed(
                self.viewport,
                self.build_info.clone(),
                Default::default(),
                surface,
            )
            .unwrap(),
        );
        info!("Initialized windowed tracer");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::Resized(physical_size) => {
                info!("Window resized to {:?}", physical_size);
                self.viewport = UVec2::new(physical_size.width, physical_size.height);
                self.tracer.as_mut().unwrap().resize(self.viewport).unwrap();
            }
            WindowEvent::RedrawRequested => {
                self.tracer.as_mut().unwrap().trace().unwrap();
            }
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

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        self.window.as_ref().unwrap().request_redraw();
    }
}
