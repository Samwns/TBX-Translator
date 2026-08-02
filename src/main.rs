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
    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(ui::build_ui);
    app.run()
}
