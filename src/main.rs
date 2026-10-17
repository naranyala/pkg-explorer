#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cli;
mod extract;
mod models;
mod sources;

use eframe::egui;

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        cli::run(&args);
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("PKG Explorer"),
        ..Default::default()
    };
    eframe::run_native(
        "PKG Explorer",
        options,
        Box::new(|cc| Ok(Box::new(app::PkgApp::new(cc)))),
    )
}
