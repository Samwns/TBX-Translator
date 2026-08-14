#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-$(sed -nE 's/^version = "([^"]+)"/\1/p' "$PROJECT_DIR/Cargo.toml" | head -n 1)}"
VERSION="${VERSION#v}"
VERSION="${VERSION%%-*}"
BUILD_NUMBER="${2:?Informe o número do build como segundo argumento.}"
PREVIOUS_TAG="${3:-}"
OUTPUT_DIR="${4:-$PROJECT_DIR/release/messages}"
TAG_NAME="v${VERSION}-build-${BUILD_NUMBER}"
RELEASE_NAME="TBX Translator v${VERSION} (build ${BUILD_NUMBER})"
SUMMARY_FILE="$PROJECT_DIR/docs/releases/UPDATE_SUMMARY.md"

if [[ -z "$PREVIOUS_TAG" ]] && git -C "$PROJECT_DIR" rev-parse --git-dir >/dev/null 2>&1; then
    PREVIOUS_TAG="$(git -C "$PROJECT_DIR" tag --list "v${VERSION}-build-*" --sort=-v:refname | head -n 1)"
fi

mkdir -p "$OUTPUT_DIR"

if [[ -n "$PREVIOUS_TAG" ]]; then
    CHANGELOG_URL="https://github.com/Samwns/TBX-Translator/compare/${PREVIOUS_TAG}...${TAG_NAME}"
else
    CHANGELOG_URL="https://github.com/Samwns/TBX-Translator/commits/${TAG_NAME}"
fi

write_downloads() {
    cat <<'EOF'
### Downloads

- **Windows (Portátil .zip):** `TBX-Translator-Windows-x64.zip`
- **Windows (Instalador):** `TBX-Translator-Setup.exe`
- **Debian / Ubuntu:** `TBX-Translator-Debian-Ubuntu-amd64.deb`
- **Fedora:** `TBX-Translator-Fedora-x86_64.rpm`
- **Arch Linux:** `TBX-Translator-Arch-x86_64.pkg.tar.zst`
- **Linux portátil:** `TBX-Translator-x86_64.AppImage`
EOF
}

{
    printf '## %s\n\n' "$RELEASE_NAME"
    printf '### Resumo das mudanças\n\n'
    cat "$SUMMARY_FILE"
    printf '\n\n'
    write_downloads
    cat <<'EOF'


### Comunidade e apoio

- [Servidor do Discord](https://discord.gg/xsxhvWgWBz)
- [Ko-fi — samwns](https://ko-fi.com/samwns)

### Licença

CC BY-NC-SA 4.0 — uso e redistribuição não comerciais, com atribuição.
EOF
    printf '\n**Full Changelog**: %s\n' "$CHANGELOG_URL"
} > "$OUTPUT_DIR/release-body.md"

{
    printf '# [**%s**](https://github.com/Samwns/TBX-Translator/releases/latest)\n\n' "$RELEASE_NAME"
    printf 'update/changes:\n'
    printf '### Resumo das mudanças\n\n'
    cat "$SUMMARY_FILE"
    printf '\n\n@Member @here\n'
} > "$OUTPUT_DIR/discord-update.md"

printf 'Mensagens geradas em %s para %s.\n' "$OUTPUT_DIR" "$TAG_NAME"
