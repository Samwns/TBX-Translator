use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box, Button, Label, ListBox, ListBoxRow,
    Orientation, ScrolledWindow, SearchEntry, Separator, TextView, WrapMode,
};
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use walkdir::WalkDir;

#[derive(Clone)]
struct DialogData {
    original: String,
    translated: String,
}

pub fn show_editor(app: &Application, folder: PathBuf) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("TBX Translator - Editor Manual")
        .default_width(900)
        .default_height(600)
        .build();
    crate::ui::apply_windows_native_styling(&window);
    window.add_css_class("editor-window");

    let main_box = Box::new(Orientation::Horizontal, 0);

    // Left sidebar: file list
    let sidebar = Box::new(Orientation::Vertical, 0);
    sidebar.set_size_request(250, -1);
    sidebar.add_css_class("sidebar");

    let sidebar_title = Label::new(Some("Arquivos Traduzidos"));
    sidebar_title.set_margin_top(10);
    sidebar_title.set_margin_bottom(10);
    sidebar_title.add_css_class("section-label-purple");
    sidebar.append(&sidebar_title);

    let file_list = ListBox::new();
    file_list.set_selection_mode(gtk::SelectionMode::Single);
    let file_scroll = ScrolledWindow::new();
    file_scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    file_scroll.set_child(Some(&file_list));
    file_scroll.set_vexpand(true);
    sidebar.append(&file_scroll);

    main_box.append(&sidebar);
    main_box.append(&Separator::new(Orientation::Vertical));

    // Right content: editor
    let content_box = Box::new(Orientation::Vertical, 10);
    content_box.set_hexpand(true);
    content_box.set_margin_top(10);
    content_box.set_margin_bottom(10);
    content_box.set_margin_start(10);
    content_box.set_margin_end(10);

    let search_bar = SearchEntry::new();
    search_bar.set_placeholder_text(Some("Pesquisar original ou tradução..."));
    
    let btn_save = Button::with_label("Salvar Arquivo");
    btn_save.add_css_class("btn-translate-renpy");

    let top_bar = Box::new(Orientation::Horizontal, 10);
    top_bar.append(&search_bar);
    top_bar.append(&btn_save);
    search_bar.set_hexpand(true);

    content_box.append(&top_bar);

    let dialogs_list = ListBox::new();
    dialogs_list.set_selection_mode(gtk::SelectionMode::None);
    let dialogs_scroll = ScrolledWindow::new();
    dialogs_scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    dialogs_scroll.set_child(Some(&dialogs_list));
    dialogs_scroll.set_vexpand(true);
    content_box.append(&dialogs_scroll);

    main_box.append(&content_box);
    window.set_child(Some(&main_box));

    // State
    let current_file = Rc::new(RefCell::new(None::<PathBuf>));
    let dialogs_data = Rc::new(RefCell::new(Vec::<Rc<RefCell<DialogData>>>::new()));

    // Load files
    for entry in WalkDir::new(&folder).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "rpy" || ext == "txt" || ext == "json" {
                    let row = ListBoxRow::new();
                    let lbl = Label::new(Some(&path.file_name().unwrap().to_string_lossy()));
                    lbl.set_halign(gtk::Align::Start);
                    lbl.set_margin_start(5);
                    lbl.set_margin_end(5);
                    lbl.set_margin_top(5);
                    lbl.set_margin_bottom(5);
                    row.set_child(Some(&lbl));
                    // attach path to row
                    unsafe {
                        row.set_data("path", path.to_path_buf());
                    }
                    file_list.append(&row);
                }
            }
        }
    }

    let dialogs_list_clone = dialogs_list.clone();
    let current_file_clone = current_file.clone();
    let dialogs_data_clone = dialogs_data.clone();

    file_list.connect_row_selected(move |_, row| {
        if let Some(row) = row {
            unsafe {
                if let Some(path_ptr) = row.data::<PathBuf>("path") {
                    let path = path_ptr.as_ref().clone();
                    *current_file_clone.borrow_mut() = Some(path.clone());
                    
                    // clear list
                    while let Some(child) = dialogs_list_clone.first_child() {
                        dialogs_list_clone.remove(&child);
                    }
                    dialogs_data_clone.borrow_mut().clear();

                    load_file(&path, &dialogs_list_clone, &dialogs_data_clone);
                }
            }
        }
    });

    let dialogs_list_search = dialogs_list.clone();
    search_bar.connect_search_changed(move |entry| {
        let text = entry.text().to_string().to_lowercase();
        
        let mut child = dialogs_list_search.first_child();
        while let Some(c) = child {
            if let Some(row) = c.downcast_ref::<ListBoxRow>() {
                unsafe {
                    if let Some(data_ptr) = row.data::<Rc<RefCell<DialogData>>>("data") {
                        let data = data_ptr.as_ref().borrow();
                        let show = text.is_empty() 
                            || data.original.to_lowercase().contains(&text)
                            || data.translated.to_lowercase().contains(&text);
                        row.set_visible(show);
                    }
                }
            }
            child = c.next_sibling();
        }
    });

    let current_file_save = current_file.clone();
    let dialogs_data_save = dialogs_data.clone();
    btn_save.connect_clicked(move |_| {
        if let Some(path) = current_file_save.borrow().as_ref() {
            save_file(path, &dialogs_data_save.borrow());
            
            // show success notification
            // simple label change trick
        }
    });

    window.show();
}

fn load_file(path: &Path, listbox: &ListBox, data_store: &Rc<RefCell<Vec<Rc<RefCell<DialogData>>>>>) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let ext = path.extension().unwrap_or_default();
    if ext == "rpy" {
        let mut last_old: Option<String> = None;
        for line in content.lines() {
            let t = line.trim();
            if t.starts_with("old \"") {
                if let Some(start) = t.find('"') {
                    if let Some(end) = t.rfind('"') {
                        if start != end {
                            last_old = Some(t[start + 1..end].to_string());
                        }
                    }
                }
            } else if t.starts_with("new \"") {
                if let Some(old) = last_old.take() {
                    if let Some(start) = t.find('"') {
                        if let Some(end) = t.rfind('"') {
                            if start != end {
                                let new_txt = t[start + 1..end].to_string();
                                add_dialog_row(old, new_txt, listbox, data_store);
                            }
                        }
                    }
                }
            }
        }
    } else if ext == "txt" {
        for line in content.lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with("//") || t.starts_with('#') { continue; }
            if let Some(idx) = find_unescaped_equals(t) {
                let orig = unescape_xunity(&t[..idx]);
                let trad = unescape_xunity(&t[idx + 1..]);
                add_dialog_row(orig, trad, listbox, data_store);
            }
        }
    } else if ext == "json" {
        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(obj) = json_val.as_object() {
                for (orig, trad_val) in obj {
                    if let Some(trad) = trad_val.as_str() {
                        add_dialog_row(orig.clone(), trad.to_string(), listbox, data_store);
                    }
                }
            } else if let Some(arr) = json_val.as_array() {
                for item in arr {
                    if let Some(orig) = item.as_str() {
                        add_dialog_row(orig.to_string(), orig.to_string(), listbox, data_store);
                    }
                }
            }
        }
    }
}

fn add_dialog_row(orig: String, trad: String, listbox: &ListBox, data_store: &Rc<RefCell<Vec<Rc<RefCell<DialogData>>>>>) {
    let row = ListBoxRow::new();
    let vbox = Box::new(Orientation::Vertical, 5);
    vbox.set_margin_start(10);
    vbox.set_margin_end(10);
    vbox.set_margin_top(5);
    vbox.set_margin_bottom(5);

    let lbl = Label::new(Some(&orig));
    lbl.set_halign(gtk::Align::Start);
    lbl.set_wrap(true);
    lbl.set_selectable(true);
    lbl.set_tooltip_text(Some("Texto original — selecione para copiar"));
    vbox.append(&lbl);

    let translated_view = TextView::new();
    translated_view.set_wrap_mode(WrapMode::WordChar);
    translated_view.set_accepts_tab(false);
    translated_view.set_monospace(false);
    let translated_buffer = translated_view.buffer();
    translated_buffer.set_text(&trad);
    let translated_scroll = ScrolledWindow::new();
    translated_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    translated_scroll.set_child(Some(&translated_view));
    let resize_editor: Rc<dyn Fn(&str)> = Rc::new({
        let view = translated_view.clone();
        let scroll = translated_scroll.clone();
        move |text: &str| {
            // Approximate wrapped visual lines at the editor's normal width.
            // Keep short translations compact, while preserving a sensible
            // maximum before the inner scrollbar takes over.
            let lines: usize = text.lines()
                .map(|line| usize::max(1, (line.chars().count() + 74) / 75))
                .sum::<usize>()
                .max(1);
            let height = (lines.min(8) as i32 * 24 + 16).clamp(40, 208);
            view.set_size_request(-1, height);
            scroll.set_min_content_height(height);
        }
    });
    resize_editor(&trad);
    vbox.append(&translated_scroll);
    
    let data = Rc::new(RefCell::new(DialogData { original: orig, translated: trad }));
    data_store.borrow_mut().push(data.clone());
    
    let data_clone = data.clone();
    let resize_editor_clone = resize_editor.clone();
    translated_buffer.connect_changed(move |buffer| {
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        let text = buffer.text(&start, &end, false).to_string();
        resize_editor_clone(&text);
        data_clone.borrow_mut().translated = text;
    });

    unsafe {
        row.set_data("data", data);
    }

    row.set_child(Some(&vbox));
    listbox.append(&row);
}

fn save_file(path: &Path, data: &[Rc<RefCell<DialogData>>]) {
    let ext = path.extension().unwrap_or_default();
    let mut out = String::new();
    
    if ext == "rpy" {
        let parent_name = path.parent().and_then(|p| p.file_name()).unwrap_or_default().to_string_lossy();
        out.push_str(&format!("translate {} strings:\n\n", parent_name));
        for d in data {
            let d = d.borrow();
            out.push_str(&format!("    old \"{}\"\n", d.original.replace('"', "\\\"")));
            out.push_str(&format!("    new \"{}\"\n\n", d.translated.replace('"', "\\\"")));
        }
    } else if ext == "txt" {
        out.push_str("// Gerado pelo TBX Translator\n");
        out.push_str("// Formato compativel com XUnity.AutoTranslator: original=traducao\n\n");
        for d in data {
            let d = d.borrow();
            out.push_str(&format!("{}={}\n", escape_xunity(&d.original), escape_xunity(&d.translated)));
        }
    } else if ext == "json" {
        let mut map = serde_json::Map::new();
        for d in data {
            let d = d.borrow();
            map.insert(d.original.clone(), serde_json::Value::String(d.translated.clone()));
        }
        let json_val = serde_json::Value::Object(map);
        if let Ok(pretty) = serde_json::to_string_pretty(&json_val) {
            out.push_str(&pretty);
        }
    }
    
    let _ = fs::write(path, out);
}

fn find_unescaped_equals(s: &str) -> Option<usize> {
    let mut escaped = false;
    for (idx, c) in s.char_indices() {
        if c == '=' && !escaped {
            return Some(idx);
        }
        escaped = c == '\\' && !escaped;
        if c != '\\' {
            escaped = false;
        }
    }
    None
}

fn unescape_xunity(s: &str) -> String {
    let mut sb = String::new();
    let mut escaped = false;
    for c in s.chars() {
        if escaped {
            if c == 'n' { sb.push('\n'); }
            else { sb.push(c); }
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else {
            sb.push(c);
        }
    }
    if escaped { sb.push('\\'); }
    sb
}

fn escape_xunity(s: &str) -> String {
    s.replace('\\', r"\\")
     .replace('\r', "")
     .replace('\n', r"\n")
     .replace('=', r"\=")
}
