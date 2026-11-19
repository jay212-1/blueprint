//! Blueprint: the planning tool of the Showtime Suite (mic plots, setlists,
//! patch plans and more). This crate is the window; the data model lives in
//! `bp-core`.

pub mod app;
mod app_data;
mod app_start;
pub mod settings;
pub mod table;
pub mod theme;

pub use app::{App, Page};

use std::path::PathBuf;

fn icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(include_bytes!("../../../assets/icon.png"))
        .ok()?
        .to_rgba8();
    Some(egui::IconData {
        width: img.width(),
        height: img.height(),
        rgba: img.into_raw(),
    })
}

struct Window(App);

impl eframe::App for Window {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.0.logic(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.0.ui_root(ui);
    }
}

/// Open the window and run until it is closed. Main thread only.
pub fn run(open: Option<PathBuf>) -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Blueprint")
        .with_app_id("blueprint")
        .with_inner_size([1400.0, 860.0])
        .with_min_inner_size([900.0, 600.0])
        .with_drag_and_drop(true);
    if let Some(icon) = icon() {
        viewport = viewport.with_icon(icon);
    }
    eframe::run_native(
        "Blueprint",
        eframe::NativeOptions {
            viewport,
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(Window(App::new(&cc.egui_ctx, open))))),
    )
}
