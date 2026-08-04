// TBX Translator - main.rs
// Creator: samwns

use gtk4 as gtk;
use gtk::prelude::*;
use gtk::Application;

mod app_config;
mod api;
mod renpy_extractor;
mod unity_extractor;
mod editor_ui;
mod ui;
mod font_injector;
mod i18n;
mod paths;

const APP_ID: &str = "com.tbx.translator";

#[cfg(target_os = "windows")]
fn init_windows_environment() {
    // Forçar o backend Cairo para evitar bugs de OpenGL / glyph corruption em drivers Windows
    if std::env::var_os("GSK_RENDERER").is_none() {
        std::env::set_var("GSK_RENDERER", "cairo");
    }

    // Configurar schemas do GLib portáteis se disponíveis na pasta do app
    if std::env::var_os("GSETTINGS_SCHEMA_DIR").is_none() {
        let app_dir = paths::app_root();
        let schema_dir = app_dir.join("share").join("glib-2.0").join("schemas");
        if schema_dir.exists() {
            std::env::set_var("GSETTINGS_SCHEMA_DIR", schema_dir);
        }
    }

    // Configurar cache de módulos do gdk-pixbuf se distribuído localmente
    if std::env::var_os("GDK_PIXBUF_MODULE_FILE").is_none() {
        let app_dir = paths::app_root();
        let loaders_cache = app_dir.join("lib").join("gdk-pixbuf-2.0").join("2.10.0").join("loaders.cache");
        if loaders_cache.exists() {
            std::env::set_var("GDK_PIXBUF_MODULE_FILE", loaders_cache);
        }
    }
}

fn main() -> gtk::glib::ExitCode {
    #[cfg(target_os = "windows")]
    init_windows_environment();

    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(ui::build_ui);
    app.run()
}
