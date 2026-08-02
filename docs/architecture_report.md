# TPG Translator - Architecture Report

## Overview
This document outlines the new architectural design of the TPG Translator. Due to rendering bugs and broken visual fidelity issues with the previous Tauri (Web-based) approach, the application has been entirely rewritten using a pure, native Rust approach powered by **GTK4**.

## Why Native GTK4?
1. **Performance & Reliability:** Unlike Electron or Tauri, GTK4 does not require an embedded web browser (WebView). The UI is compiled down to native machine code.
2. **Visual Fidelity:** The previous JavaFX UI was heavily customized with a dark theme and a frameless window. Translating this directly to Web CSS resulted in a broken appearance. GTK4 natively supports CSS styling (`CssProvider`), making it trivial to replicate the JavaFX aesthetics cleanly and authentically on Linux.
3. **No Node.js Required:** The build process is streamlined to a single command (`cargo run`) without needing NPM, Node.js, or complex IPC bindings between JavaScript and Rust.

## Architecture Structure

- **`src/main.rs`**: The entry point. Initializes the GTK Application loop and mounts the main window.
- **`src/ui.rs`**: The declarative UI layer. Defines the widgets (Labels, Entries, Buttons), layout containers (Box), and loads the custom CSS (`window { background-color: #1e1e2e; }`) to maintain the signature dark theme.
- **`src/api.rs`**: Asynchronous backend module for communicating with the Google Translate API using `reqwest` and `tokio`.
- **`src/renpy_extractor.rs` & `src/unity_extractor.rs`**: The core logic ports from the original Java engine, adapted for Rust.

## Development & Building

To run the application natively, simply execute:
```bash
cargo run
```

To build a release optimized executable:
```bash
cargo build --release
```
*The resulting binary will be located in `target/release/tpg-translator`.*

## Future Work
- **Async UI Bridging:** Use `glib::MainContext::channel()` to allow background `tokio` threads (doing the translation/extraction) to safely update GTK progress labels in the main UI thread.
- **File Chooser Dialog:** Implement `gtk::FileChooserNative` inside the "PROCURAR" button callback to seamlessly pick `.exe` or `.py` files.
