#!/usr/bin/env bash
# One command for all release formats available on the current platform.
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_DIR"

if [[ "${MSYSTEM:-}" == "MINGW64" ]]; then
    exec scripts/package-windows.sh
fi

case "$(uname -s)" in
    Linux)
        exec scripts/package-linux.sh
        ;;
    *)
        echo "Erro: use Linux para os pacotes Linux ou MSYS2 MINGW64 para o instalador Windows." >&2
        exit 1
        ;;
esac
