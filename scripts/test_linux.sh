#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; "$ROOT/scripts/build_linux.sh"
CXX="$(command -v g++ || command -v clang++)"; mkdir -p "$ROOT/build/tests"
"$CXX" -std=c++20 -I"$ROOT/native/include" "$ROOT/native/tests/unit_tests.cpp" -o "$ROOT/build/tests/unit_tests"; "$ROOT/build/tests/unit_tests"
GODOT="${1:-${GODOT_BIN:-}}"; [[ -n "$GODOT" ]] || GODOT="$(command -v godot4 || command -v godot || true)"; [[ -x "$GODOT" ]] || { echo 'Godot no encontrado.' >&2; exit 1; }
"$GODOT" --headless --editor --path "$ROOT/game" --quit
for scene in scenes/ui/main_menu.tscn scenes/tracks/test_field/test_field.tscn scenes/vehicles/player_car.tscn scenes/ui/debug_hud.tscn; do "$GODOT" --headless --path "$ROOT/game" "$scene" --quit-after 2; done

