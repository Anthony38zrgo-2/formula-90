#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GODOT="${1:-${GODOT_BIN:-}}"
TEST_PATH="${2:-res://tests/gdunit}"

if [[ -z "$GODOT" ]]; then
    GODOT="$(command -v godot4 || command -v godot || true)"
fi
[[ -x "$GODOT" ]] || { echo 'Godot no encontrado.' >&2; exit 1; }

"$GODOT" \
    --headless \
    --path "$ROOT/game" \
    --script res://addons/gdUnit4/bin/GdUnitCmdTool.gd \
    --ignoreHeadlessMode \
    -a "$TEST_PATH" \
    -c \
    -rd res://reports/gdunit
