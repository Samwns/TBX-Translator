// TBX Translator - editor_ui.rs
// Creator: samwns
// Pure Rust Dialogue & Text Translation Editor using eframe/egui

use std::fs;
use std::path::{Path, PathBuf};
use egui::{Color32, Context, RichText, ScrollArea, TextEdit, Ui, Vec2};
use walkdir::WalkDir;

#[derive(Clone, Debug, PartialEq)]
pub enum FileType {
    RenpyRpy,
    UnityTxt,
    UnityJson,
}

#[derive(Clone, Debug)]
pub struct DialogueEntry {
    pub key: String,
    pub original: String,
    pub translated: String,
    pub raw_context: String, // metadata or context for reconstruction
}

#[derive(Clone, Debug)]
pub struct TranslationFile {
    pub path: PathBuf,
    pub relative_name: String,
    pub file_type: FileType,
    pub entries: Vec<DialogueEntry>,
}

pub struct EditorState {
    pub translation_dir: Option<PathBuf>,
    pub files: Vec<TranslationFile>,
    pub selected_file_index: Option<usize>,
    pub search_query: String,
    pub status_message: Option<(bool, String)>,
    pub is_dirty: bool,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            translation_dir: None,
            files: Vec::new(),
            selected_file_index: None,
            search_query: String::new(),
            status_message: None,
            is_dirty: false,
        }
    }
}

impl EditorState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_directory(&mut self, base_game_path: &str, folder_name: &str, is_unity: bool) {
        self.files.clear();
        self.selected_file_index = None;
        self.status_message = None;
        self.is_dirty = false;

        let base_path = PathBuf::from(base_game_path);
        let parent_dir = if base_path.is_file() {
            base_path.parent().unwrap_or(&base_path).to_path_buf()
        } else {
            base_path
        };

        let target_dir = if is_unity {
            let safe_name = if folder_name.trim().is_empty() { "portuguese" } else { folder_name.trim() };
            parent_dir.join(format!("TBX_Workspace_{}", safe_name))
        } else {
            let safe_folder = if folder_name.trim().is_empty() { "portuguese" } else { folder_name.trim() };
            parent_dir.join("game").join("tl").join(safe_folder)
        };

        if !target_dir.exists() {
            self.status_message = Some((
                true,
                format!("Diretório de tradução não encontrado:\n{}", target_dir.display()),
            ));
            self.translation_dir = None;
            return;
        }

        self.translation_dir = Some(target_dir.clone());

        // Scan files
        for entry in WalkDir::new(&target_dir).into_iter().flatten() {
            if entry.file_type().is_file() {
                let path = entry.path().to_path_buf();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                let rel_path = path.strip_prefix(&target_dir)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();

                if ext == "rpy" {
                    if let Ok(entries) = parse_rpy_file(&path) {
                        if !entries.is_empty() {
                            self.files.push(TranslationFile {
                                path,
                                relative_name: rel_path,
                                file_type: FileType::RenpyRpy,
                                entries,
                            });
                        }
                    }
                } else if ext == "txt" {
                    if let Ok(entries) = parse_txt_file(&path) {
                        if !entries.is_empty() {
                            self.files.push(TranslationFile {
                                path,
                                relative_name: rel_path,
                                file_type: FileType::UnityTxt,
                                entries,
                            });
                        }
                    }
                } else if ext == "json" {
                    if let Ok(entries) = parse_json_file(&path) {
                        if !entries.is_empty() {
                            self.files.push(TranslationFile {
                                path,
                                relative_name: rel_path,
                                file_type: FileType::UnityJson,
                                entries,
                            });
                        }
                    }
                }
            }
        }

        if !self.files.is_empty() {
            self.selected_file_index = Some(0);
        } else {
            self.status_message = Some((
                false,
                "Nenhum arquivo de tradução com diálogos foi encontrado nesta pasta.".to_string(),
            ));
        }
    }

    pub fn save_current_file(&mut self) {
        let Some(idx) = self.selected_file_index else { return };
        let Some(file) = self.files.get(idx) else { return };

        let result = match file.file_type {
            FileType::RenpyRpy => save_rpy_file(&file.path, &file.entries),
            FileType::UnityTxt => save_txt_file(&file.path, &file.entries),
            FileType::UnityJson => save_json_file(&file.path, &file.entries),
        };

        match result {
            Ok(_) => {
                self.is_dirty = false;
                self.status_message = Some((
                    false,
                    format!("✅ Arquivo '{}' salvo com sucesso!", file.relative_name),
                ));
            }
            Err(e) => {
                self.status_message = Some((
                    true,
                    format!("❌ Erro ao salvar '{}': {}", file.relative_name, e),
                ));
            }
        }
    }

    pub fn render_ui(&mut self, ui: &mut Ui, _ctx: &Context) {
        if self.files.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);
                if let Some((is_err, msg)) = &self.status_message {
                    let col = if *is_err { Color32::from_rgb(243, 139, 168) } else { Color32::from_rgb(166, 227, 161) };
                    ui.label(RichText::new(msg).color(col).strong());
                } else {
                    ui.label(RichText::new("Nenhum arquivo carregado no editor.").color(Color32::from_rgb(166, 173, 200)));
                }
            });
            return;
        }

        // Top bar
        ui.horizontal(|ui| {
            ui.label(RichText::new("🔍 Buscar:").color(Color32::from_rgb(203, 166, 247)).strong());
            ui.add(TextEdit::singleline(&mut self.search_query).hint_text("Filtrar diálogos ou termos..."));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let save_btn = egui::Button::new(
                    RichText::new("💾 Salvar Arquivo")
                        .color(Color32::from_rgb(17, 17, 27))
                        .strong(),
                )
                .fill(Color32::from_rgb(166, 227, 161));

                if ui.add(save_btn).clicked() {
                    self.save_current_file();
                }

                if let Some((is_err, msg)) = &self.status_message {
                    let col = if *is_err { Color32::from_rgb(243, 139, 168) } else { Color32::from_rgb(166, 227, 161) };
                    ui.label(RichText::new(msg).color(col).small());
                }
            });
        });

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);

        // Sidebar + Content layout
        egui::SidePanel::left("editor_sidebar")
            .resizable(true)
            .min_width(200.0)
            .default_width(240.0)
            .show_inside(ui, |ui| {
                ui.label(RichText::new("📂 Arquivos de Tradução").color(Color32::from_rgb(137, 180, 250)).strong());
                ui.add_space(4.0);

                ScrollArea::vertical()
                    .id_salt("editor_files_sidebar")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                    for (i, file) in self.files.iter().enumerate() {
                        let is_selected = self.selected_file_index == Some(i);
                        let text = format!("{} ({})", file.relative_name, file.entries.len());

                        let btn = egui::Button::new(
                            RichText::new(text)
                                .color(if is_selected { Color32::WHITE } else { Color32::from_rgb(166, 173, 200) })
                                .strong(),
                        )
                        .fill(if is_selected { Color32::from_rgb(49, 50, 68) } else { Color32::TRANSPARENT });

                        if ui.add(btn).clicked() {
                            self.selected_file_index = Some(i);
                            self.status_message = None;
                        }
                    }
                });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            let Some(file_idx) = self.selected_file_index else { return };
            let Some(file) = self.files.get_mut(file_idx) else { return };

                    let query = self.search_query.to_lowercase();

                    ScrollArea::vertical()
                        .id_salt("editor_dialogues_list")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                        let mut changed = false;

                        for (idx, entry) in file.entries.iter_mut().enumerate() {
                            if !query.is_empty()
                                && !entry.original.to_lowercase().contains(&query)
                                && !entry.translated.to_lowercase().contains(&query)
                            {
                                continue;
                            }

                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(format!("#{}:", idx + 1)).color(Color32::from_rgb(108, 112, 134)).small());
                                    if !entry.key.is_empty() {
                                        ui.label(RichText::new(&entry.key).color(Color32::from_rgb(203, 166, 247)).small());
                                    }
                                });

                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Original:").color(Color32::from_rgb(147, 154, 183)).small());
                                    if ui.add(egui::Button::new("📋 Copiar").small().fill(Color32::from_rgb(49, 50, 68))).clicked() {
                                        ui.output_mut(|o| o.copied_text = entry.original.clone());
                                    }
                                });
                                ui.colored_label(Color32::from_rgb(249, 226, 175), &entry.original);

                                ui.add_space(2.0);
                                ui.label(RichText::new("Tradução:").color(Color32::from_rgb(166, 227, 161)).small());
                                let resp = ui.add(
                                    TextEdit::multiline(&mut entry.translated)
                                        .desired_width(ui.available_width())
                                        .desired_rows(2),
                                );
                                if resp.changed() {
                                    changed = true;
                                }
                            });
                            ui.add_space(6.0);
                        }

                        if changed {
                            self.is_dirty = true;
                        }
                    });
        });
    }
}

// ---------------- Parsers and Savers ----------------

fn parse_rpy_file(path: &Path) -> Result<Vec<DialogueEntry>, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut entries = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();

        // Pattern 1: old "..." \n new "..."
        if line.starts_with("old \"") && line.ends_with('\"') && line.len() >= 6 {
            let orig = &line[5..line.len() - 1];
            let mut trans = String::new();
            if i + 1 < lines.len() {
                let next_line = lines[i + 1].trim();
                if next_line.starts_with("new \"") && next_line.ends_with('\"') && next_line.len() >= 6 {
                    trans = next_line[5..next_line.len() - 1].to_string();
                    i += 1;
                }
            }
            entries.push(DialogueEntry {
                key: format!("line_{}", i),
                original: orig.replace("\\\"", "\""),
                translated: trans.replace("\\\"", "\""),
                raw_context: "strings".to_string(),
            });
        }
        // Pattern 2: # <speaker> "..." \n <speaker> "..."
        else if line.starts_with("# ") && (line.contains('\"') || line.contains('“')) {
            let orig_line = line[2..].trim();
            let mut trans_line = String::new();
            if i + 1 < lines.len() {
                let next = lines[i + 1].trim();
                if !next.starts_with('#') && (next.contains('\"') || next.contains('“')) {
                    trans_line = next.to_string();
                    i += 1;
                }
            }
            entries.push(DialogueEntry {
                key: format!("line_{}", i),
                original: orig_line.to_string(),
                translated: trans_line,
                raw_context: "dialogue".to_string(),
            });
        }
        i += 1;
    }

    Ok(entries)
}

fn save_rpy_file(path: &Path, entries: &[DialogueEntry]) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    let mut entry_idx = 0;
    let mut i = 0;
    while i < lines.len() && entry_idx < entries.len() {
        let trimmed = lines[i].trim().to_string();
        if trimmed.starts_with("old \"") && trimmed.ends_with('\"') {
            if i + 1 < lines.len() {
                let next_trimmed = lines[i + 1].trim().to_string();
                if next_trimmed.starts_with("new \"") && next_trimmed.ends_with('\"') {
                    let indent = lines[i + 1].chars().take_while(|c| c.is_whitespace()).collect::<String>();
                    let escaped = entries[entry_idx].translated.replace('\"', "\\\"");
                    lines[i + 1] = format!("{}new \"{}\"", indent, escaped);
                    entry_idx += 1;
                    i += 1;
                }
            }
        } else if trimmed.starts_with("# ") && (trimmed.contains('\"') || trimmed.contains('“')) {
            if i + 1 < lines.len() {
                let next_trimmed = lines[i + 1].trim().to_string();
                if !next_trimmed.starts_with('#') && (next_trimmed.contains('\"') || next_trimmed.contains('“')) {
                    let indent = lines[i + 1].chars().take_while(|c| c.is_whitespace()).collect::<String>();
                    lines[i + 1] = format!("{}{}", indent, entries[entry_idx].translated);
                    entry_idx += 1;
                    i += 1;
                }
            }
        }
        i += 1;
    }

    let output = lines.join("\n");
    fs::write(path, output).map_err(|e| e.to_string())?;
    Ok(())
}

fn parse_txt_file(path: &Path) -> Result<Vec<DialogueEntry>, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut entries = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }

        if let Some((key, val)) = line.split_once('=') {
            entries.push(DialogueEntry {
                key: key.trim().to_string(),
                original: key.trim().to_string(),
                translated: val.trim().to_string(),
                raw_context: format!("{}", idx),
            });
        }
    }

    Ok(entries)
}

fn save_txt_file(path: &Path, entries: &[DialogueEntry]) -> Result<(), String> {
    let mut out = String::new();
    for entry in entries {
        out.push_str(&format!("{}={}\n", entry.original, entry.translated));
    }
    fs::write(path, out).map_err(|e| e.to_string())?;
    Ok(())
}

fn parse_json_file(path: &Path) -> Result<Vec<DialogueEntry>, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut entries = Vec::new();

    if let Ok(map) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&content) {
        for (k, v) in map {
            let val_str = v.as_str().unwrap_or("").to_string();
            entries.push(DialogueEntry {
                key: k.clone(),
                original: k,
                translated: val_str,
                raw_context: "json_map".to_string(),
            });
        }
    }

    Ok(entries)
}

fn save_json_file(path: &Path, entries: &[DialogueEntry]) -> Result<(), String> {
    let mut map = serde_json::Map::new();
    for entry in entries {
        map.insert(entry.original.clone(), serde_json::Value::String(entry.translated.clone()));
    }
    let val = serde_json::Value::Object(map);
    let out = serde_json::to_string_pretty(&val).map_err(|e| e.to_string())?;
    fs::write(path, out).map_err(|e| e.to_string())?;
    Ok(())
}
