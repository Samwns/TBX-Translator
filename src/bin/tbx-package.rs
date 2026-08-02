use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let target = env::args().nth(1).unwrap_or_else(|| "all".into());
    let script = match target.as_str() {
        "linux" => "scripts/package-linux.sh",
        "windows" => "scripts/package-windows.sh",
        "all" => "scripts/package-all.sh",
        _ => {
            eprintln!("Uso: cargo package-[all|linux|windows]");
            return ExitCode::FAILURE;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let shell = if cfg!(windows) {
        env::var("TBX_MSYS_BASH")
            .unwrap_or_else(|_| r"C:\msys64\usr\bin\bash.exe".into())
    } else {
        "bash".into()
    };

    match Command::new(shell).arg(root.join(script)).status() {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(error) => {
            eprintln!("Não foi possível executar o script de release: {error}");
            ExitCode::FAILURE
        }
    }
}
