#!/usr/bin/env bash
# Build the Windows installer locally. Run this from an MSYS2 MINGW64 shell.
set -euo pipefail

if [[ "${MSYSTEM:-}" != "MINGW64" ]]; then
    echo "Erro: execute no terminal 'MSYS2 MINGW64'." >&2
    exit 1
fi

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_DIR"
export PATH="/mingw64/bin:/c/Program Files/dotnet:$PATH"

need() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "Erro: é necessário o comando '$1'." >&2
        exit 1
    }
}

need cargo
need dotnet.exe

echo "==> Compilando TBX Translator para Windows"
cargo build --release

STAGE_DIR="$PROJECT_DIR/release/TBX-Translator-Windows-x64"
rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR"
cp target/release/tbx-translator.exe "$STAGE_DIR/TBX-Translator.exe"

echo "==> Publicando extrator Unity"
dotnet.exe publish unity_static_extractor/unity_static_extractor.csproj \
    --configuration Release --runtime win-x64 --self-contained true \
    --output "$STAGE_DIR/unity_static_extractor"

echo "==> Copiando assets e dependências"
cp -a assets BepInEx XUnity_AutoTranslator_bepInEx "$STAGE_DIR/"
mkdir -p "$STAGE_DIR/third_party/UnityPy"
cp -a third_party/UnityPy/UnityPy "$STAGE_DIR/third_party/UnityPy/"
cp unity_static_extractor/unitypy_extract.py "$STAGE_DIR/unity_static_extractor/"
cp /mingw64/bin/*.dll "$STAGE_DIR/"
cp README.md LICENSE THIRD_PARTY_NOTICES.md "$STAGE_DIR/"

echo "==> Criando pacote portátil (.zip)"
cd "$PROJECT_DIR/release"
if command -v zip >/dev/null 2>&1; then
    zip -r -q "TBX-Translator-Windows-x64.zip" "TBX-Translator-Windows-x64"
else
    powershell.exe -NoProfile -Command "Compress-Archive -Path 'TBX-Translator-Windows-x64' -DestinationPath 'TBX-Translator-Windows-x64.zip' -Force"
fi
cd "$PROJECT_DIR"

echo "✓ Pacote portátil criado: $PROJECT_DIR/release/TBX-Translator-Windows-x64.zip"

INNO_SETUP="/c/Program Files (x86)/Inno Setup 6/ISCC.exe"
if [[ -x "$INNO_SETUP" ]]; then
    echo "==> Criando instalador Inno Setup (opcional)"
    "$INNO_SETUP" packaging/windows/tbx-translator.iss
    echo "✓ Instalador pronto em: $PROJECT_DIR/release/TBX-Translator-Setup.exe"
else
    echo "ℹ Inno Setup não instalado. O pacote portátil (.zip) já está pronto para uso e distribuição!"
fi
