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

INNO_SETUP="/c/Program Files (x86)/Inno Setup 6/ISCC.exe"
if [[ ! -x "$INNO_SETUP" ]]; then
    echo "==> Instalando Inno Setup"
    powershell.exe -NoProfile -Command "choco install innosetup -y --no-progress"
fi

if [[ ! -x "$INNO_SETUP" ]]; then
    echo "Erro: Inno Setup não foi encontrado após a instalação." >&2
    exit 1
fi

echo "==> Compilando TBX Translator para Windows"
cargo build --release

STAGE_DIR="$PROJECT_DIR/release/TBX-Translator-Windows-x64"
mkdir -p "$STAGE_DIR"
cp target/release/tbx-translator.exe "$STAGE_DIR/TBX-Translator.exe"

echo "==> Publicando extrator Unity"
dotnet.exe publish unity_static_extractor/unity_static_extractor.csproj \
    --configuration Release --runtime win-x64 --self-contained true \
    --output "$STAGE_DIR/unity_static_extractor"
cp -a assets BepInEx XUnity_AutoTranslator_bepInEx "$STAGE_DIR/"
mkdir -p "$STAGE_DIR/third_party/UnityPy"
cp -a third_party/UnityPy/UnityPy "$STAGE_DIR/third_party/UnityPy/"
cp unity_static_extractor/unitypy_extract.py "$STAGE_DIR/unity_static_extractor/"
cp /mingw64/bin/*.dll "$STAGE_DIR/"
cp README.md LICENSE "$STAGE_DIR/"

echo "==> Criando instalador"
"$INNO_SETUP" packaging/windows/tbx-translator.iss

echo
echo "Release pronta em: $PROJECT_DIR/release/TBX-Translator-Setup.exe"
