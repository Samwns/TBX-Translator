# Week 02 — July 28–31, 2026

> The critical week. Built the Unity C# extractor from scratch, diagnosed why the GOMI game was extracting garbage instead of dialogue, rewrote the entire text filter, migrated BepInEx to local ZIPs, and added Yarn Spinner variable protection.

---

## July 28, 2026 (Monday)

### ✨ Unity C# Extractor (`unity_static_extractor/`)

Created a standalone .NET 8.0 C# project to handle Unity asset extraction. This was necessary because Unity's binary serialization format cannot be read from Rust directly — the only mature library for this is **AssetsTools.NET** (C#).

**Project structure:**
```
unity_static_extractor/
├── Program.cs                    # Main extractor logic (~600 lines)
├── unity_static_extractor.csproj # .NET 8.0 project file
└── bin/Debug/net8.0/             # Compiled output
```

**Dependencies:**
- `AssetsTools.NET` (v3.0.0) — Read/write Unity `.assets` and `.bundle` files
- `Mono.Cecil` (v0.11.6) — Inspect compiled .NET DLLs (read IL instructions)

**Two operation modes:**
1. **`extract`** — Reads game data folder, extracts all translatable strings to JSON
2. **`inject`** — Reads translated JSON, writes modifications back to `.assets` files

**Extraction pipeline:**
```
Game Data Folder
  ├── Assembly-CSharp.dll ──→ Mono.Cecil reads Ldstr instructions
  ├── sharedassets0.assets ──→ AssetsTools reads MonoBehaviours (type 114)
  ├── sharedassets1.assets ──→ AssetsTools reads TextAssets (type 49)
  ├── level0 ──→ Scene assets
  └── level1 ──→ Scene assets
```

**Integration with Rust:**
```rust
// Called from unity_extractor.rs
Command::new("dotnet")
    .arg("run")
    .arg("--project").arg("unity_static_extractor")
    .arg("--").arg("extract")
    .arg(data_folder)
    .arg(output_json)
    .spawn()
```

### 🐛 Problem Identified: `IsValidText()` Filter Too Aggressive

The first test on the game **"Get Off My Island" (GOMI 0.4)** produced only **282 strings**, almost all of which were programmer debug messages and internal variable names. No actual game dialogue was captured.

**Root cause analysis:**
The `IsValidText()` method in `Program.cs` had rules that specifically blocked the exact patterns used by real game dialogue:

```csharp
// These rules KILLED all Yarn Spinner dialogue:
if (s.Contains("{0}")) return false;  // Yarn variables like "{0}: Hello {1}"
if (s.Contains("{1}")) return false;  // Player name substitutions

// These rules KILLED all rich text UI:
if (s.StartsWith("<color=")) return false;  // Unity color tags
if (s.StartsWith("<size="))  return false;  // Unity size tags

// This rule KILLED legitimate game text:
if (s.Contains("Missing")) return false;    // "Missing something?" dialogue
```

### ⚡ Decision: Class Name Filter is Fundamentally Broken

The extractor also had a class name filter that only processed MonoBehaviours whose `m_ClassName` contained specific keywords:

```csharp
// Only these classes were scanned:
"Text", "Label", "Dialogue", "TMP_Text", "Localization",
"Database", "Data", "String", "Story"
```

This completely missed:
- Yarn Spinner runtime components (class `DialogueRunner`, `DialogueUI`)
- Custom game-specific dialogue systems
- ScriptableObjects storing dialogue data
- Any component that stores text in custom fields

---

## July 29, 2026 (Tuesday)

### 🔍 Deep Diagnosis: GOMI 0.4 Asset Analysis

Performed a thorough binary analysis of the game's asset files to understand where the real dialogue content lives.

**Findings:**

| Asset File | Content | String Count |
|------------|---------|--------------|
| `sharedassets0.assets` | Main game assets, materials, textures | ~50 UI strings |
| `sharedassets1.assets` | **Yarn Spinner dialogue database** | **141 dialogue lines** |
| `level0` | Main menu scene, warning screen | ~30 UI strings |
| `level1` | Gameplay scene | ~20 UI strings |
| `Assembly-CSharp.dll` | Compiled game code (`Ldstr` instructions) | ~200 code strings |
| `StreamingAssets/aa/` | Unity Addressables / Localization bundles | Unknown |

**Key discovery:** The game uses **Yarn Spinner** as its dialogue system. Yarn Spinner stores dialogue in a custom binary format inside MonoBehaviour components, with runtime variable substitution using `{0}`, `{1}`, `{2}` placeholders.

**Example real dialogue from GOMI:**
```
You feel a funny feeling...
Something isn't quite right.
The trees are listening.
Death is only a beginning.
How did you get to this island?
Oh! You wanna fuck me?
Pick up your friend!
```

All of these were being blocked by the `{0}` / `{1}` filter or the class name filter.

---

## July 31, 2026 (Thursday)

### 🐛 Critical Fix: `IsValidText()` Complete Rewrite

Rewrote the text filter to be permissive for game content while still blocking technical garbage.

**Rules REMOVED (were blocking real dialogue):**

| Rule | What it killed |
|------|---------------|
| `Contains("{0}")` | All Yarn Spinner dialogue with variable substitution |
| `Contains("{1}")` | Player name references in dialogue |
| `StartsWith("<color=")` | Colored UI text ("This project is in active development") |
| `StartsWith("<size=")` | Sized UI text |
| `Contains("Missing")` | Legitimate dialogue lines |

**Rules KEPT (correctly filter garbage):**

| Rule | What it blocks |
|------|---------------|
| UUID pattern (`xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`) | Unity asset GUIDs |
| Hex hash (32+ hex chars) | MD5/SHA256 hashes |
| Input paths (`/Gameplay/`, `<Keyboard>/`) | Unity Input System paths |
| File extensions (`.png`, `.dll`, `.cs`, `.shader`) | Asset file references |
| URLs (`://`) | Internal Unity URLs |
| Pure numbers | Numeric-only strings |
| Single characters | One-letter strings |
| All-uppercase single words | Enum values, constants |

### ✨ Recursive Field Scanner: `ExtractStringsFromField()`

Replaced the shallow `m_Text` / `text` field check with a recursive scanner that walks the **entire field tree** of every MonoBehaviour component.

**How it works:**
```
MonoBehaviour (type 114)
├── m_Name: "DialogueRunner"          ← BLACKLISTED (internal)
├── m_Script: {fileID: 123}           ← BLACKLISTED (internal)
├── m_GameObject: {fileID: 456}       ← BLACKLISTED (internal)
├── dialogueText: "Hello world!"      ← EXTRACTED ✅
├── buttonLabel: "Continue"           ← EXTRACTED ✅
├── responses: [                      ← RECURSE INTO ARRAY
│   ├── text: "Yes, tell me more"     ← EXTRACTED ✅
│   └── text: "No way!"              ← EXTRACTED ✅
│]
└── metadata: {
    ├── m_Shader: "Standard"          ← BLACKLISTED (internal)
    └── tooltip: "Click to interact"  ← EXTRACTED ✅
}
```

**Blacklisted internal field names (30+):**
```csharp
m_Name, m_Script, m_GameObject, m_FileID, m_PathID,
m_Father, m_Children, m_Shader, m_Material, m_Materials,
m_Texture, m_Sprite, m_Font, m_FontAsset, m_Mesh,
m_RootOrder, m_LocalPosition, m_LocalRotation, m_LocalScale,
m_Tag, m_Layer, m_IsActive, m_Enabled, m_CastShadows,
m_ReceiveShadows, m_LightProbeUsage, m_CorrespondingSourceObject,
m_PrefabInstance, m_PrefabAsset, m_EditorHideFlags, m_ObjectHideFlags,
m_ClassName, m_Namespace, m_AssemblyName
```

**Recursion limit:** Maximum depth of 8 levels to prevent infinite loops in circular references.

### 📊 Extraction Results: Before vs After

Tested on GOMI 0.4 (`GOMI_Data/`):

| Metric | Before (v2.0) | After (v2.1) | Change |
|--------|---------------|--------------|--------|
| Total strings extracted | 282 | **3,825** | +1,256% |
| Yarn Spinner dialogue lines | 0 | **160** | ∞ |
| Rich text UI strings | 0 | **19** | ∞ |
| Screenshot text found | 0 | **2** | ∞ |
| False positive debug strings | ~200 | **~6** | -97% |

**Screenshot text that was finally captured:**
```
"<color=#A3FF97>This project is in active development</color>. Features are being added and changed."
"No way!"
```

### ✨ BepInEx: Migration to Local ZIP Extraction

Completely replaced the internet-download approach with local ZIP extraction.

**Old approach (REMOVED):**
```rust
// Downloaded from GitHub on every injection — slow, fragile, requires internet
async fn download_and_extract_zip(url: &str, target_dir: &Path, tx: Sender) { ... }
```

**New approach (ADDED):**
```rust
// Extracts from ZIPs bundled with the app
fn extract_local_zip(zip_path: &Path, target_dir: &Path) -> Result<(), String> { ... }
```

**ZIP selection logic:**

| Backend | BepInEx ZIP | XUnity ZIP |
|---------|-------------|------------|
| Mono | `BepInEx_win_x64_5.4.23.5.zip` | `XUnity.AutoTranslator-BepInEx-5.6.1.zip` |
| IL2CPP | `BepInEx-Unity.IL2CPP-win-x64-6.0.0-be.785.zip` | `XUnity.AutoTranslator-BepInEx-IL2CPP-5.6.1.zip` |

The functions `find_local_bepinex_zip()` and `find_local_xunity_zip()` automatically scan the bundled folders and pick the right ZIP based on keywords in the filename (`il2cpp`, `mono`, `5.4`, `6.0`).

### 💀 Dead Code Removed

- `download_and_extract_zip()` function deleted from `unity_extractor.rs` (was the only caller)

### ✨ Yarn Spinner Variable Protection During Translation

Added a placeholder system to prevent the Google Translate API from corrupting game variables and rich text tags.

**Before sending to API:**
```
Original:  "{0}: Hello! How are you {1}?"
Protected: "TBXVAR0: Hello! How are you TBXVAR1?"

Original:  "<color=#A3FF97>Warning</color>"
Protected: "TBXTAG0Warning TBXTAG1"
```

**After receiving translation:**
```
API response: "TBXVAR0: Olá! Como você está TBXVAR1?"
Restored:     "{0}: Olá! Como você está {1}?"

API response: "TBXTAG0Aviso TBXTAG1"
Restored:     "<color=#A3FF97>Aviso</color>"
```

**Implementation:** Each string is pre-processed before the API call. A list of `(original, placeholder)` pairs is stored per string and used to restore after translation.

### 🐛 Editor Crash Fix: `char_indices()` vs `chars().enumerate()`

**The bug:**
```
thread 'main' panicked at src/editor_ui.rs:209:46:
start byte index 39 is not a char boundary; it is inside '"' (bytes 38..41)
```

**Root cause:** The function `find_unescaped_equals()` used `chars().enumerate()` which returns the **character index** (counting Unicode codepoints). But the return value was used to slice the string with `&s[..idx]`, which requires a **byte index**. For ASCII strings these are the same, but for strings containing multi-byte UTF-8 characters (like `"` which is 3 bytes, or `é`, `ç`, etc.), the indices diverge.

**Fix:** Changed to `char_indices()` which returns `(byte_offset, char)` tuples:
```rust
// Before (BROKEN):
for (i, c) in s.chars().enumerate() { ... }

// After (FIXED):
for (idx, c) in s.char_indices() { ... }
```

### ✨ AutoTranslatorConfig.ini: Auto-Generation & Update

**Problem:** On Linux, the game executable (`.exe`) cannot be launched directly to generate BepInEx config files. When users manually installed BepInEx and ran the game via PortProton, the XUnity AutoTranslator generated a default config with `Language=en` and `FromLanguage=ja` — completely wrong for Portuguese translation.

**Solution — two-path logic:**

1. **Config doesn't exist:** Create it from scratch with correct language settings
```ini
[Service]
Endpoint=GoogleTranslateV2

[General]
Language=pt
FromLanguage=en

[TextFrameworks]
EnableUGUI=True
EnableIMGUI=True
EnableTextMeshPro=True
EnableTextMesh=True
EnableFairyGUI=True
```

2. **Config already exists (generated by game):** Update only `Language=` and `FromLanguage=` fields via regex, preserving all other settings (like `MaxCharactersPerTranslation`, `IgnoreWhitespaceInDialogue`, etc.)

```rust
let re_lang = regex::Regex::new(r"(?m)^Language=.*").unwrap();
let re_from = regex::Regex::new(r"(?m)^FromLanguage=.*").unwrap();
content = re_lang.replace(&content, format!("Language={}", target_code)).to_string();
content = re_from.replace(&content, "FromLanguage=en").to_string();
```

### ✨ Linux/Proton/PortProton Instructions

Added console warnings for Linux users:
```
[Unity] IMPORTANT (LINUX/PROTON): To make BepInEx work, add to launch options:
[Unity] WINEDLLOVERRIDES="winhttp=n,b" %command%
```

This is necessary because Wine/Proton by default loads its own `winhttp.dll` instead of the BepInEx one (`winhttp.dll` is BepInEx's entry point — it intercepts the game's DLL loading to inject itself).

---

## 📊 End of Week Status

| Component | Status | Notes |
|-----------|--------|-------|
| Unity C# extractor | ✅ Working | 3,825 strings from GOMI |
| IsValidText filter | ✅ Rewritten | Permissive for dialogue, strict for garbage |
| Recursive field scanner | ✅ Working | Walks full MonoBehaviour trees |
| BepInEx local install | ✅ Working | No internet required |
| Variable protection | ✅ Working | {0}, {1}, rich text tags preserved |
| AutoTranslator config | ✅ Working | Auto-create or auto-update |
| Editor crash fix | ✅ Fixed | char_indices() instead of enumerate() |
| PortProton integration | 🔧 Testing | Game still showing English (config was wrong) |

### Files Created/Modified

| File | Action | Lines Changed | Description |
|------|--------|---------------|-------------|
| `unity_static_extractor/Program.cs` | Modified | ~150 | IsValidText rewrite, ExtractStringsFromField, class filter removed |
| `src/unity_extractor.rs` | Modified | ~200 | Local ZIPs, variable protection, config generation |
| `src/editor_ui.rs` | Modified | ~5 | char_indices() fix |
