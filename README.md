<div align="center">
  <img src="assets/com.tbx.translator.svg" width="96" height="96" alt="TBX Translator">

  # TBX Translator

  **Extract, translate, review, and install translations for Ren'Py, Unity, and Godot games.**

  [![Version](https://img.shields.io/github/v/release/Samwns/TBX-Translator?display_name=tag&sort=semver&label=version)](https://github.com/Samwns/TBX-Translator/releases/latest)
  [![Downloads](https://img.shields.io/github/downloads/Samwns/TBX-Translator/total?label=downloads)](https://github.com/Samwns/TBX-Translator/releases/latest)
  [![License](https://img.shields.io/badge/license-CC%20BY--NC--SA%204.0-lightgrey)](LICENSE)

  [![Website](https://img.shields.io/badge/Website-Open-89B4FA?logo=googlechrome&logoColor=white)](https://samwns.github.io/TBX-Translator/)
  [![GitHub Releases](https://img.shields.io/badge/GitHub-Releases-313244?logo=github&logoColor=white)](https://github.com/Samwns/TBX-Translator/releases/latest)
  [![Ko-fi](https://img.shields.io/badge/Ko--fi-Support%20the%20project-FF5E5B?logo=ko-fi&logoColor=white)](https://ko-fi.com/samwns)

  **English** · [Português (Brasil)](README.pt-BR.md)
</div>

## Features

- Ren'Py dialogue extraction, language-menu integration, and font injection.
- Unity Mono/IL2CPP support through AssetsTools.NET, UnityPy, BepInEx, and XUnity AutoTranslator.
- Godot native catalogs, PO files, PCK packages, binary resources, and story files.
- Visual editor for reviewing `.rpy`, `.txt`, and `.json` files before installation.
- Preservation of variables, tags, BBCode, whitespace, and formatting.
- Independent engine tasks with cache, controlled concurrency, and cancellation.
- In-app updates and an interface available in 104 languages.

## Download

The [latest release](https://github.com/Samwns/TBX-Translator/releases/latest) provides:

- Portable Windows package (`.zip`) and installer (`.exe`)
- Debian/Ubuntu package (`.deb`)
- Fedora package (`.rpm`)
- Arch Linux package (`.pkg.tar.zst`)
- Portable Linux AppImage (`.AppImage`)

The application, packages, and releases derive their version from `Cargo.toml`.
GitHub Actions appends the actual build number automatically.

## Development

Development requires stable Rust. The .NET 8 SDK is also required for the Unity extractor.

```bash
# Fedora
sudo dnf install dotnet-sdk-8.0

# Debian/Ubuntu
sudo apt install dotnet-sdk-8.0 libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev

cargo run
```

Create every package supported on the current platform with:

```bash
cargo package-all
```

## Documentation

- [Documentation website](https://samwns.github.io/TBX-Translator/docs.html)
- [Developer manual](docs/MANUAL_DESENVOLVEDOR_EN.md)
- [Architecture](docs/arquitetura/ARQUITETURA.md)
- [API](docs/api/API_MODULE.md)
- [Changelog](docs/changelog/CHANGELOG.md)

## Website and support

- [Official website](https://samwns.github.io/TBX-Translator/)
- [GitHub Releases](https://github.com/Samwns/TBX-Translator/releases/latest)
- [Support development on Ko-fi](https://ko-fi.com/samwns)

GitHub also displays the **Sponsor** button through `.github/FUNDING.yml`.

## License

Distributed under [CC BY-NC-SA 4.0](LICENSE): non-commercial use and
redistribution with attribution and share-alike licensing.
