# Week 01 — July 26–27, 2026

> Project birth. Full migration from Java/JavaFX to Rust/GTK4. Renamed from "TPG Translator" to "TBX - Translator". Built the base UI, translation editor, and font injector module.

---

## July 26, 2026 (Saturday)

### ⚡ Project Rename: TPG → TBX

The project was officially renamed from **"TPG Translator"** to **"TBX - Translator"** (Toolbox Translator). This was done to better reflect the app's purpose as a multi-engine translation toolbox rather than being tied to a specific group name.

**Files changed:**
- `Cargo.toml` — Updated `name = "tbx-translator"`, `version = "2.0.0"`, `description = "TBX Translator : Toolbox - Translator"`
- `src/ui.rs` — All window titles and internal references updated
- All documentation headers updated

### ✨ Architecture Review

Performed a full audit of the existing codebase that had already been migrated from Java/JavaFX to Rust/GTK4. The application is a single-binary native desktop app with no web dependencies.

**Module breakdown:**

| Module | Size | Purpose |
|--------|------|---------|
| `main.rs` | ~450B | Entry point, GTK Application init |
| `ui.rs` | ~65KB | Entire UI built in pure Rust GTK4 code (no XML/Glade) |
| `api.rs` | ~3.3KB | Google Translate API communication via `reqwest` |
| `app_config.rs` | ~5.2KB | JSON-based persistent config (`~/.tbx-translator/`) |
| `renpy_extractor.rs` | ~10.7KB | Ren'Py extraction engine |
| `unity_extractor.rs` | ~11KB | Unity extraction engine (initial version) |
| `font_injector.rs` | ~28KB | Custom font injection with preview |
| `editor_ui.rs` | ~11.7KB | Manual translation editor |
| `i18n.rs` | ~4.4KB | UI internationalization |
| `desired_python.py` | ~8.1KB | Python script injected into Ren'Py games |

### ✨ GTK4 UI Confirmation

The UI was confirmed working with the following characteristics:
- **Theme:** Dark mode using Catppuccin Mocha palette via GTK `CssProvider`
- **Window:** Frameless (no native decorations), custom drag via `GestureClick` on the title bar
- **Layout:** Tabbed interface with separate panels for Ren'Py and Unity engines
- **Stack-based navigation:** Translate → Logs → Tools → Settings
- **Rendering:** 100% native, no WebView/Electron/Tauri — compiled directly to machine code

---

## July 27, 2026 (Sunday)

### ✨ Font Injector Module (`font_injector.rs`)

Built the complete font injection system to solve the problem where games with ASCII-only fonts can't render accented characters (ã, é, ç, ñ, etc.) after translation.

**How it works:**
1. User selects a `.ttf` or `.otf` font file from their system
2. The app shows a **live preview** of the font rendering sample text with accented characters
3. On injection:
   - **Ren'Py:** Copies the font to the game's `game/` directory and generates a `tl/XX/style.rpy` file that overrides `gui.text_font` and `gui.name_text_font`
   - **Unity:** Copies the font and updates `BepInEx/config/AutoTranslatorConfig.ini` to reference the custom font

**Technical details:**
- Preview is generated using the `rusttype` crate for font rasterization
- GTK4 `@font-face` CSS is used for real-time font preview in the UI
- Supports Unicode coverage detection to warn if the font is missing characters

### ✨ Translation Editor (`editor_ui.rs`)

Built a manual translation editor that allows users to review, edit, and save translated texts before injecting them into the game.

**Supported file formats:**

| Format | Extension | Structure | Engine |
|--------|-----------|-----------|--------|
| Ren'Py translation | `.rpy` | `old "..." / new "..."` blocks | Ren'Py |
| XUnity dictionary | `.txt` | `Original=Translation` per line | Unity |
| JSON map | `.json` | `{"original": "translated"}` object or `["string"]` array | Unity |

**UI Components:**
- `gtk4::ListBox` with dynamic rows
- Each row has two `Entry` fields: "Original" (read-only) and "Translated" (editable)
- Changes are tracked via `Rc<RefCell<Vec<DialogData>>>` for safe cross-closure mutation
- Save button writes back to the original format

### ✨ Ren'Py Extraction Tests

Full end-to-end testing of the Ren'Py extraction pipeline:

1. **Injection:** `desired_python.py` is injected into the game's `renpy/` directory
2. **Boot script:** `tpg_boot.rpy` is created to force the game to execute the dump script
3. **Headless execution:** Game is run via `xvfb-run` (Linux) for ~15 seconds to generate text dumps
4. **Parsing:** `dump.txt` is read and filtered
5. **Variable protection:** Special markers are replaced with numeric placeholders before translation:
   - `{b}` → `777001777`, `{/b}` → `777002777`
   - `{i}` → `777003777`, `{/i}` → `777004777`
   - `[player_name]` → `777010777`
6. **Translation:** Batched via Google Translate API
7. **Reconstruction:** Placeholders restored, `.rpy` translation files generated

---

## 📊 End of Week Status

| Component | Status |
|-----------|--------|
| Ren'Py extraction | ✅ Working |
| Ren'Py translation | ✅ Working |
| Ren'Py injection | ✅ Working |
| Unity extraction | 🔧 Basic (needs filter fix) |
| Unity injection | 🔧 Basic (uses HTTP download) |
| Translation editor | ✅ Working |
| Font injector | ✅ Working |
| GTK4 UI | ✅ Working |

### Files Created/Modified

| File | Action | Description |
|------|--------|-------------|
| `Cargo.toml` | Modified | Name, version, description updated |
| `src/ui.rs` | Modified | All TPG→TBX references |
| `src/font_injector.rs` | Created/Modified | Complete font injection module |
| `src/editor_ui.rs` | Created/Modified | Manual translation editor |
| `src/renpy_extractor.rs` | Modified | Variable protection system |
| `src/desired_python.py` | Modified | Dump script improvements |
