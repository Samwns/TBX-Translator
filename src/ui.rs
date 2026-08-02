// TPG Translator - ui.rs
// Creator: samwns
// GTK4 UI — faithful replica of the Java/JavaFX design with:
//   • Real TPG icon from assets/icon.png
//   • Draggable frameless window (GestureClick + begin_move_drag)
//   • CSS keyframe animations (fade-in, pulse, hover transitions)

use gtk4 as gtk;
use gtk4::{
    prelude::*, Align, Application, ApplicationWindow, Box, Button, ComboBoxText,
    Entry, Label, Orientation, ProgressBar, ScrolledWindow, Separator, Stack,
    TextBuffer, TextView, ToggleButton, CheckButton, MessageDialog, MessageType,
    ButtonsType, ResponseType, DialogFlags, CssProvider, Image
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::path::Path;

use crate::app_config::AppConfig;

const IDIOMAS: &[&str] = &[
    "Detectar Automaticamente",
    "Alemão","Chinês (Simplificado)","Coreano",
    "Espanhol","Francês","Inglês","Italiano",
    "Japonês","Português","Russo",
];

// ── CSS — exact colors from style.css + GTK4 animations ────────────
const CSS: &str = "
/* ─── Global ───────────────────────────────────────────────────── */
* { font-family: sans-serif; }

window.main-transparent {
    background-color: transparent;
}

/* ─── Fade-in animation for the whole window on open ────────────── */
@keyframes fadein {
    from { opacity: 0; }
    to   { opacity: 1; }
}
@keyframes slidein {
    from { opacity: 0; margin-top: 12px; }
    to   { opacity: 1; margin-top: 0px;  }
}
@keyframes pulse-glow {
    0%   { box-shadow: 0 0 0px 0px rgba(249,226,175,0.0); }
    50%  { box-shadow: 0 0 14px 4px rgba(249,226,175,0.35); }
    100% { box-shadow: 0 0 0px 0px rgba(249,226,175,0.0); }
}
@keyframes pulse-glow-unity {
    0%   { box-shadow: 0 0 0px 0px rgba(137,180,250,0.0); }
    50%  { box-shadow: 0 0 14px 4px rgba(137,180,250,0.35); }
    100% { box-shadow: 0 0 0px 0px rgba(137,180,250,0.0); }
}
@keyframes spin {
    from { -gtk-icon-transform: rotate(0deg); }
    to   { -gtk-icon-transform: rotate(360deg); }
}

/* ─── App shell ─────────────────────────────────────────────────── */
.app-shell {
    background-color: #1e1e2e;
    border-radius: 8px;
    border: 2px solid #585b70;
    border-top-color: #a6adc8;
    border-left-color: #a6adc8;
    border-bottom-color: #11111b;
    border-right-color: #11111b;
    animation: fadein 350ms ease-out both;
}

/* ─── Title bar ─────────────────────────────────────────────────── */
.title-bar {
    background-color: #0b0b12;
    padding: 10px 15px 10px 14px;
    border-radius: 6px 6px 0 0;
    border-bottom: 1px solid #181825;
}

.app-title {
    font-size: 17px;
    font-weight: bold;
    color: #f9e2af;
    transition: color 400ms ease;
}
.app-title.unity-mode { color: #89b4fa; }

.title-underline {
    background-color: #f9e2af;
    min-height: 2px;
    border-radius: 2px;
    opacity: 0.45;
    transition: background-color 400ms ease;
}
.title-underline.unity-mode { background-color: #89b4fa; }

/* ─── Window controls ───────────────────────────────────────────── */
.btn-win {
    background-color: transparent;
    border: none;
    font-size: 15px;
    min-width: 0;
    padding: 4px 8px;
    border-radius: 4px;
    transition: background-color 150ms ease, color 150ms ease;
}
.btn-win-min { color: #6c7086; }
.btn-win-min:hover { background-color: #45475a; color: #ffffff; }
.btn-win-max { color: #a6e3a1; }
.btn-win-max:hover { background-color: rgba(166,227,161,0.15); }
.btn-win-close { color: #f38ba8; }
.btn-win-close:hover { background-color: #f38ba8; color: #11111b; }

/* ─── Toolbar ───────────────────────────────────────────────────── */
.toolbar {
    background-color: #161622;
    padding: 3px 8px;
    border-bottom: 1px solid #313244;
}
.toolbar-tab {
    background-color: transparent;
    border: none;
    border-radius: 6px 6px 0 0;
    padding: 7px 12px;
    min-width: 0;
    color: #585b70;
    font-size: 18px;
    transition: background-color 150ms ease, color 150ms ease;
    border-bottom: 2px solid transparent;
}
.toolbar-tab:hover { background-color: #1e1e2e; color: #ffffff; }
.toolbar-tab.active {
    background-color: #1e1e2e;
    color: #ffffff;
    border-bottom: 2px solid #cba6f7;
}

/* ─── General labels ────────────────────────────────────────────── */
label { 
    color: #ffffff; 
    font-size: 14px; 
}
.detected-label { font-weight: bold; color: #ffffff; }
.muted-label { color: #a6adc8; font-size: 13px; }
.section-label-purple { color: #cba6f7; font-weight: bold; font-size: 13px; }
.settings-title { font-size: 20px; font-weight: bold; color: #cba6f7; }
.warning-label { color: #f38ba8; font-size: 11px; }

/* ─── Entries ───────────────────────────────────────────────────── */
entry {
    background-color: #313244;
    color: #ffffff;
    border-radius: 4px;
    border: 1px solid #45475a;
    padding: 8px;
    caret-color: #cba6f7;
    transition: border-color 200ms ease;
}
entry:focus { border-color: #cba6f7; }
entry:disabled { opacity: 0.4; }

/* ─── ComboBox ──────────────────────────────────────────────────── */
combobox button {
    background-color: #313244;
    color: #ffffff;
    border-radius: 4px;
    border: 2px solid #585b70;
    border-top-color: #11111b;
    border-left-color: #11111b;
    border-bottom-color: #a6adc8;
    border-right-color: #a6adc8;
    padding: 7px 10px;
    transition: background-color 150ms ease;
}
combobox button:hover { background-color: #3d3f55; }

/* ─── Engine tab buttons (Ren'Py / Unity) ───────────────────────── */
.engine-bar {
    background-color: #11111b;
    border-radius: 8px 8px 0 0;
    padding: 4px 6px 0 6px;
    border-bottom: 1px solid #313244;
}
.game-tab-btn {
    background-color: transparent;
    color: #585b70;
    border: none;
    border-radius: 6px 6px 0 0;
    padding: 8px 22px;
    font-size: 13px;
    font-weight: bold;
    transition: background-color 200ms ease, color 200ms ease;
    border-bottom: 2px solid transparent;
}
.game-tab-btn:hover { color: #ffffff; background-color: #1e1e2e; }
.game-tab-btn.active-renpy {
    background-color: #1e1e2e;
    color: #f9e2af;
    border-bottom: 2px solid #f9e2af;
}
.game-tab-btn.active-unity {
    background-color: #1e1e2e;
    color: #89b4fa;
    border-bottom: 2px solid #89b4fa;
}

/* ─── Action buttons ────────────────────────────────────────────── */
.btn-translate-renpy {
    background-color: #f9e2af;
    color: #11111b;
    font-weight: bold;
    font-size: 15px;
    border-radius: 10px;
    border: none;
    padding: 14px 32px;
    animation: pulse-glow 2.5s ease-in-out infinite;
    transition: background-color 180ms ease, box-shadow 180ms ease;
}
.btn-translate-renpy:hover {
    background-color: #fab387;
    animation: none;
    box-shadow: 0 4px 18px rgba(249,226,175,0.4);
}
.btn-translate-renpy label {
    color: #ffffff;
    text-shadow: -1px -1px 0 #11111b, 1px -1px 0 #11111b,
                 -1px 1px 0 #11111b, 1px 1px 0 #11111b;
}

.btn-translate-unity {
    background-color: #89b4fa;
    color: #11111b;
    font-weight: bold;
    font-size: 15px;
    border-radius: 10px;
    border: none;
    padding: 14px 32px;
    animation: pulse-glow-unity 2.5s ease-in-out infinite;
    transition: background-color 180ms ease, box-shadow 180ms ease;
}
.btn-translate-unity:hover {
    background-color: #74c7ec;
    animation: none;
    box-shadow: 0 4px 18px rgba(137,180,250,0.4);
}
.btn-translate-unity label {
    color: #ffffff;
    text-shadow: -1px -1px 0 #11111b, 1px -1px 0 #11111b,
                 -1px 1px 0 #11111b, 1px 1px 0 #11111b;
}

.btn-editor {
    background-color: #89b4fa;
    color: #11111b;
    font-weight: bold;
    font-size: 14px;
    border-radius: 10px;
    border: none;
    padding: 14px 24px;
    transition: background-color 180ms ease;
}
.btn-editor:hover { background-color: #74c7ec; }
.btn-editor label {
    color: #ffffff;
    text-shadow: -1px -1px 0 #11111b, 1px -1px 0 #11111b,
                 -1px 1px 0 #11111b, 1px 1px 0 #11111b;
}

.btn-browse {
    background-color: #313244;
    color: #cba6f7;
    font-weight: bold;
    border-radius: 4px;
    border: 2px solid #585b70;
    border-top-color: #a6adc8;
    border-left-color: #a6adc8;
    border-bottom-color: #11111b;
    border-right-color: #11111b;
    padding: 8px 16px;
    transition: background-color 150ms ease, color 150ms ease;
}
.btn-browse:hover { background-color: #cba6f7; color: #11111b; }
.btn-browse:active {
    border-top-color: #11111b;
    border-left-color: #11111b;
    border-bottom-color: #a6adc8;
    border-right-color: #a6adc8;
}

.btn-cancel {
    background-color: #f38ba8;
    color: #11111b;
    font-weight: bold;
    border-radius: 4px;
    border: 2px solid #585b70;
    border-top-color: #ffb4c2;
    border-left-color: #ffb4c2;
    border-bottom-color: #11111b;
    border-right-color: #11111b;
    padding: 5px 10px;
    min-width: 36px;
    transition: background-color 150ms ease;
}
.btn-cancel:hover { background-color: #eba0ac; }
.btn-cancel:active {
    border-top-color: #11111b;
    border-left-color: #11111b;
    border-bottom-color: #ffb4c2;
    border-right-color: #ffb4c2;
}

.btn-save-config {
    background-color: #a6e3a1;
    color: #11111b;
    font-weight: bold;
    font-size: 13px;
    border-radius: 4px;
    border: 2px solid #585b70;
    border-top-color: #c0ebbd;
    border-left-color: #c0ebbd;
    border-bottom-color: #11111b;
    border-right-color: #11111b;
    padding: 10px 22px;
    transition: background-color 180ms ease;
}
.btn-save-config:hover { background-color: #94d58d; }
.btn-save-config:active {
    border-top-color: #11111b;
    border-left-color: #11111b;
    border-bottom-color: #c0ebbd;
    border-right-color: #c0ebbd;
}

/* ─── Toggle / Switch ───────────────────────────────────────────── */
.switch-toggle {
    min-width: 66px;
    min-height: 30px;
    background-color: #45475a;
    border-radius: 999px;
    border: 2px solid #585b70;
    border-top-color: #11111b;
    border-left-color: #11111b;
    color: #ffffff;
    font-size: 11px;
    font-weight: bold;
    padding: 4px 10px;
    transition: background-color 220ms ease, border-color 220ms ease, color 150ms ease;
}
.switch-toggle:hover { background-color: #585b70; }
.switch-toggle:checked { 
    background-color: #a6e3a1; 
    border-color: #a6e3a1; 
    border-top-color: #11111b;
    border-left-color: #11111b;
    color: #11111b; 
}
.switch-toggle:checked:hover { background-color: #94d58d; }
.switch-toggle:disabled { opacity: 0.4; }

/* ─── Progress bar ──────────────────────────────────────────────── */
progressbar trough {
    background-color: #313244;
    border-radius: 4px;
    min-height: 8px;
}
progressbar progress {
    background-color: #cba6f7;
    border-radius: 4px;
    background-image: linear-gradient(
        -45deg,
        rgba(255, 255, 255, 0.15) 25%,
        transparent 25%,
        transparent 50%,
        rgba(255, 255, 255, 0.15) 50%,
        rgba(255, 255, 255, 0.15) 75%,
        transparent 75%,
        transparent
    );
    background-size: 20px 20px;
    animation: move-stripes 1.5s linear infinite;
    transition: min-width 0.3s ease;
}
@keyframes move-stripes {
    from { background-position: 0 0; }
    to { background-position: 20px 0; }
}

/* ─── Log view ──────────────────────────────────────────────────── */
.log-view text {
    background-color: #0b0b12;
    color: #a6e3a1;
    font-family: 'Consolas', 'JetBrains Mono', 'Monospace';
    font-size: 13px;
}
.log-tab-close {
    min-width: 0;
    min-height: 0;
    padding: 0 5px;
    margin: 0;
    border: none;
    background: transparent;
    color: #f38ba8;
    font-weight: bold;
}
.log-tab-close:hover { background-color: #f38ba8; color: #11111b; }
.log-view { 
    background-color: #0b0b12; 
    border: 3px solid #313244; 
    border-top-color: #11111b; 
    border-left-color: #11111b; 
    border-bottom-color: #585b70; 
    border-right-color: #585b70; 
    border-radius: 4px; 
}

/* ─── Separator ─────────────────────────────────────────────────── */
separator { background-color: #313244; min-height: 1px; }

/* ─── Page content areas ────────────────────────────────────────── */
.page-translate { animation: slidein 300ms ease-out both; }
.page-logs      { animation: slidein 300ms ease-out both; }
.page-settings  { animation: slidein 300ms ease-out both; }
";

// ── Message enum ────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub enum UiMsg {
    Log(String),
    Progress(usize, usize),
    Done(String),
}

fn log_tab_title(notebook: &gtk::Notebook, page: &gtk::Widget) -> Option<String> {
    let tab = notebook.tab_label(page)?;
    if let Ok(label) = tab.clone().downcast::<Label>() {
        return Some(label.text().to_string());
    }
    tab.downcast::<Box>().ok()?
        .first_child()?
        .downcast::<Label>()
        .ok()
        .map(|label| label.text().to_string())
}

fn append_log_tab(notebook: &gtk::Notebook, scroll: &ScrolledWindow, title: &str, closable: bool) {
    if !closable {
        notebook.append_page(scroll, Some(&Label::new(Some(title))));
        return;
    }

    let tab = Box::new(Orientation::Horizontal, 5);
    let label = Label::new(Some(title));
    let close = Button::with_label("×");
    close.add_css_class("log-tab-close");
    close.set_tooltip_text(Some("Fechar log"));
    tab.append(&label);
    tab.append(&close);

    let notebook_for_close = notebook.clone();
    let scroll_for_close = scroll.clone();
    close.connect_clicked(move |_| {
        if let Some(page) = notebook_for_close.page_num(&scroll_for_close) {
            notebook_for_close.remove_page(Some(page));
        }
    });
    notebook.append_page(scroll, Some(&tab));
}

pub fn append_log(buf: &TextBuffer, msg: &str) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    let mut end = buf.end_iter();
    buf.insert(&mut end, &format!("[{:02}:{:02}:{:02}] {}\n", h, m, s, msg));
}

pub fn build_ui(app: &Application) {
    // Load CSS
    let provider = CssProvider::new();
    provider.load_from_data(CSS);
    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("no display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let cfg = AppConfig::carregar();
    let cfg_rc = Rc::new(RefCell::new(cfg.clone()));
    let cancelled = Arc::new(AtomicBool::new(false));
    let engine_mode: Rc<RefCell<u32>> = Rc::new(RefCell::new(
        if cfg.modo_jogo == "unity" { 1 } else { 0 }
    ));
    let lang = cfg.ui_language.clone();
    let lang_t = lang.clone();
    let t = Rc::new(move |k: &str| crate::i18n::t(k, &lang_t).to_string());

    // ── Window ──────────────────────────────────────────────────────
    // Register Icon Theme search path so GTK can find com.tbx.translator.svg
    let display = gtk::gdk::Display::default().unwrap();
    let icon_theme = gtk::IconTheme::for_display(&display);
    let assets_dir = crate::paths::app_root().join("assets");
    icon_theme.add_search_path(&assets_dir);

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Toolbox Translator")
        .default_width(854)
        .default_height(530)
        .resizable(false)
        .decorated(false)
        .build();
    window.set_default_size(950, 650);
    window.set_size_request(950, 650);
    window.add_css_class("main-transparent");

    let root = Box::new(Orientation::Vertical, 0);
    root.add_css_class("app-shell");

    // ════════════════════════════════════════════════════════════════
    // TITLE BAR
    // ════════════════════════════════════════════════════════════════
    let title_bar = Box::new(Orientation::Horizontal, 12);
    title_bar.add_css_class("title-bar");

    // ── Real icon from assets/com.tbx.translator.svg ───────────────────────────
    // Try to load from the workspace-relative path
    let icon_path = crate::paths::asset_path("com.tbx.translator.svg");
    let icon_img = if icon_path.exists() {
            let img = Image::from_file(&icon_path);
            img.set_pixel_size(32);
            img
        } else {
            let img = Image::from_icon_name("application-x-executable");
            img.set_pixel_size(32);
            img
        };
    icon_img.set_margin_end(4);

    // ── Title + underline ─────────────────────────────────────────
    let title_vbox = Box::new(Orientation::Vertical, 3);
    let app_title_lbl = Label::new(Some("TBX Translator"));
    app_title_lbl.add_css_class("app-title");
    app_title_lbl.set_halign(gtk::Align::Start);
    let title_line = Box::new(Orientation::Horizontal, 0); // underline box
    title_line.add_css_class("title-underline");
    title_line.set_size_request(130, 2);
    title_vbox.append(&app_title_lbl);
    title_vbox.append(&title_line);

    let spacer = Box::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);

    // ── Window controls ───────────────────────────────────────────
    let win_btns = Box::new(Orientation::Horizontal, 2);
    let btn_min = Button::with_label("—");
    btn_min.add_css_class("btn-win"); btn_min.add_css_class("btn-win-min");
    let btn_close = Button::with_label("✕");
    btn_close.add_css_class("btn-win"); btn_close.add_css_class("btn-win-close");
    win_btns.append(&btn_min); win_btns.append(&btn_close);

    let win_min = window.clone();
    btn_min.connect_clicked(move |_| {
        win_min.minimize();
    });
    
    let win_close = window.clone();
    btn_close.connect_clicked(move |_| {
        win_close.close();
    });

    title_bar.append(&icon_img);
    title_bar.append(&title_vbox);
    title_bar.append(&spacer);
    title_bar.append(&win_btns);

    let window_handle = gtk::WindowHandle::new();
    window_handle.set_child(Some(&title_bar));

    // Window button actions
    let app_clone = app.clone(); btn_close.connect_clicked(move |_| app_clone.quit());
    let wc = window.clone(); btn_min.connect_clicked(move |_| wc.minimize());

    root.append(&window_handle);

    // ════════════════════════════════════════════════════════════════
    // TOOLBAR (3 icon tabs: 📁 ☰ ⚙)
    // ════════════════════════════════════════════════════════════════
    let toolbar = Box::new(Orientation::Horizontal, 0);
    toolbar.add_css_class("toolbar");

    macro_rules! toolbar_btn {
        ($icon_path:expr) => {{
            let b = Button::new();
            let img = Image::from_file($icon_path);
            img.set_pixel_size(20);
            b.set_child(Some(&img));
            b.add_css_class("toolbar-tab");
            b
        }};
    }
    let tb_translate = toolbar_btn!(crate::paths::asset_path("folder_icon.svg"));
    let tb_logs      = toolbar_btn!(crate::paths::asset_path("logs_icon.svg"));
    let tb_tools     = toolbar_btn!(crate::paths::asset_path("tools_icon.svg"));
    let tb_settings  = toolbar_btn!(crate::paths::asset_path("settings_icon.svg"));
    tb_translate.add_css_class("active");

    toolbar.append(&tb_translate);
    toolbar.append(&tb_logs);
    toolbar.append(&tb_tools);
    toolbar.append(&tb_settings);

    let tb_spacer = gtk::Box::new(Orientation::Horizontal, 0);
    tb_spacer.set_hexpand(true);
    toolbar.append(&tb_spacer);

    let lbl_version = Label::new(Some("v0.0.1-alpha | by samwns"));
    lbl_version.add_css_class("version-label");
    lbl_version.set_margin_end(15);
    toolbar.append(&lbl_version);

    root.append(&toolbar);
    root.append(&Separator::new(Orientation::Horizontal));

    // ════════════════════════════════════════════════════════════════
    // STACK
    // ════════════════════════════════════════════════════════════════
    let stack = gtk::Stack::new();
    stack.add_css_class("main-stack");
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    stack.set_transition_duration(400);
    stack.set_vexpand(true);

    // ────────────────────────────────────────────────────────────────
    // HELPERS
    // ────────────────────────────────────────────────────────────────
    let mk_engine_btn = |label: &str, icon_path: &str| {
        let b = Button::new();
        let bx = Box::new(Orientation::Horizontal, 8);
        bx.set_halign(gtk::Align::Center);
        bx.set_valign(gtk::Align::Center);
        let img = Image::from_file(icon_path);
        img.set_pixel_size(18);
        let lbl = Label::new(Some(label));
        bx.append(&img);
        bx.append(&lbl);
        b.set_child(Some(&bx));
        b.add_css_class("game-tab-btn");
        b
    };

    // ────────────────────────────────────────────────────────────────
    // PAGE 1: TRANSLATE
    // ────────────────────────────────────────────────────────────────
    let page_translate = Box::new(Orientation::Vertical, 18);
    page_translate.add_css_class("page-translate");
    page_translate.set_margin_top(28); page_translate.set_margin_bottom(28);
    page_translate.set_margin_start(30); page_translate.set_margin_end(30);

    // Engine tab bar
    let engine_bar = Box::new(Orientation::Horizontal, 0);
    engine_bar.add_css_class("engine-bar");

    let btn_renpy_tab = mk_engine_btn("Ren'Py", &crate::paths::asset_path("renpy_icon.svg").to_string_lossy());
    let btn_unity_tab = mk_engine_btn("Unity", &crate::paths::asset_path("unity_icon.svg").to_string_lossy());
    btn_renpy_tab.add_css_class("active-renpy");
    engine_bar.append(&btn_renpy_tab);
    engine_bar.append(&btn_unity_tab);
    page_translate.append(&engine_bar);

    // Executable path row
    let exe_row = Box::new(Orientation::Horizontal, 10);
    let exe_lbl = Label::new(Some("Executable:"));
    let path_entry = Entry::new();
    path_entry.set_placeholder_text(Some(&t("selecione_pasta")));
    path_entry.set_hexpand(true);
    path_entry.set_text(if cfg.modo_jogo == "unity" { &cfg.caminho_jogo_unity } else { &cfg.caminho_jogo_renpy });
    let btn_browse = Button::with_label("Procurar...");
    btn_browse.add_css_class("btn-browse");
    exe_row.append(&exe_lbl); exe_row.append(&path_entry); exe_row.append(&btn_browse);
    page_translate.append(&exe_row);

    // Detected row
    let det_row = Box::new(Orientation::Horizontal, 8);
    let det_muted = Label::new(Some("Detected:"));
    det_muted.add_css_class("muted-label");
    let detected_lbl = Label::new(Some("None"));
    detected_lbl.add_css_class("detected-label");
    
    // Auto-detect on startup based on config
    let init_path = path_entry.text().to_string();
    if !init_path.is_empty() {
        let low = init_path.to_lowercase();
        if low.ends_with(".py") || Path::new(&init_path).parent().map(|p| p.join("game").is_dir()).unwrap_or(false) {
            detected_lbl.set_text("Ren'Py");
        } else if low.ends_with(".exe") {
            let backend = crate::unity_extractor::detect_unity_backend(&init_path).unwrap_or("Desconhecido");
            detected_lbl.set_text(&format!("Unity ({})", backend));
        }
    }
    
    det_row.append(&det_muted); det_row.append(&detected_lbl);
    page_translate.append(&det_row);

    // Language + folder row
    let lang_row = Box::new(Orientation::Horizontal, 10);
    let mk_lbl = |s: &str| { let l = Label::new(Some(s)); l };
    let combo_origem = ComboBoxText::new();
    for id in IDIOMAS { combo_origem.append_text(id); }
    if let Some(p) = IDIOMAS.iter().position(|&s| s == cfg.idioma_origem) { combo_origem.set_active(Some(p as u32)); } else { combo_origem.set_active(Some(0)); }
    combo_origem.set_size_request(150, -1);

    let combo_alvo = ComboBoxText::new();
    for id in &IDIOMAS[1..] { combo_alvo.append_text(id); }
    if let Some(p) = IDIOMAS[1..].iter().position(|&s| s == cfg.idioma_alvo) { combo_alvo.set_active(Some(p as u32)); } else { combo_alvo.set_active(Some(8)); }
    combo_alvo.set_size_request(150, -1);

    let pasta_entry = Entry::new();
    pasta_entry.set_text(&cfg.pasta_traducao);
    pasta_entry.set_size_request(140, -1);
    let lbl_para = mk_lbl(&t("idioma_alvo")); lbl_para.set_margin_start(10);
    let lbl_pasta = mk_lbl(&t("pasta_trad")); lbl_pasta.set_margin_start(10);
    
    let lbl_pasta_clone = lbl_pasta.clone();
    
    lang_row.append(&mk_lbl(&t("idioma_orig"))); lang_row.append(&combo_origem);
    lang_row.append(&lbl_para);    lang_row.append(&combo_alvo);
    lang_row.append(&lbl_pasta);   lang_row.append(&pasta_entry);
    page_translate.append(&lang_row);

    // Action area (buttons ↔ progress)
    let action_stack = gtk::Stack::new();
    action_stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    action_stack.set_transition_duration(400);
    action_stack.set_size_request(-1, 80);

    // Buttons layer
    let buttons_box = Box::new(Orientation::Horizontal, 20);
    buttons_box.set_halign(gtk::Align::Center);
    buttons_box.set_valign(gtk::Align::Center);

    let btn_translate = Button::with_label(&t("iniciar_trad_renpy"));
    btn_translate.add_css_class("btn-translate-renpy");
    btn_translate.set_size_request(240, 50);
    
    let btn_inject = Button::with_label("INJETAR TRADUÇÃO");
    btn_inject.add_css_class("btn-translate-renpy"); // we will swap this dynamically too
    btn_inject.set_size_request(200, 50);
    btn_inject.set_visible(false); // Default false in Ren'Py unless checked

    let btn_editor = Button::with_label(&t("abrir_editor"));
    btn_editor.add_css_class("btn-editor");
    btn_editor.set_size_request(200, 50);
    btn_editor.set_visible(true);

    buttons_box.append(&btn_translate);
    buttons_box.append(&btn_inject);
    buttons_box.append(&btn_editor);

    // Progress layer
    let progress_box = Box::new(Orientation::Vertical, 8);
    progress_box.set_halign(gtk::Align::Center);
    progress_box.set_valign(gtk::Align::Center);
    let prog_inner = Box::new(Orientation::Horizontal, 14);
    let progress_bar = ProgressBar::new();
    progress_bar.set_size_request(540, 22);
    let btn_cancel = Button::with_label("✕");
    btn_cancel.add_css_class("btn-cancel");
    btn_cancel.set_size_request(36, 30);
    prog_inner.append(&progress_bar); prog_inner.append(&btn_cancel);
    let progress_lbl = Label::new(Some("0 / 0 extraídos"));
    progress_lbl.add_css_class("muted-label");
    progress_lbl.set_halign(gtk::Align::Center);
    progress_box.append(&prog_inner); progress_box.append(&progress_lbl);

    action_stack.add_named(&buttons_box, Some("buttons"));
    action_stack.add_named(&progress_box, Some("progress"));
    action_stack.set_visible_child_name("buttons");
    page_translate.append(&action_stack);

    stack.add_named(&page_translate, Some("translate"));

    // ────────────────────────────────────────────────────────────────
    // PAGE 2: LOGS
    // ────────────────────────────────────────────────────────────────
    let page_logs = Box::new(Orientation::Vertical, 10);
    page_logs.add_css_class("page-logs");
    page_logs.set_margin_top(20); page_logs.set_margin_bottom(20);
    page_logs.set_margin_start(20); page_logs.set_margin_end(20);

    let console_lbl = Label::new(Some("Console de Eventos:"));
    console_lbl.add_css_class("section-label-purple");
    console_lbl.set_halign(gtk::Align::Start);

    let log_notebook = gtk::Notebook::new();
    log_notebook.set_vexpand(true);
    
    let log_buffer_geral = TextBuffer::new(None);
    let log_view_geral = TextView::with_buffer(&log_buffer_geral);
    log_view_geral.add_css_class("log-view");
    log_view_geral.set_editable(false);
    log_view_geral.set_wrap_mode(gtk::WrapMode::WordChar);
    log_view_geral.set_cursor_visible(false);
    
    let log_scroll_geral = ScrolledWindow::new();
    log_scroll_geral.set_child(Some(&log_view_geral));
    log_notebook.append_page(&log_scroll_geral, Some(&Label::new(Some("Geral"))));
    
    page_logs.append(&console_lbl);
    page_logs.append(&log_notebook);
    stack.add_named(&page_logs, Some("logs"));

    // ────────────────────────────────────────────────────────────────
    // PAGE 3: SETTINGS
    // ────────────────────────────────────────────────────────────────
    let page_settings = Box::new(Orientation::Vertical, 18);
    page_settings.add_css_class("page-settings");
    page_settings.set_margin_top(28); page_settings.set_margin_bottom(28);
    page_settings.set_margin_start(30); page_settings.set_margin_end(30);

    let settings_title = Label::new(Some(&t("config_geral")));
    settings_title.add_css_class("section-title");
    settings_title.set_halign(gtk::Align::Start);
    page_settings.append(&settings_title);

    let lang_ui_row = Box::new(Orientation::Horizontal, 14);
    let lang_ui_lbl = Label::new(Some(&t("idioma_app")));
    lang_ui_lbl.set_size_request(280, -1);
    let combo_ui_lang = ComboBoxText::new();
    combo_ui_lang.append_text("Português (BR)");
    combo_ui_lang.append_text("English (US)");
    combo_ui_lang.set_active(Some(if cfg.ui_language == "en_US" { 1 } else { 0 }));
    combo_ui_lang.set_size_request(250, -1);
    lang_ui_row.append(&lang_ui_lbl); lang_ui_row.append(&combo_ui_lang);
    page_settings.append(&lang_ui_row);

    let api_row = Box::new(Orientation::Horizontal, 14);
    let api_lbl = Label::new(Some("API Selecionada:"));
    api_lbl.set_size_request(130, -1);
    let combo_motor = ComboBoxText::new();
    combo_motor.append_text("Google Translator");
    combo_motor.set_active(Some(0));
    combo_motor.set_size_request(250, -1);
    api_row.append(&api_lbl); api_row.append(&combo_motor);
    page_settings.append(&api_row);
    page_settings.append(&Separator::new(Orientation::Horizontal));

    let mk_toggle_row = |label: &str, on: bool, disabled: bool| {
        let row = Box::new(Orientation::Horizontal, 18);
        row.set_valign(gtk::Align::Center);
        let tg = ToggleButton::with_label(if on { "ON" } else { "OFF" });
        tg.set_active(on);
        tg.set_sensitive(!disabled);
        tg.add_css_class("switch-toggle");
        let lbl = Label::new(Some(label));
        row.append(&tg); row.append(&lbl);
        (row, tg)
    };

    // Global Settings Header
    let lbl_global = Label::new(Some("Geral"));
    lbl_global.add_css_class("section-title");
    lbl_global.set_halign(gtk::Align::Start);
    lbl_global.set_margin_top(10);
    page_settings.append(&lbl_global);

    // Multi-thread
    let mt_row = Box::new(Orientation::Horizontal, 18);
    mt_row.set_valign(gtk::Align::Center);
    let tg_multi = ToggleButton::with_label(if cfg.usar_multi_trad { "ON" } else { "OFF" });
    tg_multi.set_active(cfg.usar_multi_trad);
    tg_multi.add_css_class("switch-toggle");
    let mt_lbl = Label::new(Some(&t("ativar_multi")));
    let threads_lbl = Label::new(Some(&t("qtd_threads")));
    threads_lbl.add_css_class("muted-label");
    threads_lbl.set_margin_start(10);
    threads_lbl.set_sensitive(cfg.usar_multi_trad);
    let threads_entry = Entry::new();
    threads_entry.set_text(&cfg.threads_ativas.to_string());
    threads_entry.set_size_request(60, -1);
    threads_entry.set_sensitive(cfg.usar_multi_trad);
    mt_row.append(&tg_multi); mt_row.append(&mt_lbl);
    mt_row.append(&threads_lbl); mt_row.append(&threads_entry);
    page_settings.append(&mt_row);
    
    // Inject button for Ren'Py
    let renpy_inject_row = Box::new(Orientation::Horizontal, 18);
    renpy_inject_row.set_valign(gtk::Align::Center);
    let chk_renpy_inject = CheckButton::with_label("Exibir botão 'Injetar' na aba Ren'Py");
    chk_renpy_inject.set_active(false); // default to false
    renpy_inject_row.append(&chk_renpy_inject);
    page_settings.append(&renpy_inject_row);

    // ============================================
    // CREATING THE MODAL WINDOW FOR ENGINES
    let engine_win = gtk::ApplicationWindow::builder()
        .title("Configurações dos Motores")
        .default_width(500)
        .default_height(350)
        .modal(true)
        .transient_for(&window)
        .hide_on_close(true)
        .build();
    
    let notebook = gtk::Notebook::new();
    
    // Page Ren'Py
    let bx_renpy = Box::new(Orientation::Vertical, 10);
    bx_renpy.set_margin_top(20); bx_renpy.set_margin_bottom(20);
    bx_renpy.set_margin_start(20); bx_renpy.set_margin_end(20);
    let (struct_row, tg_struct) = mk_toggle_row(&t("manter_estrtura"), cfg.manter_estrutura_original, false);
    bx_renpy.append(&struct_row);
    let (names_row, _tg_names) = mk_toggle_row(&t("proteger_var"), true, true);
    bx_renpy.append(&names_row);
    let (tradnomes_row, tg_tradnomes) = mk_toggle_row(&t("trad_nomes"), cfg.traduzir_nomes_personagens_renpy, false);
    bx_renpy.append(&tradnomes_row);
    
    let tab_lbl_renpy = Label::new(Some("Ren'Py"));
    notebook.append_page(&bx_renpy, Some(&tab_lbl_renpy));

    // Page Unity
    let bx_unity = Box::new(Orientation::Vertical, 10);
    bx_unity.set_margin_top(20); bx_unity.set_margin_bottom(20);
    bx_unity.set_margin_start(20); bx_unity.set_margin_end(20);
    let unity_desc = Label::new(Some("O Unity AutoTranslator opera de forma global nas pastas BepInEx.\nNenhuma configuração extra é necessária neste momento."));
    unity_desc.add_css_class("muted-label");
    unity_desc.set_wrap(true);
    bx_unity.append(&unity_desc);
    
    let tab_lbl_unity = Label::new(Some("Unity"));
    notebook.append_page(&bx_unity, Some(&tab_lbl_unity));
    
    let engine_root = Box::new(Orientation::Vertical, 0);
    let engine_header = Box::new(Orientation::Horizontal, 8);
    engine_header.set_margin_top(8);
    engine_header.set_margin_bottom(4);
    engine_header.set_margin_start(12);
    engine_header.set_margin_end(8);
    let engine_title = Label::new(Some("Configurações dos Motores"));
    engine_title.add_css_class("section-label-purple");
    engine_title.set_hexpand(true);
    engine_title.set_halign(gtk::Align::Start);
    let engine_close = Button::with_label("✕");
    engine_close.add_css_class("btn-win");
    engine_close.add_css_class("btn-win-close");
    let engine_win_for_close = engine_win.clone();
    engine_close.connect_clicked(move |_| engine_win_for_close.close());
    engine_header.append(&engine_title);
    engine_header.append(&engine_close);
    engine_root.append(&engine_header);
    engine_root.append(&notebook);
    engine_win.set_child(Some(&engine_root));

    let btn_engine_configs = Button::with_label("Configurações dos Motores");
    btn_engine_configs.add_css_class("btn-save-config");
    btn_engine_configs.set_margin_top(20);
    btn_engine_configs.set_size_request(300, 42);
    btn_engine_configs.set_halign(gtk::Align::Start);
    let engine_win_clone = engine_win.clone();
    btn_engine_configs.connect_clicked(move |_| {
        engine_win_clone.present();
    });
    page_settings.append(&btn_engine_configs);
    // ============================================

    let warn_lbl = Label::new(Some(&t("aviso_ip")));
    warn_lbl.add_css_class("warning-label");
    warn_lbl.set_margin_top(20);
    warn_lbl.set_halign(gtk::Align::Start);
    page_settings.append(&warn_lbl);

    let btn_save = Button::with_label(&t("salvar_config"));
    btn_save.add_css_class("btn-save-config");
    btn_save.set_size_request(300, 42);
    btn_save.set_halign(gtk::Align::Start);
    page_settings.append(&btn_save);
    stack.add_named(&page_settings, Some("settings"));

    // ────────────────────────────────────────────────────────────────
    // PAGE 4: TOOLS
    // ────────────────────────────────────────────────────────────────
    let page_tools = Box::new(Orientation::Vertical, 18);
    page_tools.set_margin_top(28); page_tools.set_margin_bottom(28);
    page_tools.set_margin_start(30); page_tools.set_margin_end(30);
    
    let tools_title = Label::new(Some("Ferramentas Adicionais"));
    tools_title.add_css_class("section-title");
    tools_title.set_halign(gtk::Align::Start);
    page_tools.append(&tools_title);
    
    let tools_desc = Label::new(Some(&t("ferramentas_desc")));
    tools_desc.add_css_class("muted-label");
    tools_desc.set_halign(gtk::Align::Start);
    page_tools.append(&tools_desc);

    let btn_font_tool = Button::new();
    let box_font = Box::new(Orientation::Horizontal, 10);
    box_font.set_halign(gtk::Align::Center);
    box_font.set_valign(gtk::Align::Center);
    let img_font = Image::from_file("assets/font_icon.svg");
    img_font.set_pixel_size(18);
    img_font.add_css_class("font-btn-icon-outline");
    
    let css_icon = "
        .font-btn-icon-outline {
            filter: drop-shadow(1px 1px 0px rgba(0,0,0,1)) drop-shadow(-1px -1px 0px rgba(0,0,0,1));
        }
        .font-pic-outline {
            filter: drop-shadow(1px 1px 0px rgba(0,0,0,1)) drop-shadow(-1px -1px 0px rgba(0,0,0,1)) drop-shadow(1px -1px 0px rgba(0,0,0,1)) drop-shadow(-1px 1px 0px rgba(0,0,0,1));
        }
    ";
    let provider = gtk::CssProvider::new();
    provider.load_from_data(css_icon);
    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().unwrap(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION
    );
    
    let lbl_font = Label::new(Some(&t("btn_font")));
    lbl_font.add_css_class("font-btn-icon-outline");
    box_font.append(&img_font);
    box_font.append(&lbl_font);
    btn_font_tool.set_child(Some(&box_font));
    
    btn_font_tool.add_css_class("btn-save-config"); // reuse a nice big button class
    btn_font_tool.set_size_request(400, 42);
    btn_font_tool.set_halign(gtk::Align::Start);
    page_tools.append(&btn_font_tool);

    let pasta_entry_clone = path_entry.clone();
    let em_clone = engine_mode.clone();
    let main_window_clone = window.clone();
    let lang_font = lang.clone();
    btn_font_tool.connect_clicked(move |_| {
        crate::font_injector::show_font_window(&main_window_clone, pasta_entry_clone.text().to_string(), *em_clone.borrow(), lang_font.clone());
    });

    stack.add_named(&page_tools, Some("tools"));

    root.append(&stack);
    window.set_child(Some(&root));

    // ════════════════════════════════════════════════════════════════
    // CALLBACKS
    // ════════════════════════════════════════════════════════════════

    // Toolbar switching
    macro_rules! connect_toolbar {
        ($btn:expr, $page:expr, $tb:expr, $tbc:expr, $tbd:expr, $tbe:expr) => {{
            let sc = stack.clone();
            let a = $tb.clone(); let b = $tbc.clone(); let c = $tbd.clone(); let d = $tbe.clone();
            $btn.connect_clicked(move |btn| {
                sc.set_visible_child_name($page);
                btn.add_css_class("active");
                b.remove_css_class("active"); c.remove_css_class("active"); d.remove_css_class("active");
            });
        }};
    }
    connect_toolbar!(tb_translate, "translate", tb_translate, tb_logs, tb_tools, tb_settings);
    connect_toolbar!(tb_logs,      "logs",      tb_logs, tb_translate, tb_tools, tb_settings);
    connect_toolbar!(tb_tools,     "tools",     tb_tools, tb_translate, tb_logs, tb_settings);
    connect_toolbar!(tb_settings,  "settings",  tb_settings, tb_translate, tb_logs, tb_tools);

    {
        let em = engine_mode.clone();
        let br = btn_renpy_tab.clone(); let bu = btn_unity_tab.clone();
        let bt = btn_translate.clone();
        let pe = path_entry.clone();
        let cfg_r = cfg_rc.clone();
        let tl = title_line.clone(); let al = app_title_lbl.clone();
        let pasta_entry = pasta_entry.clone();
        let lbl_pasta = lbl_pasta_clone.clone();
        let btn_inject = btn_inject.clone();
        let chk_renpy_inject = chk_renpy_inject.clone();
        btn_renpy_tab.connect_clicked(move |btn| {
            if *em.borrow() == 1 {
                let mut c = cfg_r.borrow_mut();
                c.caminho_jogo_unity = pe.text().to_string();
                pe.set_text(&c.caminho_jogo_renpy);
            }
            *em.borrow_mut() = 0;
            btn.remove_css_class("active-unity"); btn.add_css_class("active-renpy");
            bu.remove_css_class("active-renpy"); bu.remove_css_class("active-unity");
            bt.remove_css_class("btn-translate-unity"); bt.add_css_class("btn-translate-renpy");
            bt.set_label("INICIAR TRADUÇÃO REN'PY");
            btn_inject.remove_css_class("btn-translate-unity"); btn_inject.add_css_class("btn-translate-renpy");
            let show_inject = chk_renpy_inject.is_active();
            btn_inject.set_visible(show_inject);
            pasta_entry.set_sensitive(true);
            lbl_pasta.set_sensitive(true);
            tl.remove_css_class("unity-mode"); al.remove_css_class("unity-mode");
        });
    }
    {
        let em = engine_mode.clone();
        let br = btn_renpy_tab.clone(); let bu = btn_unity_tab.clone();
        let bt = btn_translate.clone();
        let pe = path_entry.clone();
        let cfg_r = cfg_rc.clone();
        let tl = title_line.clone(); let al = app_title_lbl.clone();
        let pasta_entry = pasta_entry.clone();
        let lbl_pasta = lbl_pasta_clone.clone();
        let btn_inject = btn_inject.clone();
        btn_unity_tab.connect_clicked(move |btn| {
            if *em.borrow() == 0 {
                let mut c = cfg_r.borrow_mut();
                c.caminho_jogo_renpy = pe.text().to_string();
                pe.set_text(&c.caminho_jogo_unity);
            }
            *em.borrow_mut() = 1;
            btn.remove_css_class("active-renpy"); btn.add_css_class("active-unity");
            br.remove_css_class("active-renpy"); br.remove_css_class("active-unity");
            bt.remove_css_class("btn-translate-renpy"); bt.add_css_class("btn-translate-unity");
            bt.set_label("EXTRAIR TEXTOS UNITY");
            btn_inject.remove_css_class("btn-translate-renpy"); btn_inject.add_css_class("btn-translate-unity");
            btn_inject.set_visible(true); // Always visible in Unity
            pasta_entry.set_sensitive(false);
            lbl_pasta.set_sensitive(false);
            tl.add_css_class("unity-mode"); al.add_css_class("unity-mode");
        });
    }
    
    {
        let btn_inject = btn_inject.clone();
        let em = engine_mode.clone();
        chk_renpy_inject.connect_toggled(move |chk| {
            if *em.borrow() == 1 { // Unity mode: always visible, but we shouldn't change it here unless...
                // Actually, if we are in unity mode, changing the checkbox shouldn't affect the button
            } else {
                btn_inject.set_visible(chk.is_active());
            }
        });
    }

    // Init to saved mode
    if cfg.modo_jogo == "unity" { btn_unity_tab.emit_clicked(); }

    // Toggle callbacks
    { let tl = threads_lbl.clone(); let te = threads_entry.clone();
      tg_multi.connect_toggled(move |btn| {
          let on = btn.is_active(); btn.set_label(if on { "ON" } else { "OFF" });
          tl.set_sensitive(on); te.set_sensitive(on);
      }); }
    tg_struct.connect_toggled(|btn| btn.set_label(if btn.is_active() { "ON" } else { "OFF" }));
    tg_tradnomes.connect_toggled(|btn| btn.set_label(if btn.is_active() { "ON" } else { "OFF" }));

    // File picker
    { let pe = path_entry.clone(); let dl = detected_lbl.clone(); let wc = window.clone();
      btn_browse.connect_clicked(move |_| {
          let dialog = gtk::FileChooserNative::new(
              Some("Selecionar Executável"), Some(&wc),
              gtk::FileChooserAction::Open, Some("Abrir"), Some("Cancelar"),
          );
          let pe2 = pe.clone(); let dl2 = dl.clone();
          dialog.connect_response(move |d, resp| {
              if resp == gtk::ResponseType::Accept {
                  if let Some(f) = d.file().and_then(|f| f.path()) {
                      let s = f.to_string_lossy().to_string();
                      pe2.set_text(&s);
                      let has_game = f.parent().map(|p| p.join("game").is_dir()).unwrap_or(false);
                      let low = s.to_lowercase();
                      if has_game || low.ends_with(".py") { dl2.set_text("Ren'Py"); }
                      else if low.ends_with(".exe") { 
                          let backend = crate::unity_extractor::detect_unity_backend(&s).unwrap_or("Desconhecido");
                          dl2.set_text(&format!("Unity ({})", backend)); 
                      }
                      else { dl2.set_text("Não identificado"); }
                  }
              }
          });
          dialog.show();
      }); }

    // Save settings
    { let cfg_r = cfg_rc.clone();
      let cm = combo_motor.clone(); let tm = tg_multi.clone();
      let ts = tg_struct.clone(); let tn = tg_tradnomes.clone();
      let te = threads_entry.clone();
      let cl = combo_ui_lang.clone();
      let lb = log_buffer_geral.clone(); let sc = stack.clone();
      let tbt = tb_translate.clone(); let tbl = tb_logs.clone(); let tbs = tb_settings.clone();
      btn_save.connect_clicked(move |_| {
          let mut c = cfg_r.borrow_mut();
          c.motor_api = cm.active_text().map(|s| s.to_string()).unwrap_or_default();
          c.usar_multi_trad = tm.is_active();
          c.manter_estrutura_original = ts.is_active();
          c.preservar_nomes_renpy = true;
          c.traduzir_nomes_personagens_renpy = tn.is_active();
          c.threads_ativas = te.text().parse().unwrap_or(5);
          
          let new_lang = if cl.active() == Some(1) { "en_US".to_string() } else { "pt_BR".to_string() };
          let changed_lang = c.ui_language != new_lang;
          c.ui_language = new_lang;

          c.salvar();
          append_log(&lb, "[Config] Preferências salvas.");
          if changed_lang {
              append_log(&lb, "[Config] O idioma da interface foi alterado. Por favor, reinicie o aplicativo.");
          }
          append_log(&lb, &format!("[Config] Arquivo: {}", AppConfig::config_path_str()));
          sc.set_visible_child_name("logs");
          tbl.add_css_class("active"); tbt.remove_css_class("active"); tbs.remove_css_class("active");
      }); }

    // Cancel
    { let cc = cancelled.clone();
      btn_cancel.connect_clicked(move |btn| { cc.store(true, Ordering::SeqCst); btn.set_sensitive(false); }); }

    // Translate
    { let pe = path_entry.clone(); let pa = pasta_entry.clone();
      let co = combo_origem.clone(); let ca = combo_alvo.clone();
      let cm = combo_motor.clone();
      let ts = tg_struct.clone(); let tn = tg_tradnomes.clone(); let te = threads_entry.clone();
      let a_stack = action_stack.clone();
      let pb = progress_bar.clone(); let pl = progress_lbl.clone();
      let log_notebook = log_notebook.clone(); let lb_geral = log_buffer_geral.clone(); let cc = cancelled.clone();
      let cfg_r = cfg_rc.clone(); let em = engine_mode.clone();
      let sc = stack.clone();
      let tbt = tb_translate.clone(); let tbl = tb_logs.clone(); let tbs = tb_settings.clone();
      let app_window = window.clone();

      {
          let app_clone = app.clone();
          let pe = path_entry.clone();
          let pa = pasta_entry.clone();
          let ca = combo_alvo.clone();
          let em = engine_mode.clone();
          btn_editor.connect_clicked(move |_| {
              let exe_path = pe.text().to_string();
              let is_renpy = *em.borrow() == 0;
              let pasta_str = pa.text().to_string();
              let alvo = ca.active_text().map(|s| s.to_string()).unwrap_or_else(|| "Português".into());

              let exe = std::path::Path::new(&exe_path);
              if let Some(parent) = exe.parent() {
                  let target_dir = if is_renpy {
                      parent.join("game").join("tl").join(&pasta_str)
                  } else {
                      crate::unity_extractor::output_folder(exe.to_str().unwrap_or(""), &alvo)
                  };
                  crate::editor_ui::show_editor(&app_clone, target_dir);
              }
          });
      }

      let t_for_translate = t.clone();

      btn_translate.connect_clicked(move |_| {
          let exe = pe.text().to_string();
          if exe.is_empty() {
              append_log(&lb_geral, &t_for_translate("erro_sem_pasta"));
              sc.set_visible_child_name("logs");
              tbl.add_css_class("active"); tbt.remove_css_class("active"); tbs.remove_css_class("active");
              return;
          }
          let is_renpy = *em.borrow() == 0;
          let pasta = pa.text().to_string();
          let origem = co.active_text().map(|s| s.to_string()).unwrap_or_else(|| "auto".into());
          let alvo = ca.active_text().map(|s| s.to_string()).unwrap_or_else(|| "Português".into());
          let motor = cm.active_text().map(|s| s.to_string()).unwrap_or_else(|| "Google Translator".into());
          let keep_struct = ts.is_active();
          let trad_nomes = tn.is_active();
          let threads: u32 = te.text().parse().unwrap_or(5);

          { let mut c = cfg_r.borrow_mut();
            if is_renpy {
                c.caminho_jogo_renpy = exe.clone();
            } else {
                c.caminho_jogo_unity = exe.clone();
            }
            c.pasta_traducao = pasta.clone();
            c.idioma_origem = origem.clone(); c.idioma_alvo = alvo.clone();
            c.motor_api = motor.clone();
            c.modo_jogo = if is_renpy { "renpy" } else { "unity" }.into();
            c.manter_estrutura_original = keep_struct;
            c.traduzir_nomes_personagens_renpy = trad_nomes;
            c.threads_ativas = threads;
            c.salvar(); }

          let out_dir = if is_renpy {
              std::path::Path::new(&exe).parent().unwrap_or(std::path::Path::new(".")).join("game/tl").join(&pasta)
          } else {
              crate::unity_extractor::output_folder(&exe, &alvo)
          };
          
          let game_name = Path::new(&exe).file_name().unwrap_or_default().to_string_lossy().to_string();

          let start_extraction = Rc::new({
              let a_stack = a_stack.clone();
              let pb = pb.clone();
              let cc = cc.clone();
              let lb_g = lb_geral.clone();
              let pl = pl.clone();
              let exe = exe.clone();
              let pasta = pasta.clone();
              let origem = origem.clone();
              let alvo = alvo.clone();
              let motor = motor.clone();
              let app_window = app_window.clone();
              let log_notebook = log_notebook.clone();
              let game_name = game_name.clone();
              
              move |overwrite: bool| {
                  let mut existing_buffer: Option<TextBuffer> = None;
                  for i in 0..log_notebook.n_pages() {
                      if let Some(page) = log_notebook.nth_page(Some(i)) {
                          if log_tab_title(&log_notebook, &page).as_deref() == Some(&game_name) {
                              if let Some(scroll) = page.downcast::<ScrolledWindow>().ok() {
                                  if let Some(tv) = scroll.child().and_then(|w| w.downcast::<TextView>().ok()) {
                                      existing_buffer = Some(tv.buffer());
                                      log_notebook.set_current_page(Some(i));
                                      break;
                                  }
                              }
                          }
                      }
                  }

                  let lb_jogo = if let Some(buf) = existing_buffer {
                      buf.set_text(""); // Clear existing log
                      buf
                  } else {
                      let buf = TextBuffer::new(None);
                      let tv = TextView::with_buffer(&buf);
                      tv.add_css_class("log-view");
                      tv.set_editable(false);
                      tv.set_wrap_mode(gtk::WrapMode::WordChar);
                      tv.set_cursor_visible(false);
                      let scroll = ScrolledWindow::new();
                      scroll.set_child(Some(&tv));
                      append_log_tab(&log_notebook, &scroll, &game_name, true);
                      log_notebook.set_current_page(Some(log_notebook.n_pages() - 1));
                      buf
                  };

                  let lb = lb_jogo.clone();
                  a_stack.set_visible_child_name("progress");
                  pb.set_fraction(0.0);
                  cc.store(false, Ordering::SeqCst);
                  append_log(&lb, &format!("Iniciando extração {}...", if is_renpy { "Ren'Py" } else { "Unity" }));
                  append_log(&lb_g, &format!("Iniciando extração {}...", if is_renpy { "Ren'Py" } else { "Unity" }));

                  #[allow(deprecated)]
                  let (tx, rx) = gtk::glib::MainContext::channel(gtk::glib::Priority::DEFAULT);
                  let tx2 = tx.clone(); let cc2 = cc.clone();
                  let exe2 = exe.clone(); let pasta2 = pasta.clone(); let origem2 = origem.clone(); let alvo2 = alvo.clone(); let motor2 = motor.clone();
                  std::thread::spawn(move || {
                      let rt = tokio::runtime::Runtime::new().unwrap();
                      rt.block_on(async {
                          let res = if is_renpy {
                              crate::renpy_extractor::extract_texts(
                                  &exe2, &pasta2, &origem2, &alvo2, keep_struct, trad_nomes, threads, &motor2,
                                  tx2.clone(), cc2, overwrite).await
                          } else {
                              crate::unity_extractor::extract_texts(
                                  &exe2, &pasta2, &origem2, &alvo2, threads, &motor2, tx2.clone(), cc2, overwrite).await
                          };
                          let _ = tx2.send(UiMsg::Done(match res {
                              Ok(_) => "✅ Operação concluída com sucesso!".into(),
                              Err(e) => format!("❌ ERRO: {}", e),
                          }));
                      });
                  });

                  let pb2 = pb.clone(); let pl2 = pl.clone();
                  let lb2 = lb.clone(); let lbg2 = lb_g.clone(); let as2 = a_stack.clone();
                  let window2 = app_window.clone();
                  rx.attach(None, move |msg| {
                      match msg {
                          UiMsg::Log(t)  => { append_log(&lb2, &t); }
                          UiMsg::Progress(c, t) => {
                              if t > 0 { pb2.set_fraction(c as f64 / t as f64); }
                              pl2.set_text(&format!("{} / {} extraídos", c, t));
                          }
                          UiMsg::Done(m) => {
                              append_log(&lb2, &m);
                              append_log(&lbg2, &m);
                              as2.set_visible_child_name("buttons");
                              let is_err = m.starts_with('❌');
                              let dialog = gtk::MessageDialog::builder()
                                  .transient_for(&window2)
                                  .modal(true)
                                  .message_type(if is_err { gtk::MessageType::Error } else { gtk::MessageType::Info })
                                  .buttons(gtk::ButtonsType::Ok)
                                  .text(if is_err { "Erro na Tradução" } else { "Tradução Concluída" })
                                  .secondary_text(if is_err { &m } else { "A extração e a tradução foram concluídas com sucesso!" })
                                  .build();
                              dialog.connect_response(|d, _| d.close());
                              dialog.show();
                              return gtk::glib::ControlFlow::Break;
                          }
                      }
                      gtk::glib::ControlFlow::Continue
                  });
              }
          });

          if out_dir.exists() {
              let dialog = gtk::MessageDialog::builder()
                  .transient_for(&app_window)
                  .modal(true)
                  .message_type(gtk::MessageType::Question)
                  .text("Tradução Antiga Detectada")
                  .secondary_text(&format!("Uma tradução antiga foi detectada em:\n{}\n\nDeseja apagá-la completamente e iniciar uma nova do zero, ou apenas atualizar mantendo os arquivos antigos?", out_dir.display()))
                  .build();
              
              dialog.add_button("Sobrescrever", gtk::ResponseType::Yes);
              dialog.add_button("Atualizar", gtk::ResponseType::Apply);
              dialog.add_button("Cancelar", gtk::ResponseType::Cancel);

              let se = start_extraction.clone();
              dialog.connect_response(move |d, response| {
                  d.close();
                  match response {
                      gtk::ResponseType::Yes => se(true),
                      gtk::ResponseType::Apply => se(false),
                      _ => {}
                  }
              });
              dialog.show();
          } else {
              start_extraction(true);
          }
      }); }

      {
          let pe = path_entry.clone();
          let a_stack = action_stack.clone();
          let pb = progress_bar.clone();
          let pl = progress_lbl.clone();
          let log_notebook = log_notebook.clone();
          let lb_geral = log_buffer_geral.clone();
          let cc = cancelled.clone();
          let em = engine_mode.clone();
          let sc = stack.clone();
          let tbt = tb_translate.clone(); let tbl = tb_logs.clone(); let tbs = tb_settings.clone();
          let app_window = window.clone();
          let combo_alvo = combo_alvo.clone();
          let t_for_inject = t.clone();
          
          btn_inject.connect_clicked(move |_| {
              let exe = pe.text().to_string();
              if exe.is_empty() {
                  append_log(&lb_geral, &t_for_inject("erro_sem_pasta"));
                  sc.set_visible_child_name("logs");
                  tbl.add_css_class("active"); tbt.remove_css_class("active"); tbs.remove_css_class("active");
                  return;
              }
              let is_renpy = *em.borrow() == 0;
              let alvo = combo_alvo.active_text().unwrap_or_else(|| "pt".into()).to_string();
              let game_name = Path::new(&exe).file_name().unwrap_or_default().to_string_lossy().to_string();
              
              let tab_title = format!("{} (Injeção)", game_name);
              let mut existing_buffer: Option<TextBuffer> = None;
              for i in 0..log_notebook.n_pages() {
                  if let Some(page) = log_notebook.nth_page(Some(i)) {
                      if log_tab_title(&log_notebook, &page).as_deref() == Some(&tab_title) {
                          if let Some(scroll) = page.downcast::<ScrolledWindow>().ok() {
                              if let Some(tv) = scroll.child().and_then(|w| w.downcast::<TextView>().ok()) {
                                  existing_buffer = Some(tv.buffer());
                                  log_notebook.set_current_page(Some(i));
                                  break;
                              }
                          }
                      }
                  }
              }

              let lb_jogo = if let Some(buf) = existing_buffer {
                  buf.set_text(""); // Clear existing log
                  buf
              } else {
                  let buf = TextBuffer::new(None);
                  let tv = TextView::with_buffer(&buf);
                  tv.add_css_class("log-view");
                  tv.set_editable(false);
                  tv.set_wrap_mode(gtk::WrapMode::WordChar);
                  tv.set_cursor_visible(false);
                  let scroll = ScrolledWindow::new();
                  scroll.set_child(Some(&tv));
                  append_log_tab(&log_notebook, &scroll, &tab_title, true);
                  log_notebook.set_current_page(Some(log_notebook.n_pages() - 1));
                  buf
              };
              
              a_stack.set_visible_child_name("progress");
              pb.set_fraction(1.0); 
              pl.set_text("Injetando...");
              cc.store(false, Ordering::SeqCst);
              
              append_log(&lb_jogo, &format!("Iniciando injeção {}...", if is_renpy { "Ren'Py" } else { "Unity" }));
              append_log(&lb_geral, &format!("Iniciando injeção {}...", if is_renpy { "Ren'Py" } else { "Unity" }));
              
              #[allow(deprecated)]
              let (tx, rx) = gtk::glib::MainContext::channel(gtk::glib::Priority::DEFAULT);
              
              std::thread::spawn(move || {
                  let rt = tokio::runtime::Runtime::new().unwrap();
                  rt.block_on(async {
                      let res = if is_renpy {
                          Err("Injeção manual separada para Ren'Py ainda não implementada. Use 'Iniciar Tradução'.".into())
                      } else {
                          crate::unity_extractor::inject_texts(&exe, "", &alvo, tx.clone()).await
                      };
                      let _ = tx.send(UiMsg::Done(match res {
                          Ok(_) => "✅ Operação concluída com sucesso!".into(),
                          Err(e) => format!("❌ ERRO: {}", e),
                      }));
                  });
              });
              
              let pb2 = pb.clone(); let pl2 = pl.clone();
              let lb2 = lb_jogo.clone(); let lbg2 = lb_geral.clone(); let as2 = a_stack.clone();
              let window2 = app_window.clone();
              
              rx.attach(None, move |msg| {
                  match msg {
                      UiMsg::Log(t)  => { append_log(&lb2, &t); }
                      UiMsg::Progress(_, _) => {}
                      UiMsg::Done(m) => {
                          append_log(&lb2, &m);
                          append_log(&lbg2, &m);
                          as2.set_visible_child_name("buttons");
                          let is_err = m.starts_with('❌');
                          let dialog = gtk::MessageDialog::builder()
                              .transient_for(&window2)
                              .modal(true)
                              .message_type(if is_err { gtk::MessageType::Error } else { gtk::MessageType::Info })
                              .buttons(gtk::ButtonsType::Ok)
                              .text(if is_err { "Erro na Injeção" } else { "Injeção Concluída" })
                              .secondary_text(if is_err { &m } else { "A injeção de textos foi concluída com sucesso!" })
                              .build();
                          dialog.connect_response(|d, _| d.close());
                          dialog.show();
                          return gtk::glib::ControlFlow::Break;
                      }
                  }
                  gtk::glib::ControlFlow::Continue
              });
          });
      }

    window.present();
}
