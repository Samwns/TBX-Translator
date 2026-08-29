//! Wrapper para o GDRE Tools (gdsdecomp) CLI.
//!
//! Usado para extrair recursos de jogos Godot (PCK/EXE embutido) e para
//! reinjetar arquivos traduzidos diretamente no executável/PCK original
//! via `--pck-patch --embed`, dispensando patch PCK separado + override.cfg.

use std::fs;
use std::path::{Path, PathBuf};

/// Localiza o executável gdre_tools.
///
/// Ordem de busca:
/// 1. Variável de ambiente `TBX_GDRE_TOOLS`
/// 4. `<dir_do_app>/gdre_tools[.exe]`
/// 2. `<dir_do_app>/third_party/releases/gdre_tools[.exe]` (dev/repo)
/// 3. `<dir_do_app>/../third_party/releases/gdre_tools[.exe]`
/// 5. `gdre_tools` no PATH
pub fn locate() -> Option<PathBuf> {
    if let Ok(custom) = std::env::var("TBX_GDRE_TOOLS") {
        let p = PathBuf::from(custom);
        if p.is_file() {
            return Some(p);
        }
    }

    let exe_name = if cfg!(windows) { "gdre_tools.exe" } else { "gdre_tools.x86_64" };

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(exe_name));
            candidates.push(dir.join("third_party/releases").join(exe_name));
            candidates.push(dir.join("../third_party/releases").join(exe_name));
        }
    }
    // Diretório de trabalho (útil em desenvolvimento via cargo run)
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("third_party/releases").join(exe_name));
    }

    candidates.into_iter().find(|p| p.is_file()).or_else(|| {
        // PATH
        let path_var = std::env::var_os("PATH")?;
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(exe_name);
            if candidate.is_file() {
                return Some(candidate);
            }
            if cfg!(windows) {
                let alt = dir.join("gdre_tools.exe");
                if alt.is_file() {
                    return Some(alt);
                }
            }
        }
        None
    })
}

fn run(args: &[String]) -> Result<String, String> {
    run_with_cwd(args, None)
}

/// Executa gdre_tools com um cwd explicitamente controlado. Algumas operacoes
/// (como --bin-to-txt) gravam o arquivo de saida NO CWD em vez de ao lado do
/// input, entao precisamos apontar para um diretorio conhecido.
fn run_with_cwd(args: &[String], cwd: Option<&Path>) -> Result<String, String> {
    let bin = locate().ok_or_else(|| {
        "GDRE Tools não encontrado. Coloque gdre_tools.x86_64 em third_party/releases ou defina TBX_GDRE_TOOLS.".to_string()
    })?;
    let mut cmd = crate::paths::hidden_command(&bin);
    cmd.arg("--headless").args(args);
    // O Godot carrega override.cfg do cwd. Se o cwd for a raiz do projeto (ou
    // outra pasta que contenha um override.cfg), ele quebra a inicializacao do
    // gdre_tools. Rodamos num diretorio temporario dedicado (sem arquivos
    // conflitantes) para evitar o erro "Cannot open res://tbx_injector.tscn".
    // Quando `cwd` eh fornecido explicitamente (ex.: bin_to_txt), usamos ele
    // porque gdre grava a saida no CWD do processo, nao ao lado do input.
    let effective_cwd: PathBuf = match cwd {
        Some(c) => c.to_path_buf(),
        None => {
            let n = std::env::temp_dir().join(format!("tbx-gdre-cwd-{}", std::process::id()));
            let _ = std::fs::create_dir_all(&n);
            n
        }
    };
    let _ = std::fs::create_dir_all(&effective_cwd);
    cmd.current_dir(&effective_cwd);
    let output = cmd.output().map_err(|e| format!("Falha ao executar gdre_tools: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{stdout}\n{stderr}");
    let succeeded = output.status.success()
        || combined.contains("no errors detected")
        || combined.contains("operation complete")
        || combined.contains("Patched PCK file:")
        || combined.contains("Successfully");
    let fatal = combined.contains("Failed to open")
        || combined.contains("No valid paths provided")
        || combined.contains("ERROR: Failed");
    if fatal && !succeeded {
        return Err(format!("gdre_tools falhou: {}", tail(&combined, 30)));
    }
    Ok(combined)
}

fn tail(s: &str, lines: usize) -> String {
    let v: Vec<&str> = s.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.contains('%') && !l.contains("WARNING"))
        .collect();
    v.iter().rev().take(lines).rev().cloned().collect::<Vec<_>>().join("\n")
}

/// Extrai todos os arquivos do PCK/EXE para `output_dir`.
pub fn extract(game: &Path, output_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;
    let args = vec![
        format!("--extract={}", game.display()),
        format!("--output={}", output_dir.display()),
    ];
    let log = run(&args)?;
    if log.contains("no errors detected") || log.contains("Extraction operation complete") {
        Ok(())
    } else {
        Err(format!("Extração gdre_tools com problemas: {}", tail(&log, 15)))
    }
}

/// Reinjeta `files` (src local → res://dest) copiando todo o restante do PCK
/// original e embutindo de volta no executável (quando `embed_in_exe`).
/// Retorna o caminho do novo arquivo gerado (não sobrescreve o original).
pub fn patch_embed(game: &Path, files: &[(PathBuf, String)], embed_exe: Option<&Path>, output: &Path) -> Result<(), String> {
    let mut args: Vec<String> = vec![format!("--pck-patch={}", game.display())];
    for (src, dest) in files {
        args.push(format!("--patch-file={}={}", src.display(), dest));
    }
    if let Some(exe) = embed_exe {
        args.push(format!("--embed={}", exe.display()));
    }
    args.push(format!("--output={}", output.display()));
    run(&args)?;
    if output.is_file() {
        Ok(())
    } else {
        Err("gdre_tools não gerou o arquivo de saída do patch.".into())
    }
}

/// Converte um .po em .translation usando o próprio gdre_tools
/// (--txt-to-bin não serve para po; usamos o fluxo de recover com patch-translations
/// seria complexo). Por ora, tenta `--patch-translations` em modo offline exige PCK;
/// assim, esta função delega para o builder godot existente no chamador quando
/// disponível. Mantida aqui para futura implementação via gdre.
/// Converte `project.binary` (empacotado) para texto (`project.godot`).
/// `input` deve ser o caminho LOCAL do arquivo binário; a saída é gravada
/// ao lado com extensão `.godot`. Retorna o caminho do texto.
pub fn bin_to_txt(input: &Path) -> Result<PathBuf, String> {
    // O gdre grava a saida no CWD do processo, nao ao lado do input. Entao
    // rodamos DENTRO da pasta do input para garantir que o arquivo caia la.
    let parent = input.parent().ok_or_else(|| "input sem diretorio pai".to_string())?;
    let args = vec![format!("--bin-to-txt={}", input.display())];
    run_with_cwd(&args, Some(parent))?;
    let out = input.with_extension("godot");
    if out.is_file() { Ok(out) } else { Err(format!("bin-to-txt não gerou {}", out.display())) }
}

/// Converte `project.godot` (texto) para `project.binary`. Retorna o caminho
/// do arquivo binário gerado.
pub fn txt_to_bin(input: &Path) -> Result<PathBuf, String> {
    // Mesmo motivo do bin_to_txt: gdre grava a saida no CWD.
    let parent = input.parent().ok_or_else(|| "input sem diretorio pai".to_string())?;
    let args = vec![format!("--txt-to-bin={}", input.display())];
    run_with_cwd(&args, Some(parent))?;
    let out = input.with_extension("binary");
    if out.is_file() { Ok(out) } else { Err(format!("txt-to-bin não gerou {}", out.display())) }
}

/// Compila um script GDScript para bytecode (.gdc) mirando uma versão exata
/// do engine (ex.: "4.7.0"). Retorna o caminho do .gdc gerado.
pub fn compile_gd(input: &Path, bytecode_version: Option<&str>) -> Result<PathBuf, String> {
    let mut args = vec![format!("--compile={}", input.display())];
    if let Some(v) = bytecode_version {
        args.push(format!("--force-bytecode-version={}", v));
    }
    run(&args)?;
    let out = input.with_extension("gdc");
    if out.is_file() { Ok(out) } else { Err(format!("Compilação não gerou {}", out.display())) }
}

#[allow(dead_code)]
pub fn version() -> Option<String> {
    let bin = locate()?;
    let out = crate::paths::hidden_command(bin).arg("--version").output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    s.lines().next().map(|l| l.trim().to_string())
}
