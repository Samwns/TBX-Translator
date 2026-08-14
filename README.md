<div align="center">
  <img src="assets/com.tbx.translator.svg" width="96" height="96" alt="TBX Translator">

  # TBX Translator

  **Extraia, traduza, revise e instale traduções em jogos Ren'Py, Unity e Godot.**

  [![Versão](https://img.shields.io/github/v/release/Samwns/TBX-Translator?display_name=tag&sort=semver&label=vers%C3%A3o)](https://github.com/Samwns/TBX-Translator/releases/latest)
  [![Downloads](https://img.shields.io/github/downloads/Samwns/TBX-Translator/total?label=downloads)](https://github.com/Samwns/TBX-Translator/releases/latest)
  [![Licença](https://img.shields.io/badge/licen%C3%A7a-CC%20BY--NC--SA%204.0-lightgrey)](LICENSE)

  [![Discord](https://img.shields.io/badge/Discord-Entrar%20no%20servidor-5865F2?logo=discord&logoColor=white)](https://discord.gg/xsxhvWgWBz)
  [![Ko-fi](https://img.shields.io/badge/Ko--fi-Apoiar%20o%20projeto-FF5E5B?logo=ko-fi&logoColor=white)](https://ko-fi.com/samwns)

  **Português (Brasil)** · [English](README.en.md)
</div>

## Recursos

- Ren'Py: extração de diálogos, integração do idioma ao menu e injeção de fontes.
- Unity Mono/IL2CPP: AssetsTools.NET, UnityPy, BepInEx e XUnity AutoTranslator.
- Godot: catálogos nativos, PO, PCK, recursos binários e arquivos de história.
- Editor visual para revisar `.rpy`, `.txt` e `.json` antes da instalação.
- Proteção de variáveis, tags, BBCode, espaços e formatação.
- Traduções independentes por engine, cache, concorrência controlada e cancelamento.
- Atualização pelo aplicativo e interface disponível em 104 idiomas.

## Baixar

A [release mais recente](https://github.com/Samwns/TBX-Translator/releases/latest) oferece:

- Windows portátil (`.zip`) e instalador (`.exe`)
- Debian/Ubuntu (`.deb`)
- Fedora (`.rpm`)
- Arch Linux (`.pkg.tar.zst`)
- Linux portátil (`.AppImage`)

A versão exibida no aplicativo, nos pacotes e nas releases vem do `Cargo.toml`;
o número do build é acrescentado automaticamente pelo GitHub Actions.

## Desenvolvimento

Requer Rust estável. O .NET SDK 8 é necessário para desenvolver o extrator Unity.

```bash
# Fedora
sudo dnf install dotnet-sdk-8.0

# Debian/Ubuntu
sudo apt install dotnet-sdk-8.0 libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev

cargo run
```

Para criar os pacotes suportados na plataforma atual:

```bash
cargo package-all
```

## Documentação

- [Manual do desenvolvedor](docs/MANUAL_DESENVOLVEDOR_PTBR.md)
- [Arquitetura](docs/arquitetura/ARQUITETURA.md)
- [API](docs/api/API_MODULE.md)
- [Changelog](docs/changelog/CHANGELOG.md)

## Comunidade e apoio

- [Servidor oficial do Discord](https://discord.gg/xsxhvWgWBz)
- [Apoie o desenvolvimento no Ko-fi](https://ko-fi.com/samwns)

O GitHub também exibe o botão **Sponsor** por meio de `.github/FUNDING.yml`.

## Licença

Distribuído sob a [CC BY-NC-SA 4.0](LICENSE): uso e redistribuição não comerciais,
com atribuição e compartilhamento pela mesma licença.
