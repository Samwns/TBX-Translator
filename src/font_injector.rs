use gtk4 as gtk;
use gtk::prelude::*;
use std::path::{Path, PathBuf};
use std::fs;
use gtk::{prelude::*, Align, ApplicationWindow, Box, Button, Dialog, Entry, FileChooserNative, FileChooserAction, ResponseType, FileChooserDialog, Label, Orientation, TextView, CssProvider, Stack, ListBox, ListBoxRow, ScrolledWindow, Image, Picture};
use rusttype::{Font, Scale, point};
use std::rc::Rc;
use std::cell::RefCell;
use std::thread;
use serde_json;

pub fn show_font_window(parent: &gtk::ApplicationWindow, game_path: String, engine_mode: u32, lang: String) {
    if game_path.is_empty() {
        let dialog = gtk::MessageDialog::new(
            Some(parent),
            gtk::DialogFlags::MODAL,
            gtk::MessageType::Error,
            gtk::ButtonsType::Ok,
            crate::i18n::t("erro_sem_pasta", &lang)
        );
        dialog.connect_response(|d, _| d.destroy());
        dialog.show();
        return;
    }

    let win = ApplicationWindow::builder()
        .title(crate::i18n::t("janela_fonte_titulo", &lang))
        .default_width(600)
        .default_height(500)
        .modal(true)
        .transient_for(parent)
        .decorated(false)
        .build();
    crate::ui::apply_windows_native_styling(&win);
    win.add_css_class("main-transparent");
    
    let shell = Box::new(Orientation::Vertical, 0);
    shell.add_css_class("app-shell");

    // Title bar
    let title_bar = Box::new(Orientation::Horizontal, 12);
    title_bar.add_css_class("title-bar");

    let title_vbox = Box::new(Orientation::Vertical, 3);
    let app_title_lbl = Label::new(Some(&crate::i18n::t("janela_fonte_titulo", &lang)));
    app_title_lbl.add_css_class("app-title");
    app_title_lbl.set_halign(gtk::Align::Start);
    let title_line = Box::new(Orientation::Horizontal, 0);
    title_line.add_css_class("title-underline");
    title_line.set_size_request(130, 2);
    title_vbox.append(&app_title_lbl);
    title_vbox.append(&title_line);

    let spacer = Box::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);

    let win_btns = Box::new(Orientation::Horizontal, 2);
    let btn_min = Button::with_label("—");
    btn_min.add_css_class("btn-win"); btn_min.add_css_class("btn-win-min");
    let btn_close = Button::with_label("✕");
    btn_close.add_css_class("btn-win"); btn_close.add_css_class("btn-win-close");
    win_btns.append(&btn_min); win_btns.append(&btn_close);

    let win_min = win.clone();
    btn_min.connect_clicked(move |_| { win_min.minimize(); });
    
    let win_close = win.clone();
    btn_close.connect_clicked(move |_| { win_close.close(); });

    title_bar.append(&title_vbox);
    title_bar.append(&spacer);
    title_bar.append(&win_btns);

    let window_handle = gtk::WindowHandle::new();
    window_handle.set_child(Some(&title_bar));
    shell.append(&window_handle);

    let root = Box::new(Orientation::Vertical, 15);
    root.set_margin_top(20); root.set_margin_bottom(20);
    root.set_margin_start(20); root.set_margin_end(20);
    shell.append(&root);

    let engine_bar = Box::new(Orientation::Horizontal, 0);
    engine_bar.add_css_class("engine-bar");
    
    let mk_engine_btn = |label: &str, icon_path: &str| {
        let b = Button::new();
        let bx = Box::new(Orientation::Horizontal, 8);
        bx.set_halign(gtk::Align::Center);
        bx.set_valign(gtk::Align::Center);
        let img = gtk::Image::from_file(icon_path);
        img.set_pixel_size(18);
        let lbl = Label::new(Some(label));
        bx.append(&img);
        bx.append(&lbl);
        b.set_child(Some(&bx));
        b.add_css_class("game-tab-btn");
        b
    };
    let btn_renpy_tab = mk_engine_btn("Ren'Py", "assets/renpy_icon.svg");
    let btn_unity_tab = mk_engine_btn("Unity", "assets/unity_icon.svg");
    engine_bar.append(&btn_renpy_tab);
    engine_bar.append(&btn_unity_tab);
    root.append(&engine_bar);

    let stack = Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    root.append(&stack);

    // ==========================================
    // REN'PY PAGE
    // ==========================================
    let page_renpy = Box::new(Orientation::Vertical, 10);
    
    let btn_scan = Button::with_label("Escanear Fontes do Jogo");
    btn_scan.add_css_class("suggested-action");
    page_renpy.append(&btn_scan);
    
    let list_fonts = ListBox::new();
    list_fonts.set_selection_mode(gtk::SelectionMode::None);
    list_fonts.add_css_class("boxed-list");
    
    let scroll_renpy = ScrolledWindow::new();
    scroll_renpy.set_child(Some(&list_fonts));
    scroll_renpy.set_min_content_height(200);
    scroll_renpy.set_vexpand(true);
    page_renpy.append(&scroll_renpy);

    stack.add_named(&page_renpy, Some("renpy"));

    let win_scan = win.clone();
    let btn_scan_clone = btn_scan.clone();
    let game_path_renpy = game_path.clone();
    btn_scan.connect_clicked(move |_| {
        btn_scan_clone.set_sensitive(false);
        btn_scan_clone.set_label("Escaneando (Isso pode demorar alguns segundos)...");
        let list_fonts_inner = list_fonts.clone();
        let win_inner = win_scan.clone();
        let gp = game_path_renpy.clone();
        let gp_thread = game_path_renpy.clone();
        let b = btn_scan_clone.clone();

        let (sender, receiver) = gtk::glib::MainContext::channel(gtk::glib::Priority::DEFAULT);
        
        receiver.attach(
            None,
            move |fonts: Result<Vec<String>, String>| {
                b.set_sensitive(true);
                b.set_label("Escanear Novamente");

                while let Some(row) = list_fonts_inner.first_child() {
                    list_fonts_inner.remove(&row);
                }

                match fonts {
                    Ok(f_list) => {
                        if f_list.is_empty() {
                            let lbl = Label::new(Some("Nenhuma fonte encontrada."));
                            lbl.set_margin_top(10); lbl.set_margin_bottom(10);
                            list_fonts_inner.append(&lbl);
                        } else {
                            for f in f_list {
                                let row = create_font_row(&f, &gp, &win_inner);
                                list_fonts_inner.append(&row);
                            }
                        }
                    },
                    Err(e) => {
                        let dialog = gtk::MessageDialog::new(
                            Some(&win_inner),
                            gtk::DialogFlags::MODAL,
                            gtk::MessageType::Error,
                            gtk::ButtonsType::Ok,
                            &format!("Erro ao escanear fontes: {}", e)
                        );
                        dialog.connect_response(|d, _| d.destroy());
                        dialog.show();
                    }
                }
                gtk::glib::ControlFlow::Break
            }
        );

        thread::spawn(move || {
            let fonts = scan_renpy_fonts(&gp_thread);
            let _ = sender.send(fonts);
        });
    });

    // ==========================================
    // UNITY PAGE
    // ==========================================
    let page_unity = Box::new(Orientation::Vertical, 10);
    
    let btn_scan_unity = Button::with_label("Escanear Fontes do Jogo (Unity)");
    btn_scan_unity.add_css_class("suggested-action");
    page_unity.append(&btn_scan_unity);
    
    let list_fonts_unity = ListBox::new();
    list_fonts_unity.set_selection_mode(gtk::SelectionMode::None);
    list_fonts_unity.add_css_class("boxed-list");
    
    let scroll_unity = ScrolledWindow::new();
    scroll_unity.set_child(Some(&list_fonts_unity));
    scroll_unity.set_min_content_height(200);
    scroll_unity.set_vexpand(true);
    page_unity.append(&scroll_unity);

    stack.add_named(&page_unity, Some("unity"));

    let win_scan_unity = win.clone();
    let btn_scan_unity_clone = btn_scan_unity.clone();
    let game_path_unity = game_path.clone();
    btn_scan_unity.connect_clicked(move |_| {
        btn_scan_unity_clone.set_sensitive(false);
        btn_scan_unity_clone.set_label("Escaneando Fontes (Unity)...");
        let list_fonts_inner = list_fonts_unity.clone();
        let win_inner = win_scan_unity.clone();
        let gp = game_path_unity.clone();
        let gp_thread = game_path_unity.clone();
        let b = btn_scan_unity_clone.clone();

        let (sender, receiver) = gtk::glib::MainContext::channel(gtk::glib::Priority::DEFAULT);
        
        receiver.attach(
            None,
            move |fonts: Result<Vec<String>, String>| {
                b.set_sensitive(true);
                b.set_label("Escanear Novamente (Unity)");

                while let Some(row) = list_fonts_inner.first_child() {
                    list_fonts_inner.remove(&row);
                }

                match fonts {
                    Ok(f_list) => {
                        if f_list.is_empty() {
                            let lbl = Label::new(Some("Nenhuma fonte TTF/OTF embutida encontrada. Fontes TextMeshPro (SDF) não podem receber um .ttf diretamente; use a fonte original ou um asset bundle TMP."));
                            lbl.set_wrap(true);
                            lbl.set_margin_top(10); lbl.set_margin_bottom(10);
                            list_fonts_inner.append(&lbl);
                        } else {
                            for f in f_list {
                                let row = create_font_row_unity(&f, &gp, &win_inner);
                                list_fonts_inner.append(&row);
                            }
                        }
                    },
                    Err(e) => {
                        let dialog = gtk::MessageDialog::new(
                            Some(&win_inner),
                            gtk::DialogFlags::MODAL,
                            gtk::MessageType::Error,
                            gtk::ButtonsType::Ok,
                            &format!("Erro ao escanear fontes Unity: {}", e)
                        );
                        dialog.connect_response(|d, _| d.destroy());
                        dialog.show();
                    }
                }
                gtk::glib::ControlFlow::Break
            }
        );

        thread::spawn(move || {
            let fonts = scan_unity_fonts(&gp_thread);
            let _ = sender.send(fonts);
        });
    });

    // Connect Engine Buttons
    let stack_clone1 = stack.clone();
    let tu1 = btn_unity_tab.clone();
    btn_renpy_tab.connect_clicked(move |btn| {
        btn.add_css_class("active-renpy"); btn.remove_css_class("active-unity");
        tu1.remove_css_class("active-renpy"); tu1.remove_css_class("active-unity");
        stack_clone1.set_visible_child_name("renpy");
    });

    let stack_clone2 = stack.clone();
    let tr2 = btn_renpy_tab.clone();
    btn_unity_tab.connect_clicked(move |btn| {
        btn.remove_css_class("active-renpy"); btn.add_css_class("active-unity");
        tr2.remove_css_class("active-renpy"); tr2.remove_css_class("active-unity");
        stack_clone2.set_visible_child_name("unity");
    });

    if engine_mode == 0 {
        btn_renpy_tab.emit_clicked();
    } else {
        btn_unity_tab.emit_clicked();
    }

    win.set_child(Some(&shell));
    win.show();
}

fn create_font_row(font_internal_path: &str, game_path: &str, parent_win: &ApplicationWindow) -> ListBoxRow {
    let row = ListBoxRow::new();
    let bx_v = Box::new(Orientation::Vertical, 5);
    bx_v.set_margin_top(8); bx_v.set_margin_bottom(8);
    bx_v.set_margin_start(10); bx_v.set_margin_end(10);

    let lbl = Label::new(Some(font_internal_path));
    lbl.set_hexpand(true);
    lbl.set_halign(gtk::Align::Start);
    
    let btn_replace = Button::with_label("Substituir");
    
    let bx_h = Box::new(Orientation::Horizontal, 10);
    bx_h.append(&lbl);
    bx_h.append(&btn_replace);
    bx_v.append(&bx_h);

    let entry_test = Entry::new();
    entry_test.set_placeholder_text(Some("Digite aqui para testar a fonte original..."));
    entry_test.set_hexpand(true);
    
    let font_file_name = Path::new(font_internal_path).file_name().and_then(|s| s.to_str()).unwrap_or("");
    let mut base_dir = PathBuf::from(game_path);
    if base_dir.is_file() {
        if let Some(p) = base_dir.parent() {
            base_dir = p.to_path_buf();
        }
    }
    let dumped_font_path = base_dir.join("game").join("tpg_temp_fonts").join(font_file_name);
    
    if dumped_font_path.exists() {
        if let Ok(font_data) = std::fs::read(&dumped_font_path) {
            let pic = Picture::new();
            pic.set_halign(gtk::Align::Start);
            pic.set_margin_top(5);
            pic.add_css_class("font-pic-outline");
            
            let pic_clone = pic.clone();
            entry_test.connect_changed(move |e| {
                let text = e.text();
                if text.is_empty() {
                    pic_clone.set_paintable(None::<&gtk::gdk::Paintable>);
                    return;
                }
                
                if let Some(font) = Font::try_from_vec(font_data.clone()) {
                    let scale = Scale::uniform(32.0);
                    let v_metrics = font.v_metrics(scale);
                    
                    let glyphs: Vec<_> = font.layout(&text, scale, point(0.0, v_metrics.ascent)).collect();
                    let width = glyphs.iter().map(|g| g.position().x + g.unpositioned().h_metrics().advance_width).last().unwrap_or(0.0).ceil() as u32;
                    let height = (v_metrics.ascent - v_metrics.descent).ceil() as u32;
                    
                    println!("Font loaded correctly! Text length: {}, Width: {}, Height: {}", text.len(), width, height);
                    
                    let width = width.max(1);
                    let height = height.max(1);
                    
                    let mut img_data = vec![0u8; (width * height * 4) as usize];
                    let mut drew_anything = false;
                    for g in glyphs {
                        if let Some(bb) = g.pixel_bounding_box() {
                            g.draw(|x, y, v| {
                                let px = x as i32 + bb.min.x;
                                let py = y as i32 + bb.min.y;
                                if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                                    let idx = ((py * width as i32 + px) * 4) as usize;
                                    let alpha = (v * 255.0) as u8;
                                    if alpha > 0 { drew_anything = true; }
                                    img_data[idx] = 255;
                                    img_data[idx + 1] = 255;
                                    img_data[idx + 2] = 255;
                                    img_data[idx + 3] = img_data[idx + 3].max(alpha);
                                }
                            });
                        }
                    }
                    
                    println!("Drew anything? {}", drew_anything);
                    
                    let bytes = gtk::glib::Bytes::from(&img_data);
                    let texture = gtk::gdk::MemoryTexture::new(
                        width as i32,
                        height as i32,
                        gtk::gdk::MemoryFormat::R8g8b8a8,
                        &bytes,
                        (width * 4) as usize
                    );
                    pic_clone.set_paintable(Some(&texture));
                    pic_clone.set_size_request(width as i32, height as i32);
                    println!("Paintable set!");
                } else {
                    println!("Failed to load font from bytes! Path: {}", dumped_font_path.display());
                }
            });
            
            entry_test.set_text("Preview da fonte original");
            
            bx_v.append(&entry_test);
            bx_v.append(&pic);
        } else {
            let lbl_err = Label::new(Some("Aviso: Falha ao ler o arquivo da fonte."));
            lbl_err.add_css_class("muted-label");
            lbl_err.set_halign(gtk::Align::Start);
            bx_v.append(&lbl_err);
        }
    } else {
        let lbl_err = Label::new(Some("Aviso: Não foi possível extrair a prévia desta fonte."));
        lbl_err.add_css_class("muted-label");
        lbl_err.set_halign(gtk::Align::Start);
        bx_v.append(&lbl_err);
    }

    row.set_child(Some(&bx_v));

    let f_path = font_internal_path.to_string();
    let gp = game_path.to_string();
    let win_inner = parent_win.clone();
    
    btn_replace.connect_clicked(move |_| {
        let dialog = FileChooserNative::new(
            Some("Selecione a Nova Fonte"),
            Some(&win_inner),
            FileChooserAction::Open,
            Some("Substituir"),
            Some("Cancelar"),
        );

        let filter = gtk::FileFilter::new();
        filter.set_name(Some("Fontes (*.ttf, *.otf)"));
        filter.add_pattern("*.ttf");
        filter.add_pattern("*.otf");
        dialog.add_filter(&filter);

        let gp_clone = gp.clone();
        let f_path_clone = f_path.clone();
        let win_dialog = win_inner.clone();

        dialog.connect_response(move |d, response| {
            if response == ResponseType::Accept {
                if let Some(file) = d.file() {
                    if let Some(path) = file.path() {
                        let res = inject_renpy_individual(&gp_clone, &path, &f_path_clone);
                        let (msg_type, msg) = match res {
                            Ok(_) => (gtk::MessageType::Info, format!("Fonte '{}' substituída com sucesso!", f_path_clone)),
                            Err(e) => (gtk::MessageType::Error, e),
                        };

                        let msg_dialog = gtk::MessageDialog::new(
                            Some(&win_dialog),
                            gtk::DialogFlags::MODAL,
                            msg_type,
                            gtk::ButtonsType::Ok,
                            &msg
                        );
                        msg_dialog.connect_response(|md, _| md.destroy());
                        msg_dialog.show();
                    }
                }
            }
            d.destroy();
        });
        dialog.show();
    });

    row
}

fn scan_renpy_fonts(game_path_str: &str) -> Result<Vec<String>, String> {
    let original_path = PathBuf::from(game_path_str);
    let mut base_dir = original_path.clone();
    let mut executable = original_path.clone();

    if base_dir.is_file() {
        if let Some(p) = base_dir.parent() {
            base_dir = p.to_path_buf();
        }
    } else {
        let name = base_dir.file_name().unwrap_or_default().to_string_lossy().to_string();
        if cfg!(target_os = "windows") {
            executable = base_dir.join(format!("{}.exe", name));
        } else {
            executable = base_dir.join(name);
        }
    }
    
    if !executable.exists() {
        // Fallback: search for any .exe or executable if not found
        if let Ok(entries) = fs::read_dir(&base_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let path = entry.path();
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        if cfg!(target_os = "windows") && ext == "exe" {
                            executable = path;
                            break;
                        } else if !cfg!(target_os = "windows") && (ext == "sh" || ext == "x86_64") {
                            executable = path;
                            break;
                        }
                    }
                }
            }
        }
    }

    if !executable.exists() {
        return Err("Executável do Ren'Py não encontrado. Selecione o executável do jogo (.exe / .sh) em vez da pasta.".to_string());
    }

    let game_dir = base_dir.join("game");
    if !game_dir.exists() {
        return Err("Pasta 'game' não encontrada.".to_string());
    }

    let dumper_script = r#"
init 999 python:
    import json
    import os
    import sys
    fonts = []
    
    font_dir = os.path.join(renpy.config.basedir, "game", "tpg_temp_fonts")
    os.makedirs(font_dir, exist_ok=True)
    
    for f in renpy.list_files():
        fl = f.lower()
        if fl.endswith((".ttf", ".otf", ".woff", ".woff2")):
            fonts.append(f)
            try:
                content = renpy.file(f).read()
                out_path = os.path.join(font_dir, os.path.basename(f))
                with open(out_path, "wb") as out_f:
                    out_f.write(content)
            except:
                pass
    try:
        with open(renpy.config.basedir + "/game/tpg_fonts.json", "w", encoding="utf-8") as out:
            json.dump(fonts, out, ensure_ascii=False, indent=4)
    except:
        pass
    renpy.quit()
"#;

    let dumper_path = game_dir.join("tpg_font_dumper.rpy");
    fs::write(&dumper_path, dumper_script).map_err(|e| format!("Falha ao escrever dumper: {}", e))?;

    let json_path = game_dir.join("tpg_fonts.json");
    let _ = fs::remove_file(&json_path);

    let mut proc = crate::renpy_extractor::spawn_renpy_hidden(&executable.to_string_lossy())
        .map_err(|e| format!("Falha ao iniciar Ren'Py: {}", e))?;
    
    let _ = proc.wait();
    
    let _ = fs::remove_file(&dumper_path);
    let _ = fs::remove_file(game_dir.join("tpg_font_dumper.rpyc"));

    if !json_path.exists() {
        return Err("Arquivo JSON não gerado pelo motor.".to_string());
    }

    let content = fs::read_to_string(&json_path).map_err(|e| format!("Erro ao ler JSON: {}", e))?;
    let _ = fs::remove_file(&json_path);

    let fonts: Vec<String> = serde_json::from_str(&content).map_err(|e| format!("JSON inválido: {}", e))?;
    Ok(fonts)
}

fn inject_renpy_individual(game_path_str: &str, user_font_path: &Path, target_internal_path: &str) -> Result<(), String> {
    let mut base_dir = PathBuf::from(game_path_str);
    if base_dir.is_file() {
        if let Some(p) = base_dir.parent() {
            base_dir = p.to_path_buf();
        }
    }
    let game_dir = base_dir.join("game");
    if !game_dir.exists() {
        return Err("Pasta 'game' não encontrada.".to_string());
    }

    // target_internal_path might be "gui/fonts/montserrat.ttf"
    // we need to create "game/gui/fonts/" and copy there
    let target_full_path = game_dir.join(target_internal_path);
    if let Some(parent) = target_full_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Falha ao criar diretório: {}", e))?;
    }

    if target_full_path.exists() {
        // Backup original if it's not our backup already
        let backup_path = game_dir.join(format!("{}.tpg_backup", target_internal_path));
        if !backup_path.exists() {
            let _ = fs::copy(&target_full_path, &backup_path);
        }
    }

    fs::copy(user_font_path, &target_full_path).map_err(|e| format!("Falha ao copiar fonte: {}", e))?;

    Ok(())
}

fn append_font_preview(container: &Box, font_path: &Path) -> Result<(), String> {
    let font_data = fs::read(font_path).map_err(|e| format!("Falha ao ler a fonte: {e}"))?;
    if Font::try_from_vec(font_data.clone()).is_none() {
        return Err("A fonte extraída não é um TTF/OTF compatível com a prévia.".into());
    }
    let entry = Entry::new();
    entry.set_placeholder_text(Some("Digite para testar a fonte..."));
    entry.set_hexpand(true);
    let picture = Picture::new();
    picture.set_halign(gtk::Align::Start);
    picture.set_margin_top(5);
    picture.add_css_class("font-pic-outline");
    let picture_for_change = picture.clone();
    entry.connect_changed(move |entry| {
        let Some(font) = Font::try_from_vec(font_data.clone()) else { return; };
        let text = entry.text();
        if text.is_empty() { picture_for_change.set_paintable(None::<&gtk::gdk::Paintable>); return; }
        let scale = Scale::uniform(30.0);
        let metrics = font.v_metrics(scale);
        let glyphs: Vec<_> = font.layout(&text, scale, point(0.0, metrics.ascent)).collect();
        let width = glyphs.iter().map(|g| g.position().x + g.unpositioned().h_metrics().advance_width)
            .last().unwrap_or(1.0).ceil().max(1.0) as u32;
        let height = (metrics.ascent - metrics.descent).ceil().max(1.0) as u32;
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        for glyph in glyphs {
            if let Some(bb) = glyph.pixel_bounding_box() {
                glyph.draw(|x, y, value| {
                    let px = x as i32 + bb.min.x;
                    let py = y as i32 + bb.min.y;
                    if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                        let i = ((py * width as i32 + px) * 4) as usize;
                        pixels[i] = 255; pixels[i + 1] = 255; pixels[i + 2] = 255;
                        pixels[i + 3] = pixels[i + 3].max((value * 255.0) as u8);
                    }
                });
            }
        }
        let bytes = gtk::glib::Bytes::from(&pixels);
        let texture = gtk::gdk::MemoryTexture::new(width as i32, height as i32,
            gtk::gdk::MemoryFormat::R8g8b8a8, &bytes, (width * 4) as usize);
        picture_for_change.set_paintable(Some(&texture));
        picture_for_change.set_size_request(width as i32, height as i32);
    });
    entry.set_text("Prévia da fonte Unity: Áá Çç 123");
    container.append(&entry);
    container.append(&picture);
    Ok(())
}

fn create_font_row_unity(font_id: &str, game_path: &str, parent_win: &ApplicationWindow) -> ListBoxRow {
    let row = ListBoxRow::new();
    let bx_v = Box::new(Orientation::Vertical, 5);
    bx_v.set_margin_top(8); bx_v.set_margin_bottom(8);
    bx_v.set_margin_start(10); bx_v.set_margin_end(10);

    let font_parts = font_id
        .splitn(4, '|')
        .collect::<Vec<_>>();
    let is_embedded = font_parts.len() == 4 && font_parts[0] == "EMBEDDED";
    let display_name = if font_parts.len() == 4 {
        let kind = if is_embedded { "TTF/OTF incorporada" } else { "TextMeshPro/SDF" };
        format!("{}  —  {} ({})", font_parts[2], font_parts[1], kind)
    } else {
        font_id.to_string()
    };
    let lbl = Label::new(Some(&display_name));
    lbl.set_hexpand(true);
    lbl.set_halign(gtk::Align::Start);
    
    let btn_extract = Button::with_label("Extrair original");
    let btn_replace = Button::with_label("Substituir");
    if !is_embedded {
        btn_extract.set_sensitive(false);
        btn_replace.set_sensitive(false);
        btn_extract.set_tooltip_text(Some("Fontes TMP/SDF usam atlas; não possuem um TTF/OTF extraível."));
        btn_replace.set_tooltip_text(Some("Para trocar TMP/SDF é necessário um asset bundle TMP compatível."));
    }
    
    let bx_h = Box::new(Orientation::Horizontal, 10);
    bx_h.append(&lbl);
    bx_h.append(&btn_extract);
    bx_h.append(&btn_replace);
    bx_v.append(&bx_h);

    let f_path = if is_embedded {
        format!("{}|{}|{}", font_parts[1], font_parts[2], font_parts[3])
    } else { String::new() };
    let gp = game_path.to_string();
    let win_inner = parent_win.clone();

    if is_embedded {
        match export_unity_original_font(&gp, &f_path).and_then(|path| {
            append_font_preview(&bx_v, &path).map(|_| path)
        }) {
            Ok(_) => {}
            Err(error) => {
                let status = Label::new(Some(&format!("Prévia indisponível: {error}")));
                status.add_css_class("muted-label");
                status.set_halign(gtk::Align::Start);
                bx_v.append(&status);
            }
        }
    } else {
        match export_tmp_atlas_preview(&gp, font_parts[1], font_parts[3]) {
            Ok(path) => {
                let status = Label::new(Some("Prévia do atlas SDF usado pelo TextMeshPro:"));
                status.add_css_class("muted-label");
                status.set_halign(gtk::Align::Start);
                let picture = Picture::for_filename(path);
                picture.set_halign(gtk::Align::Start);
                picture.set_size_request(260, 120);
                bx_v.append(&status);
                bx_v.append(&picture);
            }
            Err(error) => {
                let status = Label::new(Some(&format!("Prévia do atlas indisponível: {error}")));
                status.add_css_class("muted-label");
                status.set_wrap(true);
                status.set_halign(gtk::Align::Start);
                bx_v.append(&status);
            }
        }
    }

    row.set_child(Some(&bx_v));

    let export_path = f_path.clone();
    let export_game = gp.clone();
    let export_window = win_inner.clone();
    btn_extract.connect_clicked(move |_| {
        let result = export_unity_original_font(&export_game, &export_path);
        let (kind, text) = match result {
            Ok(path) => (gtk::MessageType::Info, format!("Fonte original extraída em:\n{}", path.display())),
            Err(error) => (gtk::MessageType::Error, error),
        };
        let dialog = gtk::MessageDialog::new(
            Some(&export_window), gtk::DialogFlags::MODAL, kind, gtk::ButtonsType::Ok, &text,
        );
        dialog.connect_response(|d, _| d.destroy());
        dialog.show();
    });
    
    btn_replace.connect_clicked(move |_| {
        let dialog = FileChooserNative::new(
            Some("Selecione a Nova Fonte"),
            Some(&win_inner),
            FileChooserAction::Open,
            Some("Substituir"),
            Some("Cancelar"),
        );

        let filter = gtk::FileFilter::new();
        filter.set_name(Some("Fontes (*.ttf, *.otf)"));
        filter.add_pattern("*.ttf");
        filter.add_pattern("*.otf");
        dialog.add_filter(&filter);

        let gp_clone = gp.clone();
        let f_path_clone = f_path.clone();
        let win_dialog = win_inner.clone();

        dialog.connect_response(move |d, response| {
            if response == ResponseType::Accept {
                if let Some(file) = d.file() {
                    if let Some(path) = file.path() {
                        let res = inject_unity_individual(&gp_clone, &path, &f_path_clone);
                        let (msg_type, msg) = match res {
                            Ok(_) => (gtk::MessageType::Info, format!("Fonte Unity substituída com sucesso!")),
                            Err(e) => (gtk::MessageType::Error, e),
                        };

                        let msg_dialog = gtk::MessageDialog::new(
                            Some(&win_dialog),
                            gtk::DialogFlags::MODAL,
                            msg_type,
                            gtk::ButtonsType::Ok,
                            &msg
                        );
                        msg_dialog.connect_response(|md, _| md.destroy());
                        msg_dialog.show();
                    }
                }
            }
            d.destroy();
        });
        dialog.show();
    });

    row
}

fn scan_unity_fonts(game_path_str: &str) -> Result<Vec<String>, String> {
    let mut base_dir = PathBuf::from(game_path_str);
    if base_dir.is_file() {
        if let Some(p) = base_dir.parent() {
            base_dir = p.to_path_buf();
        }
    }
    
    let script_path = crate::paths::app_root().join("unity_static_extractor");
    
    let packaged = script_path.join(if cfg!(windows) { "unity_static_extractor.exe" } else { "unity_static_extractor" });
    let mut command = if packaged.is_file() {
        crate::paths::hidden_command(packaged)
    } else {
        let mut command = crate::paths::hidden_command("dotnet");
        command.arg("run").arg("--");
        command
    };
    let out = command
        .arg("font-scan")
        .arg(&base_dir.to_string_lossy().to_string())
        .current_dir(&script_path)
        .output()
        .map_err(|e| format!("Falha ao chamar C#: {}", e))?;
        
    if !out.status.success() {
        return Err(format!("Extrator UABEA falhou: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    let txt = String::from_utf8_lossy(&out.stdout);
    let mut fonts = Vec::new();
    for line in txt.lines() {
        if line.starts_with("[FONT_SCAN] ") {
            // UABEA gives us a stable locator: asset path, asset name and
            // path ID.  The ID prevents replacing a similarly named font.
            let parts: Vec<&str> = line["[FONT_SCAN] ".len()..].splitn(4, '|').collect();
            if parts.len() == 4 && matches!(parts[0], "EMBEDDED" | "TMP") {
                fonts.push(format!("{}|{}|{}|{}", parts[0], parts[1], parts[2], parts[3]));
            }
        }
    }
    
    Ok(fonts)
}

fn inject_unity_individual(game_path_str: &str, user_font_path: &Path, target_internal_path: &str) -> Result<(), String> {
    let parts: Vec<&str> = target_internal_path.splitn(3, '|').collect();
    if parts.len() != 3 {
        return Err("Formato de fonte Unity inválido.".to_string());
    }
    let asset_file = parts[0];
    let font_name = parts[1];
    let path_id = parts[2];
    let font_locator = format!("{}|{}", asset_file, path_id);
    
    let mut base_dir = PathBuf::from(game_path_str);
    if base_dir.is_file() {
        if let Some(p) = base_dir.parent() {
            base_dir = p.to_path_buf();
        }
    }
    
    let script_path = crate::paths::app_root().join("unity_static_extractor");
    
    let packaged = script_path.join(if cfg!(windows) { "unity_static_extractor.exe" } else { "unity_static_extractor" });
    let mut command = if packaged.is_file() {
        crate::paths::hidden_command(packaged)
    } else {
        let mut command = crate::paths::hidden_command("dotnet");
        command.arg("run").arg("--");
        command
    };
    let out = command
        .arg("font-inject")
        .arg(&base_dir.to_string_lossy().to_string())
        .arg(font_locator)
        .arg(font_name)
        .arg(&user_font_path.to_string_lossy().to_string())
        .current_dir(&script_path)
        .output()
        .map_err(|e| format!("Falha ao chamar C#: {}", e))?;
        
    let txt = String::from_utf8_lossy(&out.stdout);
    if txt.contains("[SUCCESS]") {
        Ok(())
    } else {
        Err(format!("Falha na injeção C#:\n{}", txt))
    }
}

fn export_unity_original_font(game_path_str: &str, target_internal_path: &str) -> Result<PathBuf, String> {
    let parts: Vec<&str> = target_internal_path.splitn(3, '|').collect();
    if parts.len() != 3 {
        return Err("Formato de fonte Unity inválido.".to_string());
    }
    let mut base_dir = PathBuf::from(game_path_str);
    if base_dir.is_file() {
        base_dir = base_dir.parent().ok_or("Pasta do jogo inválida.")?.to_path_buf();
    }
    // Same lifecycle as the Ren'Py preview: UABEA exports the original font
    // once into a game-local temporary directory, then the UI reads that file
    // for both preview and later manual export/replacement.
    let output_dir = base_dir.join("tpg_temp_fonts");
    let locator = format!("{}|{}", parts[0], parts[2]);
    let script_path = crate::paths::app_root().join("unity_static_extractor");
    let packaged = script_path.join(if cfg!(windows) { "unity_static_extractor.exe" } else { "unity_static_extractor" });
    let mut command = if packaged.is_file() {
        crate::paths::hidden_command(packaged)
    } else {
        let mut command = crate::paths::hidden_command("dotnet");
        command.arg("run").arg("--");
        command
    };
    let out = command
        .arg("font-export").arg(&base_dir).arg(locator).arg(&output_dir)
        .current_dir(&script_path).output()
        .map_err(|e| format!("Falha ao chamar UABEA: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    if !out.status.success() || !text.contains("[SUCCESS]") {
        return Err(format!("Falha ao extrair fonte com UABEA:\n{}", text));
    }
    text.lines().find_map(|line| line.strip_prefix("[SUCCESS] "))
        .map(PathBuf::from).ok_or("UABEA não retornou o arquivo extraído.".to_string())
}

fn export_tmp_atlas_preview(game_path_str: &str, asset_path: &str, path_id: &str) -> Result<PathBuf, String> {
    let mut base_dir = PathBuf::from(game_path_str);
    if base_dir.is_file() { base_dir = base_dir.parent().ok_or("Pasta do jogo inválida.")?.to_path_buf(); }
    let output_dir = base_dir.join("tpg_temp_fonts");
    let script_path = crate::paths::app_root().join("unity_static_extractor");
    let packaged = script_path.join(if cfg!(windows) { "unity_static_extractor.exe" } else { "unity_static_extractor" });
    let mut command = if packaged.is_file() { crate::paths::hidden_command(packaged) } else {
        let mut command = crate::paths::hidden_command("dotnet"); command.arg("run").arg("--"); command
    };
    let output = command.arg("tmp-atlas-export").arg(&base_dir).arg(asset_path).arg(path_id).arg(&output_dir)
        .current_dir(&script_path).output().map_err(|e| format!("Falha ao chamar UABEA: {e}"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() || !text.contains("[SUCCESS]") { return Err(text.trim().to_string()); }
    text.lines().find_map(|line| line.strip_prefix("[SUCCESS] ")).map(PathBuf::from)
        .ok_or("UABEA não retornou a prévia do atlas.".to_string())
        .and_then(|ppm_path| {
            // GTK on Windows does not bundle a PPM decoder. Convert the
            // temporary, lossless atlas to PNG before handing it to Picture.
            let png_path = ppm_path.with_extension("png");
            image::open(&ppm_path)
                .map_err(|e| format!("Falha ao abrir atlas temporário: {e}"))?
                .save(&png_path)
                .map_err(|e| format!("Falha ao converter atlas para PNG: {e}"))?;
            let _ = fs::remove_file(ppm_path);
            Ok(png_path)
        })
}
