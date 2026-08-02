# Week 03 — August 01–02, 2026

> Documentation week. Reorganized the entire `docs/` folder, created the development diary structure, updated all references from TPG to TBX, and finalized PortProton testing notes.

---

## August 01, 2026 (Friday)

### 🔍 PortProton Testing Session

Continued testing the BepInEx + XUnity AutoTranslator integration on the GOMI 0.4 game running via PortProton on Linux.

**Findings:**
- BepInEx was installed correctly in the game folder (`BepInEx/core/`, `BepInEx/plugins/`)
- XUnity AutoTranslator plugin was present (`BepInEx/plugins/XUnity.AutoTranslator/`)
- `BepInEx/config/BepInEx.cfg` was generated (BepInEx itself was loading)
- `BepInEx/config/AutoTranslatorConfig.ini` existed but had **wrong language settings**:
  - `Language=en` (should be `pt`)
  - `FromLanguage=ja` (should be `en`)
- `BepInEx/LogOutput.log` was **empty** — suggesting BepInEx loaded but XUnity didn't fully initialize
- Translation files were in the correct location (`BepInEx/Translation/pt/Text/`)

**Root cause of "still in English":**
The AutoTranslator was configured to translate FROM Japanese TO English. Since the game is already in English, it had nothing to do. The fix was to set `Language=pt` and `FromLanguage=en`.

### ⚡ Decision: Always Update Config on Injection

Previously, the TBX Translator only created the config file if it didn't exist. This was wrong because:
1. The game generates the config on first BepInEx boot with default (wrong) values
2. The user may change the target language between runs

**New behavior:** On every injection, the TBX Translator:
1. Checks if `AutoTranslatorConfig.ini` exists
2. If yes → updates `Language=` and `FromLanguage=` via regex (preserves all other settings)
3. If no → creates it from scratch with correct values

---

## August 02, 2026 (Saturday)

### ✨ Full Documentation Reorganization

Restructured the entire `docs/` folder from a flat collection of files into an organized hierarchy.

**Old structure:**
```
docs/
├── MANUAL_DESENVOLVEDOR_EN.md
├── MANUAL_DESENVOLVEDOR_PTBR.md
├── architecture_report.md
├── api/
│   └── API_MODULE.md
├── changelog/
│   └── CHANGELOG.md
├── releases/
└── screenshots/
```

**New structure:**
```
docs/
├── MANUAL_DESENVOLVEDOR_EN.md      # Developer manual (English)
├── MANUAL_DESENVOLVEDOR_PTBR.md    # Developer manual (Portuguese)
├── api/
│   └── API_MODULE.md               # API module documentation
├── arquitetura/
│   └── ARQUITETURA.md              # Architecture document (updated)
├── changelog/
│   └── CHANGELOG.md                # Version history (updated with v2.1.0)
├── diary/                          # NEW: Development diary
│   ├── README.md                   # Diary index
│   ├── week-01/
│   │   └── README.md               # Jul 26-27: Project birth
│   ├── week-02/
│   │   └── README.md               # Jul 28-31: Unity extractor & fixes
│   └── week-03/
│       └── README.md               # Aug 01-02: Documentation & testing
├── releases/
└── screenshots/
```

### ✨ README.md Rewrite

The root `README.md` was completely rewritten from a generic Tauri template to a comprehensive project description including:
- Project purpose and description
- Supported engines table (Ren'Py, Unity Mono, Unity IL2CPP)
- Build instructions
- Full project directory structure
- Links to all documentation
- Technology stack

### ✨ Changelog Update

Updated `docs/changelog/CHANGELOG.md` with:
- **v2.1.0** (2026-07-31) — All Unity extractor fixes, BepInEx local, variable protection
- **v2.0.0** (2026-07-26) — Rust/GTK4 migration, full feature list
- **v1.0.0** (Legacy) — Java/JavaFX original release

### 📝 Documentation Fixes

All documentation files were audited and updated:
- All references to "TPG Translator" changed to "TBX - Translator"
- Architecture report updated with current module list
- API documentation verified against current `api.rs` implementation

---

## 📊 End of Week Status

| Component | Status | Notes |
|-----------|--------|-------|
| Ren'Py extraction | ✅ Working | Full pipeline tested |
| Ren'Py translation | ✅ Working | Variable protection active |
| Ren'Py injection | ✅ Working | `.rpy` files generated |
| Unity extraction | ✅ Working | 3,825 strings from GOMI |
| Unity translation | ✅ Working | Yarn variables protected |
| Unity BepInEx install | ✅ Working | Local ZIPs, no internet |
| Unity config generation | ✅ Working | Auto-create or auto-update |
| Translation editor | ✅ Working | `.rpy`, `.txt`, `.json` formats |
| Font injector | ✅ Working | Preview + injection |
| GTK4 UI | ✅ Working | Dark theme, frameless |
| Documentation | ✅ Complete | Diary, changelog, README, architecture |
| PortProton integration | 🔧 Needs user testing | Config fixed, awaiting confirmation |

### Files Created/Modified

| File | Action | Description |
|------|--------|-------------|
| `README.md` | Rewritten | Full project description |
| `docs/diary/README.md` | Created | Diary index |
| `docs/diary/week-01/README.md` | Created | Week 1 detailed log |
| `docs/diary/week-02/README.md` | Created | Week 2 detailed log |
| `docs/diary/week-03/README.md` | Created | Week 3 detailed log |
| `docs/changelog/CHANGELOG.md` | Updated | Added v2.1.0 |
| `docs/arquitetura/ARQUITETURA.md` | Updated | Current architecture |
