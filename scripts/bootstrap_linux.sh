#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$ROOT"
command -v git >/dev/null || { echo 'Git no encontrado.' >&2; exit 1; }
command -v python3 >/dev/null || { echo 'Python 3.8+ no encontrado.' >&2; exit 1; }
git submodule update --init --recursive
python3 -m venv .tools/venv; .tools/venv/bin/pip install 'scons==4.10.0'
echo "Repositorio: $ROOT"; echo 'Bootstrap Linux completado. Instale Godot 4.7.1 y defina GODOT_BIN.'

