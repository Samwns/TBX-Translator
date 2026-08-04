# TPG Translator - Release Notes

**Creator:** samwns  
**Project:** TPG Translator  
**License:** Private

---

## Release 2.0.0 — Rust/GTK4 Native Port (Current)

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
./target/release/tpg-translator
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
