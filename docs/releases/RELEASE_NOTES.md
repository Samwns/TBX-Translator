# TBX Translator - Release Notes

**Creator:** samwns  
**Project:** TBX Translator  
**License:** Non-commercial source-available

---

## Release 0.0.1-alpha — Windows GTK4 Package

**Status:** Alpha

**Target Platform:** Windows x64  
**Framework:** GTK4 via MSYS2

The `Release Windows` GitHub Actions workflow produces a portable ZIP. Extract
it anywhere and run `tbx-translator.exe`; keep all included files and folders
next to the executable. The Unity extractor is bundled self-contained, so it
does not require the .NET SDK.

## Release 2.0.0 — Rust/GTK4 Native Port

**Status:** In Development  
**Target Platform:** Linux (Fedora/Arch/Debian)  
**Build System:** Cargo  
**Framework:** GTK4 via `gtk4-rs`

### Installation Requirements

On **Fedora / RHEL / Nobara**:
```bash
sudo dnf install gtk4-devel
```

On **Debian / Ubuntu / Mint**:
```bash
sudo apt install libgtk-4-dev
```

On **Arch Linux**:
```bash
sudo pacman -S gtk4
```

### Building and Running
```bash
cargo build --release
./target/release/tbx-translator
```

Or in development mode:
```bash
cargo run
```

---

## Release 1.0.0 — Java/JavaFX Legacy

**Status:** Archived in `backups/`  
**Platform:** Cross-platform (Windows/Linux with JRE)  
**Build System:** Maven  
**Framework:** JavaFX 21  

This version is kept as a reference in `backups/TPG - Translator/`.
