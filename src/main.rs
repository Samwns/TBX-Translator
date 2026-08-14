#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// TBX Translator - main.rs
// Creator: samwns
// Pure Rust Cross-Platform Entry Point

mod types;
mod app_config;
mod api;
mod renpy_extractor;
mod renpy_parser;
mod unity_extractor;
mod godot_pck;
pub mod godot_extractor;
mod editor_ui;
mod font_injector;
pub mod dictionary;
mod locales_gen;
mod i18n;
mod paths;
mod updater;
mod ui;

fn load_icon() -> Option<std::sync::Arc<egui::IconData>> {
    let image_bytes = include_bytes!("../assets/app_icon.png");
    if let Ok(img) = image::load_from_memory(image_bytes) {
        let rgba = img.into_rgba8();
        let (width, height) = rgba.dimensions();
        Some(std::sync::Arc::new(egui::IconData {
            rgba: rgba.into_raw(),
            width,
            height,
        }))
    } else {
        None
    }
}

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    ui::run_app(load_icon())
}
