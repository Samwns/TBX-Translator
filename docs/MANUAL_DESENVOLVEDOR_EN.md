# Developer Manual - TBX Translator

This document describes the internals and architecture of TBX Translator.

## General Architecture

The application is written in **Rust** and divided into modules, each with a well-defined responsibility:

- `main.rs`: The application's entry point. Initializes the GTK environment (via `gtk::Application`).
- `ui.rs`: The main User Interface file. Builds windows, widgets, and connects the primary events.
- `app_config.rs`: Handles loading and saving user configurations in JSON format, persisting options like themes, directories, and API modes.
- `renpy_extractor.rs`: Handles the reverse engineering of games using the Ren'Py engine (Python script injection, dump extraction, batch API translation, and `.rpy` regeneration).
- `unity_extractor.rs`: Handles extraction and configuration for games using the Unity engine, primarily interacting with the AutoTranslator plugin.
- `font_injector.rs`: Auxiliary window and patch logic to inject custom fonts (`.ttf`, `.otf`) into engines, fixing rendering issues with accented characters.
- `api.rs`: HTTP communication via `reqwest` with the Google API for translating text blocks.

---

## How the App Works

### 1. Initialization and Interface
Upon running the binary, GTK initializes the main window. The UI does not use visual builders (like Glade or XML). The entire interface is drawn purely via Rust code in `ui.rs`. The window is "frameless" (custom decorated), using `GestureClick` for title bar movement and removing native OS frames.

### 2. Engine Selection
The user can choose different tabs for Ren'Py and Unity. This choice defines the `engine_mode`.
Depending on the tab, the "Translate" button passes commands to either `renpy_extractor` or `unity_extractor`.

### 3. Extraction and Translation (Ren'Py)
1. **Injection:** The app temporarily inserts `tbx_dumper.rpy` to collect text and generates `tbx_boot.rpy` to integrate the translation without forcing the player's selected language.
2. **Headless Execution:** The `renpy_extractor` starts the game executable hidden (`xvfb-run` on Linux, or silent arguments) just long enough for Python to run and generate a log containing the texts (`dump.txt`).
3. **Parse and Filter:** The `dump.txt` file is read by Rust. Complex rules are applied to protect `{b}...{/b}` tags and `[player_name]` script variables. This is done by replacing them with temporary numerical markers (e.g., `777001777`) before sending them to the API.
4. **Translation:** Text blocks are dispatched to `api.rs`, using multi-threading or sequential batches depending on the configuration.
5. **Reconstruction:** After translation, the numerical markers are converted back to the originals.
6. **Deploy:** The original Ren'Py translation files (`.rpy`) are generated, which the game will compile on the next launch, ensuring exact key compatibility (`old "..."`).

### 4. Font Correction (Font Injector)
In American or Japanese games, the native font does not know how to draw "ã", "é", "ç", etc., making the letters invisible.
The `font_injector.rs` module copies a font from the user's computer to the game root, creating an `.rpy` patch (in Ren'Py) or manipulating the `Config.ini` (in Unity) to force graphical interfaces to replace the font family. GTK4 supports dynamic CSS (`@font-face`), so this module can instantly show the user a Font Preview before they even inject it into the game.

---

## How we use the GTK Library (gtk4-rs)

The UI construction is 100% declarative and reactive via Rust closures:

### Widgets and Hierarchy
Instead of defining separate XMLs, containers are nested in Rust. For example, the window receives a vertical `Box`, which receives the `Stack` and the custom `HeaderBar`.
```rust
let root = Box::new(Orientation::Vertical, 0);
let title_bar = Box::new(Orientation::Horizontal, 0);
root.append(&title_bar);
window.set_child(Some(&root));
```

### CSS Styling (CssProvider)
All formatting (colors, rounded borders, shadows, hover effects) is defined in `.css` files or native strings, applied globally or per widget via `CssProvider`.
The application loads the `style.css` file (or uses an in-memory fallback) at startup:
```rust
let provider = gtk::CssProvider::new();
provider.load_from_data(include_str!("style.css"));
gtk::style_context_add_provider_for_display(
    &gdk::Display::default().unwrap(),
    &provider,
    gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
);
```

### Cloning Variables for Closures (Callbacks)
The biggest difficulty in developing with `gtk-rs` in Rust is the strict ownership rules (variable borrowing).
To handle buttons altering other UI elements, we use **Smart Clones** (Reference Counted with RC).
```rust
let input_box = Entry::new();
let button = Button::new();

// We need to clone memory references before throwing into the move closure
let ib = input_box.clone(); 
button.connect_clicked(move |_| {
    ib.set_text("Button clicked!");
});
```

### Safe Multi-threading (Channels)
GTK runs on a strict "Main Loop". You cannot update a progress bar or log box from a Thread making heavy requests (this would crash the UI or generate strange behavior in C).
To extract a game in the Background (asynchronous/parallel) without freezing the app, the flow used is:
1. We create a native GTK channel, `glib::MainContext::channel`.
2. The UI gets the *Receiver* (rx), and triggers the Extraction process in a separate Thread (passing the *Sender* tx).
3. The *Background Thread* uses `tx.send(Message)` sending data (log strings or integers for progress).
4. The `rx.attach()` running on the GTK Main Thread listens for this data, and only it natively changes the Labels or Progress Bars.
This flow ensures multi-thread performance, safe memory protection, and a 100% fluid interface without freezing.
