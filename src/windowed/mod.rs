use crate::config::TracerConfig;
use crate::fps::FPS;
use crate::tracer::Tracer;
use crate::windowed::front::TracerWindowedFront;
use crate::windowed::ui::UICompositor;
use build_info::BuildInfo;
use egui::{ClippedPrimitive, FullOutput, TexturesDelta};
use glam::UVec2;
use log::info;
use std::cell::RefCell;
use std::rc::Rc;
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalSize, Size};
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::{Window, WindowAttributes, WindowId};

mod front;
mod runtime;
mod ui;

struct Context {
    fps: FPS,
    window: Window,
    tracer: Tracer<TracerWindowedFront>,
    ui: Rc<RefCell<UICompositor>>,
}

pub struct TracerApp {
    build_info: BuildInfo,
    viewport: UVec2,
    config: TracerConfig,
    context: Option<Context>,
}

impl TracerApp {
    pub fn new(config: TracerConfig, initial_viewport: UVec2, bi: BuildInfo) -> Self {
        Self {
            viewport: initial_viewport,
            build_info: bi,
            context: None,
            config,
        }
    }
}

impl ApplicationHandler for TracerApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let title = format!(
            "{} (v{})",
            self.build_info.crate_info.name,
            self.build_info.crate_info.version.to_string()
        );
        let size = Size::Physical(PhysicalSize::new(self.viewport.x, self.viewport.y));

        let attributes = WindowAttributes::default().with_title(title);

        #[cfg(target_os = "linux")]
        let attributes = {
            use winit::platform::x11::WindowAttributesExtX11;
            // Make my I3 happy
            attributes.with_base_size(size).with_x11_window_type(vec![
                winit::platform::x11::WindowType::Normal,
                winit::platform::x11::WindowType::Dialog,
            ])
        };

        #[cfg(not(target_os = "linux"))]
        let attributes = { attributes.with_inner_size(size) };
        let window = event_loop.create_window(attributes).unwrap();

        let context = UICompositor::new_context();
        let id = context.viewport_id();
        let state = egui_winit::State::new(context, id, &window, None, None, None);
        let ui = Rc::new(RefCell::new(UICompositor::new(state)));

        let tracer = unsafe {
            Tracer::<TracerWindowedFront>::new(
                self.config.clone(),
                self.viewport,
                self.build_info.clone(),
                |entry, instance| {
                    TracerWindowedFront::new(
                        entry,
                        instance,
                        self.viewport,
                        window.window_handle()?,
                        window.display_handle()?,
                        ui.clone(),
                    )
                },
            )
            .unwrap()
        };

        self.context = Some(Context {
            fps: FPS::new(),
            window,
            tracer,
            ui,
        });

        info!("Initialized windowed tracer");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let context = self.context.as_mut().unwrap();

        let _ = context
            .ui
            .borrow_mut()
            .egui
            .on_window_event(&context.window, &event);

        match event {
            WindowEvent::Resized(physical_size) => unsafe {
                info!("Window resized to {:?}", physical_size);
                self.viewport = UVec2::new(physical_size.width, physical_size.height);
                context.tracer.resize(self.viewport).unwrap();
            },
            WindowEvent::RedrawRequested => unsafe {
                context.tracer.trace(Some(&context.window)).unwrap();

                let fps = context.fps.update();
                context.ui.borrow_mut().set_fps(fps);
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

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(context) = self.context.as_mut() {
            context.window.request_redraw();
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.context = None;
        info!("Suspended application and destroyed window");
    }
}
