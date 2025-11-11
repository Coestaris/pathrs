use gpu_allocator::AllocatorReport;

pub enum AllocatorReportRequest {
    Empty,
    Requested,
    Ready(AllocatorReport),
}

pub struct UICompositor {
    pub egui: egui_winit::State,
    pub allocator_report: AllocatorReportRequest,
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
            allocator_report: AllocatorReportRequest::Empty,
            fps: 0.0,
        }
    }

    pub fn set_fps(&mut self, fps: f32) {
        self.fps = fps;
    }

    pub fn set_allocator_report<F>(&mut self, mut set_report: F)
    where
        F: FnMut() -> AllocatorReport,
    {
        if let AllocatorReportRequest::Requested = self.allocator_report {
            self.allocator_report = AllocatorReportRequest::Ready(set_report());
        }
    }

    pub(crate) fn render(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Window::new("Allocator Report")
                .resizable(true)
                .default_width(300.0)
                .show(ui.ctx(), |ui| {
                    if ui.button("Refresh Report").clicked() {
                        self.allocator_report = AllocatorReportRequest::Requested;
                    }
                    if let AllocatorReportRequest::Ready(report) = &self.allocator_report {
                        ui.label(format!(
                            "Total Allocated: {} bytes",
                            report.total_allocated_bytes
                        ));
                        ui.label(format!(
                            "Total Capacity: {} bytes",
                            report.total_capacity_bytes
                        ));
                        ui.separator();
                        for (i, alloc) in report.allocations.iter().enumerate() {
                            ui.label(format!(
                                "{}: {} (offset: {}, size: {})",
                                i, alloc.name, alloc.offset, alloc.size
                            ));
                        }
                    } else {
                        ui.label("No allocator report available.");
                    }
                });
            egui::Window::new("Tracer Controls")
                .resizable(true)
                .default_width(300.0)
                .show(ui.ctx(), |ui| {
                    ui.label(format!("FPS: {:.2}", self.fps));
                    ui.label("Tracer Controls go here");
                });
        });
    }
}
