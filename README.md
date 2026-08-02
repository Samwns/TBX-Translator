<div align="center">
  <img src="assets/com.tbx.translator.svg" width="112" height="112" alt="Ícone do TBX Translator">

  # TBX Translator

  **Ferramenta desktop para extrair, traduzir e injetar textos em jogos Ren'Py e Unity, godot e unreal.**

  **Versão: `0.0.1-alpha`**

  [![Release](https://img.shields.io/github/v/release/Samwns/TBX-Translator?display_name=tag&sort=semver)](https://github.com/Samwns/TBX-Translator/releases/latest)
  [![License: CC BY-NC-SA 4.0](https://img.shields.io/badge/License-CC%20BY--NC--SA%204.0-lightgrey.svg)](LICENSE)
  [![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)](https://www.rust-lang.org/)
  [![GTK4](https://img.shields.io/badge/GTK-4-7FE719?logo=gtk)](https://www.gtk.org/)

  [Português](#sobre) · [English](#about)
  [![Apoie no Ko-fi](https://img.shields.io/badge/Ko--fi-Apoiar%20o%20projeto-FF5E5B?logo=ko-fi&logoColor=white)](https://ko-fi.com/samwns)
</div>

## Sobre

O TBX Translator é uma aplicação desktop nativa, feita em Rust e GTK4, para
automatizar a localização de jogos. Ela extrai textos, traduz via Google
Translate, permite a revisão manual e prepara a injeção no jogo.

- Extração e geração de traduções para jogos Ren'Py
- Extração de textos Unity Mono e IL2CPP com AssetsTools.NET
- Integração com BepInEx e XUnity AutoTranslator para Unity
- Editor visual para revisar arquivos `.rpy`, `.txt` e `.json`
- Proteção de variáveis, tags e formatação durante a tradução
- Injeção de fontes para caracteres acentuados
- Interface GTK4 nativa, sem Electron, WebView ou navegador embutido
- Interface em português e configurações persistentes locais

## Executar no Linux

Instale GTK4, Rust e .NET SDK 8 para usar também o extrator Unity:

```bash
# Fedora
sudo dnf install gtk4-devel dotnet-sdk-8.0

# Debian/Ubuntu
sudo apt install libgtk-4-dev dotnet-sdk-8.0

cargo run
```

Para uma compilação otimizada:

```bash
cargo build --release
```

## Releases: Linux e Windows

O workflow **Create release** gera os dois pacotes portáteis:

- `TBX-Translator-Linux-x64.tar.gz`, com o executável `TBX-Translator`;
- `TBX-Translator-Windows-x64.zip`, com `TBX-Translator.exe`, ícone e DLLs GTK4.

Ambos incluem os assets, o extrator Unity auto-contido e os ZIPs do
BepInEx/XUnity. No Linux, é necessário ter o runtime GTK4 instalado. Para
publicar, envie uma tag:

```bash
git tag v0.0.1-alpha
git push origin v0.0.1-alpha
```

Ou execute o workflow manualmente na aba **Actions** do GitHub. Ao fim, a mesma
release do GitHub terá os dois downloads.

## Documentação

- [Manual do desenvolvedor (PT-BR)](docs/MANUAL_DESENVOLVEDOR_PTBR.md)
- [Arquitetura](docs/arquitetura/ARQUITETURA.md)
- [Módulo de API](docs/api/API_MODULE.md)
- [Changelog](docs/changelog/CHANGELOG.md)
- [Diário de desenvolvimento](docs/diary/README.md)

## Licença

O repositório é público para consulta e colaboração, mas o software é
**source-available e não comercial**. Modificações e redistribuições gratuitas
são permitidas desde que mantenham esta licença e os créditos. Não é permitido
vender, cobrar pelo acesso, incluir em produto ou serviço pago, nem usar o
projeto ou versões modificadas para finalidade comercial sem autorização prévia
por escrito.
Veja [LICENSE](LICENSE).

## Apoie o projeto

Se o TBX Translator ajudar você, considere apoiar seu desenvolvimento no Ko-fi:

[![Apoie no Ko-fi](https://img.shields.io/badge/Ko--fi-Apoiar%20o%20projeto-FF5E5B?logo=ko-fi&logoColor=white)](https://ko-fi.com/samwns)

---

## About

TBX Translator is a native Rust and GTK4 desktop application that automates
game localization. It extracts text, translates it through Google Translate,
allows manual review, and prepares the translation for injection into the game.

- Text extraction and translation generation for Ren'Py games
- Unity Mono and IL2CPP text extraction using AssetsTools.NET
- BepInEx and XUnity AutoTranslator integration for Unity
- Visual editor for reviewing `.rpy`, `.txt`, and `.json` files
- Variable, tag, and formatting protection during translation
- Font injection for accented characters
- Native GTK4 interface, with no Electron, WebView, or embedded browser
- Portuguese interface and persistent local settings

## Run on Linux

Install GTK4, Rust, and the .NET SDK 8 to use the Unity extractor too:

```bash
# Fedora
sudo dnf install gtk4-devel dotnet-sdk-8.0

# Debian/Ubuntu
sudo apt install libgtk-4-dev dotnet-sdk-8.0

cargo run
```

For an optimized build:

```bash
cargo build --release
```

## Linux and Windows releases

The **Create release** workflow produces both portable packages:

- `TBX-Translator-Linux-x64.tar.gz`, containing the `TBX-Translator` executable;
- `TBX-Translator-Windows-x64.zip`, containing `TBX-Translator.exe`, its icon,
  and GTK4 DLLs.

Both include assets, the self-contained Unity extractor, and BepInEx/XUnity ZIP
files. Linux requires the GTK4 runtime to be installed. To publish, push a tag
or manually run **Create release** from GitHub **Actions**. The resulting GitHub
release contains both downloads.

## Documentation

- [Developer manual (PT-BR)](docs/MANUAL_DESENVOLVEDOR_PTBR.md)
- [Architecture](docs/arquitetura/ARQUITETURA.md)
- [API module](docs/api/API_MODULE.md)
- [Changelog](docs/changelog/CHANGELOG.md)
- [Development diary](docs/diary/README.md)

## License

The repository is public for reference and collaboration, but the software is
**source-available and non-commercial**. Free modifications and redistribution
are allowed if this license and the credits are retained. Selling, charging for
access, bundling it in a paid product or service, or using the project or a
modified version commercially requires prior written permission.
See [LICENSE](LICENSE).

## Support the project

If TBX Translator helps you, consider supporting its development on Ko-fi:

[![Support on Ko-fi](https://img.shields.io/badge/Ko--fi-Support%20the%20project-FF5E5B?logo=ko-fi&logoColor=white)](https://ko-fi.com/samwns)
