use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::api;
use crate::ui::UiMsg;

pub fn detect_game_data_dir(executable: &str) -> Option<PathBuf> {
    let exe_path = Path::new(executable);
    let parent = exe_path.parent()?;
    let exe_name = exe_path.file_stem()?.to_string_lossy();
    let data_dir_name = format!("{}_Data", exe_name);
    let data_dir = parent.join(&data_dir_name);
    
    if data_dir.is_dir() {
        Some(data_dir)
    } else {
        None
    }
}

pub fn detect_unity_backend(executable: &str) -> Option<&'static str> {
    let data_dir = detect_game_data_dir(executable)?;
    
    // Check for Mono
    if data_dir.join("Managed").join("Assembly-CSharp.dll").exists() {
        return Some("Mono");
    }
    
    // Check for IL2CPP
    let parent = Path::new(executable).parent().unwrap_or(Path::new("."));
    if data_dir.join("il2cpp_data").exists() || parent.join("GameAssembly.dll").exists() {
        return Some("IL2CPP");
    }
    
    // Fallback if we have a _Data folder but can't distinguish (rare)
    Some("Unknown Unity")
}


pub fn output_folder(executable: &str, translation_folder: &str, target_lang_name: &str) -> PathBuf {
    let parent = Path::new(executable).parent().unwrap_or(Path::new("."));
    let name = if translation_folder.trim().is_empty() { target_lang_name } else { translation_folder.trim() };
    let safe_name = name.replace(['/', '\\'], "_");
    parent.join(format!("TBX_Workspace_{}", safe_name))
}

/// UnityPy complements AssetsTools.NET/UABEA for bundles and Addressables which
/// are not always discoverable as a normal `.assets` file. It is optional so a
/// packaged application remains usable without a Python runtime.
fn extract_with_unitypy(
    data_dir: &Path,
    extractor_dir: &Path,
    output_json: &Path,
    tx: &std::sync::mpsc::Sender<UiMsg>,
) -> Result<Option<Vec<String>>, String> {
    let script = extractor_dir.join("unitypy_extract.py");
    if !script.is_file() {
        return Ok(None);
    }

    let python = if cfg!(windows) { "python" } else { "python3" };
    let unitypy_checkout = extractor_dir
        .parent()
        .map(|root| root.join("third_party").join("UnityPy"));
    let mut command = crate::paths::hidden_command(python);
    command.arg(&script).arg(data_dir).arg(output_json);
    // A local checkout wins over a globally installed UnityPy. Python keeps its
    // normal dependency resolution, so a missing dependency is reported in the
    // existing non-fatal fallback message.
    if let Some(checkout) = unitypy_checkout.filter(|path| path.is_dir()) {
        let mut paths = vec![checkout];
        if let Some(existing) = std::env::var_os("PYTHONPATH") {
            paths.extend(std::env::split_paths(&existing));
        }
        if let Ok(joined) = std::env::join_paths(paths) {
            command.env("PYTHONPATH", joined);
        }
    }
    let output = match command.output() {
        Ok(output) => output,
        Err(_) => {
            let _ = tx.send(UiMsg::Log("[UnityPy] Python não encontrado; continuando com AssetsTools.NET.".into()));
            return Ok(None);
        }
    };

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let _ = tx.send(UiMsg::Log(line.to_owned()));
    }
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let _ = tx.send(UiMsg::Log(format!("[UnityPy] Indisponível ({}); continuando com AssetsTools.NET.", detail)));
        return Ok(None);
    }
    let content = fs::read_to_string(output_json).map_err(|e| format!("Erro lendo saída do UnityPy: {e}"))?;
    let texts = serde_json::from_str(&content).map_err(|e| format!("JSON inválido do UnityPy: {e}"))?;
    Ok(Some(texts))
}

/// Obtém o comando para rodar o extrator C# (UABEA / AssetsTools.NET).
/// Prioriza o binário standalone compilado (essencial no AppImage e releases empacotadas)
/// e faz fallback para `dotnet run --project ...` em ambiente de desenvolvimento local.
pub fn get_unity_extractor_command() -> Result<std::process::Command, String> {
    let app_root = crate::paths::app_root();
    let extractor_dir = app_root.join("unity_static_extractor");
    let packaged_extractor = extractor_dir.join(if cfg!(windows) { "unity_static_extractor.exe" } else { "unity_static_extractor" });

    if packaged_extractor.is_file() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(&packaged_extractor) {
                let mut perms = metadata.permissions();
                if perms.mode() & 0o111 == 0 {
                    perms.set_mode(perms.mode() | 0o755);
                    let _ = std::fs::set_permissions(&packaged_extractor, perms);
                }
            }
        }
        let mut cmd = crate::paths::hidden_command(&packaged_extractor);
        cmd.current_dir(&extractor_dir);
        return Ok(cmd);
    }

    let csproj = extractor_dir.join("unity_static_extractor.csproj");
    if csproj.is_file() {
        let mut cmd = crate::paths::hidden_command("dotnet");
        cmd.arg("run").arg("--project").arg(&csproj).arg("--");
        cmd.current_dir(&extractor_dir);
        return Ok(cmd);
    }

    Err(format!(
        "Extrator Unity não encontrado em: {}. (Nem o binário executável nem o projeto C# foram encontrados)",
        extractor_dir.display()
    ))
}

pub async fn extract_texts(
    executable: &str,
    translation_folder: &str,
    source_lang: &str,
    target_lang: &str,
    threads: u32,
    _api_engine: &str,
    tx: std::sync::mpsc::Sender<UiMsg>,
    cancelled: Arc<AtomicBool>,
    overwrite: bool,
    config: crate::app_config::AppConfig,
) -> Result<(), String> {
    let Some(data_dir) = detect_game_data_dir(executable) else {
        return Err("Pasta *_Data não encontrada. Não parece ser um jogo Unity.".into());
    };

    let _ = tx.send(UiMsg::Log(format!("Pasta de dados: {}", data_dir.display())));
    
    let app_root = crate::paths::app_root();
    let extractor_dir = app_root.join("unity_static_extractor");

    let out_dir = output_folder(executable, translation_folder, target_lang);
    let _ = fs::create_dir_all(&out_dir);

    let extracted_json = out_dir.join("extracted_texts.json");
    let unitypy_json = out_dir.join("unitypy_texts.json");
    let translated_json = out_dir.join("translated_texts.json");
    
    if overwrite {
        let _ = fs::remove_file(&translated_json);
        let _ = tx.send(UiMsg::Log("Tradução anterior apagada para sobrescrita.".to_string()));
    }
    
    let _ = tx.send(UiMsg::Log(format!("Chamando extrator C# (modo extract)...")));
    
    let mut command = get_unity_extractor_command()?;
    let output = command
        .arg("extract")
        .arg(&data_dir)
        .arg(&extracted_json)
        .output()
        .map_err(|e| format!("Falha ao executar extrator Unity: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains("Restore succeeded") || line.contains("Build succeeded") || line.contains("Build started") || line.contains("MSBuild version") { continue; }
        if line.contains("warning CS") { continue; }
        let _ = tx.send(UiMsg::Log(line.to_string()));
    }
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Erro no extrator C#: {}", stderr));
    }
    
    if !extracted_json.exists() {
        return Err("Arquivo JSON de extração não foi gerado!".into());
    }
    
    let json_content = fs::read_to_string(&extracted_json).map_err(|e| e.to_string())?;
    let mut texts: Vec<String> = serde_json::from_str(&json_content).map_err(|e| format!("Erro ao ler JSON: {}", e))?;

    let _ = tx.send(UiMsg::Log("Complementando bundles/Addressables com UnityPy...".into()));
    if let Some(unitypy_texts) = extract_with_unitypy(&data_dir, &extractor_dir, &unitypy_json, &tx)? {
        let before = texts.len();
        let mut unique: std::collections::HashSet<String> = texts.into_iter().collect();
        unique.extend(unitypy_texts);
        texts = unique.into_iter().collect();
        texts.sort_unstable();
        fs::write(&extracted_json, serde_json::to_string_pretty(&texts).map_err(|e| e.to_string())?)
            .map_err(|e| format!("Erro atualizando JSON consolidado: {e}"))?;
        let _ = tx.send(UiMsg::Log(format!("UnityPy adicionou {} textos únicos.", texts.len() - before)));
    }
    
    if texts.is_empty() {
        return Err("Nenhum texto encontrado para traduzir.".into());
    }
    
    let _ = tx.send(UiMsg::Log(format!("{} textos extraídos. Iniciando tradução...", texts.len())));

    let client = reqwest::Client::new();
    let mut src_code = api::get_lang_code(source_lang);
    let tgt_code = api::get_lang_code(target_lang);
    let mut detected_mismatch = false;
    let mut detection_attempts = 0;

    let batch_size = 64usize;
    let mut processed = 0usize;
    let total = texts.len();
    
    let mut translation_map = HashMap::new();

    // Check standard dictionary for instant local resolution
    let mut to_translate_indices: Vec<usize> = Vec::new();
    let mut resolved_translations: Vec<Option<String>> = vec![None; texts.len()];
    let mut was_cancelled = false;

    for (idx, text) in texts.iter().enumerate() {
        if let Some(std_trans) = crate::dictionary::lookup(text, tgt_code) {
            resolved_translations[idx] = Some(std_trans.to_string());
        } else {
            to_translate_indices.push(idx);
        }
    }

    let dict_hits = texts.len() - to_translate_indices.len();
    if dict_hits > 0 {
        let _ = tx.send(UiMsg::Log(format!("[Unity - Dicionário Padrão] {} termos de interface pré-traduzidos instantaneamente.", dict_hits)));
    }

    let mut translation_failures: Vec<String> = Vec::new();

    let total_chunks = (to_translate_indices.len() + batch_size - 1) / batch_size;
    for (chunk_idx, chunk_indices) in to_translate_indices.chunks(batch_size).enumerate() {
        if cancelled.load(Ordering::SeqCst) { 
            let _ = tx.send(UiMsg::Log("Cancelamento solicitado pelo usuário...".into()));
            was_cancelled = true;
            break;
        }

        let chunk: Vec<&String> = chunk_indices.iter().map(|&idx| &texts[idx]).collect();

        // Protect Yarn Spinner variables {0}, {1}, {2} and rich text tags before translation
        let _ = tx.send(UiMsg::Log(format!(
            "Traduzindo lote {} de {} ({} blocos)...",
            chunk_idx + 1,
            total_chunks,
            chunk.len()
        )));

        let mut protected_chunks: Vec<(String, Vec<(String, String)>)> = Vec::new();
        for &original in &chunk {
            let mut protected = original.clone();
            let mut replacements: Vec<(String, String)> = Vec::new();
            
            // Protect {0}, {1}, {2}, etc.
            let var_re = regex::Regex::new(r"\{(\d+)\}").unwrap();
            for cap in var_re.captures_iter(original) {
                let var = cap[0].to_string();
                let placeholder = format!("TBXVAR{}", &cap[1]);
                if !replacements.iter().any(|(from, _)| from == &var) {
                    replacements.push((var.clone(), placeholder.clone()));
                }
            }
            
            // Protect rich text tags like <color=#xxx>, </color>, <size=xxx>, <b>, </b>, etc.
            let tag_re = regex::Regex::new(r"</?[a-zA-Z][^>]*>").unwrap();
            let mut tag_idx = 0;
            for mat in tag_re.find_iter(original) {
                let tag = mat.as_str().to_string();
                let placeholder = format!("TBXTAG{}", tag_idx);
                if !replacements.iter().any(|(from, _)| from == &tag) {
                    replacements.push((tag.clone(), placeholder));
                    tag_idx += 1;
                }
            }
            
            // Apply replacements
            for (from, to) in &replacements {
                protected = protected.replace(from, to);
            }
            
            protected_chunks.push((protected, replacements));
        }

        let chunk_vec: Vec<String> = protected_chunks.iter().map(|(s, _)| s.clone()).collect();
        let ignored_tags = config.get_active_tags(Some(out_dir.join("tbx_tags.txt")));
        
        if !detected_mismatch && src_code != "auto" && detection_attempts < 15 {
            if let Some(sample) = chunk_vec.iter().filter(|t| t.len() > 15).max_by_key(|t| t.len()) {
                detection_attempts += 1;
                if let Some(detected) = api::detect_language(&client, sample).await {
                    let _ = tx.send(UiMsg::Log(format!("[DEBUG] Detectando idioma de '{}'... Resultado: {}", sample, detected)));
                    if detected != src_code && detected != "auto" {
                        src_code = api::get_lang_code(crate::api::get_lang_name(&detected));
                        let _ = tx.send(UiMsg::DetectedLanguageMismatch(detected));
                        detected_mismatch = true;
                    }
                }
            }
        }

        let translated = api::translate_batch_concurrent(&client, &chunk_vec, src_code, tgt_code, threads as usize, config.usar_traducao_pivo, &ignored_tags)
            .await.unwrap_or_else(|_| vec![]);

        for (i, &orig_idx) in chunk_indices.iter().enumerate() {
            let original = &texts[orig_idx];
            let trad = translated.get(i).cloned().unwrap_or_default();
            let trad = if trad.trim().is_empty() { 
                original.clone() 
            } else { 
                // Restore protected variables and tags
                let (_, ref replacements) = protected_chunks[i];
                let mut restored = trad.trim().to_string();
                for (from, to) in replacements {
                    restored = restored.replace(to, from);
                }
                restored
            };
            let _ = tx.send(UiMsg::Log(format!("  [OK] {} -> {}", original.replace('\n', " "), trad.replace('\n', " "))));
            resolved_translations[orig_idx] = Some(trad);
        }

        processed += chunk_indices.len();
        let _ = tx.send(UiMsg::Progress(processed, total));
    }

    for (i, original) in texts.iter().enumerate() {
        let trad = resolved_translations[i].clone().unwrap_or_else(|| original.clone());
        translation_map.insert(original.clone(), trad);
    }
    
    let map_json_str = serde_json::to_string_pretty(&translation_map).unwrap();
    fs::write(&translated_json, map_json_str).map_err(|e| e.to_string())?;
    
    if was_cancelled {
        let _ = tx.send(UiMsg::Log(format!("[Aviso] A tradução Unity foi cancelada. Os textos traduzidos até o momento foram salvos no JSON.")));
        let _ = tx.send(UiMsg::Cancelled);
    } else {
        let _ = tx.send(UiMsg::Log(format!("Extração e Tradução concluídas! Arquivo JSON salvo. Clique em 'INJETAR TRADUÇÃO' para aplicar no jogo.")));
        let _ = tx.send(UiMsg::Done("Extração e Tradução Unity concluídas!".to_string()));
    }

    Ok(())
}

pub fn extract_local_zip(zip_path: &Path, target_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(zip_path).map_err(|e| format!("Erro ao abrir ZIP {}: {}", zip_path.display(), e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Erro ao ler ZIP: {}", e))?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("Erro lendo arquivo no zip: {}", e))?;
        let outpath = match file.enclosed_name() {
            Some(path) => target_dir.join(path),
            None => continue,
        };

        if (*file.name()).ends_with('/') {
            let _ = fs::create_dir_all(&outpath);
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    let _ = fs::create_dir_all(p);
                }
            }
            let mut outfile = fs::File::create(&outpath).map_err(|e| format!("Erro criando {}: {}", outpath.display(), e))?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| format!("Erro extraindo {}: {}", outpath.display(), e))?;
        }
    }
    Ok(())
}

/// Find the best matching local ZIP file for the given backend
fn find_local_bepinex_zip(backend: &str) -> Option<PathBuf> {
    let bepinex_dir = crate::paths::app_root().join("BepInEx");
    if !bepinex_dir.is_dir() { return None; }

    let entries: Vec<_> = fs::read_dir(&bepinex_dir).ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "zip").unwrap_or(false))
        .collect();

    let name_lower = |p: &PathBuf| p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();

    if backend == "IL2CPP" {
        // Prefer IL2CPP or v6 zip
        entries.iter().find(|p| {
            let n = name_lower(p);
            n.contains("il2cpp") || n.contains("6.0")
        }).cloned()
    } else {
        // Prefer Mono / v5 zip
        entries.iter().find(|p| {
            let n = name_lower(p);
            (n.contains("mono") || n.contains("5.4")) && !n.contains("il2cpp")
        }).cloned().or_else(|| {
            // Fallback: any BepInEx zip that's not IL2CPP
            entries.iter().find(|p| !name_lower(p).contains("il2cpp")).cloned()
        })
    }
}

/// Find the best matching local XUnity ZIP
fn find_local_xunity_zip(backend: &str) -> Option<PathBuf> {
    let xunity_dir = crate::paths::app_root().join("XUnity_AutoTranslator_bepInEx");
    if !xunity_dir.is_dir() { return None; }

    let entries: Vec<_> = fs::read_dir(&xunity_dir).ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "zip").unwrap_or(false))
        .collect();

    let name_lower = |p: &PathBuf| p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();

    if backend == "IL2CPP" {
        entries.iter().find(|p| name_lower(p).contains("il2cpp")).cloned()
    } else {
        entries.iter().find(|p| !name_lower(p).contains("il2cpp")).cloned()
    }
}

pub async fn inject_texts(
    executable: &str,
    translation_folder: &str,
    target_lang: &str,
    tx: std::sync::mpsc::Sender<UiMsg>,
) -> Result<(), String> {
    
    let backend = detect_unity_backend(executable).unwrap_or("Desconhecido");
    
    let out_dir = output_folder(executable, translation_folder, target_lang);
    let translated_json = out_dir.join("translated_texts.json");
    
    if !translated_json.exists() {
        return Err("Nenhum arquivo JSON de tradução encontrado! Faça a extração/tradução primeiro.".into());
    }

    let _ = tx.send(UiMsg::Log(format!("Gerando dicionário para XUnity.AutoTranslator (Motor: {})...", backend)));
    
    // Ler o JSON traduzido
    let json_content = fs::read_to_string(&translated_json).map_err(|e| e.to_string())?;
    let translation_map: HashMap<String, String> = serde_json::from_str(&json_content).map_err(|e| format!("Erro ao ler JSON: {}", e))?;
    
    if translation_map.is_empty() {
        return Err("O JSON de tradução está vazio!".into());
    }

    let mut dict_content = String::new();
    dict_content.push_str("// Formato compativel com XUnity.AutoTranslator: Original=Traducao\n");
    dict_content.push_str("// Gerado automaticamente pelo TBX - Translator\n\n");
    
    for (orig, trad) in &translation_map {
        // XUnity suporta escapes para quebras de linha
        let safe_orig = orig.replace('\n', "\\n").replace('\r', "\\r");
        let safe_trad = trad.replace('\n', "\\n").replace('\r', "\\r");
        
        // Evitar strings puramente vazias de corromper o layout
        if safe_orig.trim().is_empty() {
            continue;
        }
        
        dict_content.push_str(&format!("{}={}\n", safe_orig, safe_trad));
    }
    
    // Salvar na pasta Workspace
    let output_txt = out_dir.join("_AutoGeneratedTranslations.txt");
    fs::write(&output_txt, &dict_content).map_err(|e| e.to_string())?;
    
    let _ = tx.send(UiMsg::Log(format!("Dicionário gerado com sucesso: {}", output_txt.display())));
    
    let parent = Path::new(executable).parent().unwrap_or(Path::new("."));
    let bepinex_dir = parent.join("BepInEx");
    
    // === INSTALAÇÃO DO BEPINEX A PARTIR DOS ZIPS LOCAIS ===
    if !bepinex_dir.exists() {
        let _ = tx.send(UiMsg::Log("BepInEx não encontrado. Instalando dos ZIPs locais...".into()));
        
        // 1. Instalar BepInEx
        if let Some(bepinex_zip) = find_local_bepinex_zip(&backend) {
            let _ = tx.send(UiMsg::Log(format!("Extraindo BepInEx de: {}", bepinex_zip.file_name().unwrap_or_default().to_string_lossy())));
            extract_local_zip(&bepinex_zip, parent)?;
            let _ = tx.send(UiMsg::Log("BepInEx instalado com sucesso!".into()));
        } else {
            return Err("ZIP do BepInEx não encontrado na pasta BepInEx/ do TBX!".into());
        }
        
        // 2. Instalar XUnity.AutoTranslator
        if let Some(xunity_zip) = find_local_xunity_zip(&backend) {
            let _ = tx.send(UiMsg::Log(format!("Extraindo XUnity.AutoTranslator de: {}", xunity_zip.file_name().unwrap_or_default().to_string_lossy())));
            extract_local_zip(&xunity_zip, parent)?;
            let _ = tx.send(UiMsg::Log("XUnity.AutoTranslator instalado com sucesso!".into()));
        } else {
            let _ = tx.send(UiMsg::Log("AVISO: ZIP do XUnity.AutoTranslator não encontrado. Tradução runtime não estará disponível.".into()));
        }
        
        // 3. Rodar o jogo brevemente para gerar configs iniciais do BepInEx
        let _ = tx.send(UiMsg::Log("Iniciando jogo brevemente para gerar configs do BepInEx...".into()));
        
        let game_process = crate::paths::hidden_command(executable)
            .env("WINEDLLOVERRIDES", "winhttp=n,b")
            .spawn();
        
        match game_process {
            Ok(mut child) => {
                let _ = tx.send(UiMsg::Log("Aguardando BepInEx inicializar (10 segundos)...".into()));
                std::thread::sleep(std::time::Duration::from_secs(10));
                let _ = child.kill();
                let _ = child.wait();
                let _ = tx.send(UiMsg::Log("Jogo fechado. Configs do BepInEx gerados!".into()));
            }
            Err(_) => {
                let _ = tx.send(UiMsg::Log("(AVISO) O jogo não iniciou automaticamente (normal no Linux se for um .exe Windows).".into()));
                let _ = tx.send(UiMsg::Log("IMPORTANTE (LINUX/PROTON): Para o BepInEx funcionar, você precisa adicionar nas opções de inicialização do jogo (Steam):".into()));
                let _ = tx.send(UiMsg::Log("WINEDLLOVERRIDES=\"winhttp=n,b\" %command%".into()));
            }
        }
        
        // Configurar o AutoTranslatorConfig.ini para garantir que o idioma está correto
        let config_dir = bepinex_dir.join("config");
        let _ = fs::create_dir_all(&config_dir);
        let config_file = config_dir.join("AutoTranslatorConfig.ini");
        
        let target_code = api::get_lang_code(target_lang);
        if config_file.exists() {
            if let Ok(mut content) = fs::read_to_string(&config_file) {
                let re_lang = regex::Regex::new(r"(?m)^Language=.*").unwrap();
                let re_from = regex::Regex::new(r"(?m)^FromLanguage=.*").unwrap();
                
                content = re_lang.replace(&content, format!("Language={}", target_code)).to_string();
                content = re_from.replace(&content, "FromLanguage=en").to_string();
                
                let _ = fs::write(&config_file, content);
                let _ = tx.send(UiMsg::Log("AutoTranslatorConfig.ini atualizado para o idioma escolhido.".into()));
            }
        } else {
            let config_content = format!(
"[Service]
Endpoint=GoogleTranslateV2

[General]
Language={}
FromLanguage=en

[Files]
Directory=Translation
OutputFile=Translation\\{}\\Text\\_AutoGeneratedTranslations.txt

[TextFrameworks]
EnableUGUI=True
EnableIMGUI=True
EnableTextMeshPro=True
EnableTextMesh=True
EnableFairyGUI=True", target_code, target_code);
            let _ = fs::write(&config_file, config_content);
            let _ = tx.send(UiMsg::Log("Configuração do XUnity AutoTranslator gerada manualmente com sucesso!".into()));
        }

    }

    // BepInEx may already be present from another mod. In that case the old
    // flow skipped XUnity entirely, then reported a successful copy into a
    // folder no plugin would ever read. Check the plugin independently.
    let xunity_plugin = bepinex_dir
        .join("plugins")
        .join("XUnity.AutoTranslator")
        .join("XUnity.AutoTranslator.Plugin.BepInEx.dll");
    if !xunity_plugin.is_file() {
        let _ = tx.send(UiMsg::Log("XUnity.AutoTranslator não encontrado. Instalando...".into()));
        let xunity_zip = find_local_xunity_zip(&backend)
            .ok_or_else(|| "ZIP do XUnity.AutoTranslator não encontrado na pasta XUnity_AutoTranslator_bepInEx/.".to_string())?;
        extract_local_zip(&xunity_zip, parent)?;
    }
    if !xunity_plugin.is_file() {
        return Err("XUnity.AutoTranslator não foi instalado corretamente; o plugin DLL não foi encontrado.".into());
    }

    // Always ensure the selected target language is configured, including when
    // BepInEx was installed before this application ran.
    let config_dir = bepinex_dir.join("config");
    fs::create_dir_all(&config_dir).map_err(|e| format!("Falha criando config do BepInEx: {e}"))?;
    let config_file = config_dir.join("AutoTranslatorConfig.ini");
    let target_code = api::get_lang_code(target_lang);
    if config_file.exists() {
        let mut content = fs::read_to_string(&config_file).map_err(|e| format!("Falha lendo AutoTranslatorConfig.ini: {e}"))?;
        let re_lang = regex::Regex::new(r"(?m)^Language=.*").unwrap();
        let re_from = regex::Regex::new(r"(?m)^FromLanguage=.*").unwrap();
        content = re_lang.replace(&content, format!("Language={}", target_code)).to_string();
        content = re_from.replace(&content, "FromLanguage=en").to_string();
        fs::write(&config_file, content).map_err(|e| format!("Falha atualizando AutoTranslatorConfig.ini: {e}"))?;
    } else {
        let config_content = format!(
"[Service]
Endpoint=GoogleTranslateV2

[General]
Language={}
FromLanguage=en

[Files]
Directory=Translation
OutputFile=Translation\\{}\\Text\\_AutoGeneratedTranslations.txt

[TextFrameworks]
EnableUGUI=True
EnableIMGUI=True
EnableTextMeshPro=True
EnableTextMesh=True
EnableFairyGUI=True", target_code, target_code);
        fs::write(&config_file, config_content).map_err(|e| format!("Falha criando AutoTranslatorConfig.ini: {e}"))?;
    }
    let _ = tx.send(UiMsg::Log(format!("XUnity.AutoTranslator pronto: {}", xunity_plugin.display())));
    
    // === COPIAR TRADUÇÃO PARA O BEPINEX ===
    let bepinex_text_dir = bepinex_dir
        .join("Translation")
        .join(api::get_lang_code(target_lang)) // e.g. 'pt'
        .join("Text");
        
    // Forçar a criação da pasta caso o jogo ainda não tenha sido rodado
    if !bepinex_text_dir.exists() {
        let _ = fs::create_dir_all(&bepinex_text_dir);
    }
    
    let bepinex_file = bepinex_text_dir.join("_AutoGeneratedTranslations.txt");
    if let Err(e) = fs::write(&bepinex_file, &dict_content) {
        let _ = tx.send(UiMsg::Log(format!("Falha ao copiar dicionário para BepInEx: {}", e)));
    } else {
        let _ = tx.send(UiMsg::Log(format!("Dicionário injetado com sucesso na pasta do jogo!")));
        let _ = tx.send(UiMsg::Log(format!("Arquivo: {}", bepinex_file.display())));
        let _ = tx.send(UiMsg::Log(format!("Tudo pronto! Agora é só abrir o jogo.")));
    }

    Ok(())
}
