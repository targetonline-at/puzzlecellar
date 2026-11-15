// Im Release-Build kein schwarzes Konsolenfenster
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use eframe::egui;

fn main() -> eframe::Result {
    let optionen = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("PuzzleCellar")
            .with_inner_size([520.0, 600.0])
            .with_min_inner_size([460.0, 540.0]),
        ..Default::default()
    };
    eframe::run_native(
        "PuzzleCellar",
        optionen,
        Box::new(|cc| Ok(Box::new(app::RaetselApp::neu(cc)))),
    )
}
