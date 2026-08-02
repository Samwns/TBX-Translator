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

fn main() -> gtk::glib::ExitCode {
    // The GTK OpenGL renderer can corrupt glyphs under Wine/PortProton.  Cairo
    // is the software renderer shipped with GTK and keeps the same CSS UI,
    // while rendering reliably both on native Windows and Wine.
    #[cfg(target_os = "windows")]
    if std::env::var_os("GSK_RENDERER").is_none() {
        std::env::set_var("GSK_RENDERER", "cairo");
    }

    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(ui::build_ui);
    app.run()
}
