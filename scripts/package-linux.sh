#!/usr/bin/env bash
# Build all Linux release formats locally.
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_DIR"

need() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "Erro: é necessário o comando '$1'." >&2
        exit 1
    }
}

need cargo
need dotnet
need curl

if ! command -v fpm >/dev/null 2>&1; then
    need sudo
    echo "==> Instalando fpm (uma única vez)"
    sudo gem install fpm --no-document
fi

VERSION="$(sed -nE 's/^version = "([^"]+)"/\1/p' Cargo.toml | head -n 1)"
VERSION="${VERSION%-alpha}"
RELEASE_DIR="$PROJECT_DIR/release"
TEMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/tbx-package.XXXXXX")"
trap 'rm -rf "$TEMP_DIR"' EXIT

APP_DIR="$TEMP_DIR/TBX-Translator-Linux-x64"
PKG_ROOT="$TEMP_DIR/package-root"
APPIMAGE_DIR="$TEMP_DIR/AppDir"

echo "==> Compilando TBX Translator"
cargo build --release

echo "==> Publicando extrator Unity"
mkdir -p "$APP_DIR"
install -m 755 target/release/tbx-translator "$APP_DIR/TBX-Translator"
dotnet publish unity_static_extractor/unity_static_extractor.csproj \
    --configuration Release --runtime linux-x64 --self-contained true \
    --output "$APP_DIR/unity_static_extractor"
cp -a assets BepInEx XUnity_AutoTranslator_bepInEx "$APP_DIR/"
cp README.md LICENSE "$APP_DIR/"

echo "==> Montando pacotes nativos"
mkdir -p "$PKG_ROOT/opt/tbx-translator"
cp -a "$APP_DIR/." "$PKG_ROOT/opt/tbx-translator/"
install -Dm755 packaging/linux/tbx-translator "$PKG_ROOT/usr/bin/tbx-translator"
install -Dm644 packaging/linux/tbx-translator.desktop \
    "$PKG_ROOT/usr/share/applications/tbx-translator.desktop"
install -Dm644 assets/com.tbx.translator.svg \
    "$PKG_ROOT/usr/share/icons/hicolor/scalable/apps/com.tbx.translator.svg"
mkdir -p "$RELEASE_DIR"

fpm -s dir -t deb -n tbx-translator -v "$VERSION" -C "$PKG_ROOT" \
    -p "$RELEASE_DIR/TBX-Translator-Debian-Ubuntu-amd64.deb" \
    --license "CC-BY-NC-SA-4.0" --depends libgtk-4-1 opt usr
fpm -s dir -t rpm -n tbx-translator -v "$VERSION" -C "$PKG_ROOT" \
    -p "$RELEASE_DIR/TBX-Translator-Fedora-x86_64.rpm" \
    --license "CC-BY-NC-SA-4.0" --depends gtk4 opt usr
fpm -s dir -t pacman -n tbx-translator -v "$VERSION" -C "$PKG_ROOT" \
    -p "$RELEASE_DIR/TBX-Translator-Arch-x86_64.pkg.tar.zst" \
    --license "CC-BY-NC-SA-4.0" --depends gtk4 opt usr

echo "==> Criando AppImage"
mkdir -p "$APPIMAGE_DIR/usr/bin"
cp -a "$APP_DIR/." "$APPIMAGE_DIR/usr/bin/"
install -m755 packaging/linux/AppRun "$APPIMAGE_DIR/AppRun"
cp packaging/linux/tbx-translator.desktop "$APPIMAGE_DIR/tbx-translator.desktop"
cp assets/com.tbx.translator.svg "$APPIMAGE_DIR/com.tbx.translator.svg"
curl --fail --location --silent --show-error \
    https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage \
    --output "$TEMP_DIR/appimagetool"
chmod +x "$TEMP_DIR/appimagetool"
ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$TEMP_DIR/appimagetool" \
    "$APPIMAGE_DIR" "$RELEASE_DIR/TBX-Translator-x86_64.AppImage"

echo
echo "Release pronta em: $RELEASE_DIR"
find "$RELEASE_DIR" -maxdepth 1 -type f -printf ' - %f\n'
