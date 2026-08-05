#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GODOT="${1:-${GODOT_BIN:-}}"; [[ -n "$GODOT" ]] || GODOT="$(command -v godot4 || command -v godot || true)"
[[ -x "$GODOT" ]] || { echo 'Godot no encontrado: argumento, GODOT_BIN o PATH.' >&2; exit 1; }
[[ -f "$ROOT/game/addons/formula90s/bin/libformula90s.linux.template_debug.x86_64.so" ]] || { echo 'GDExtension no compilada.' >&2; exit 1; }
echo "Godot: $GODOT"; exec "$GODOT" --path "$ROOT/game"

