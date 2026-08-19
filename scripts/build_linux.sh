#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$ROOT"
[[ -f third_party/godot-cpp/SConstruct ]] || { echo 'godot-cpp ausente; ejecute bootstrap_linux.sh.' >&2; exit 1; }
[[ -x .tools/venv/bin/scons ]] || { echo 'SCons ausente; ejecute bootstrap_linux.sh.' >&2; exit 1; }
command -v g++ >/dev/null || command -v clang++ >/dev/null || { echo 'Compilador C++ ausente.' >&2; exit 1; }
CONFIG="${1:-debug}"; [[ "$CONFIG" == release ]] && TARGET=template_release || TARGET=template_debug
.tools/venv/bin/scons platform=linux target="$TARGET" arch=x86_64 api_version=4.7 build_profile=build_profile.json -j"$(nproc)"

# Fachada-orquestador (formula90_core). Linux runtime rust es opcional en este repo
# (el runtime de Godot estÃ¡ gateado a Windows); se compila por simetrÃ­a si cargo existe.
if command -v cargo >/dev/null 2>&1; then
  echo "Compilando formula90_core (fachada-orquestador) ..."
  if [[ "$CONFIG" == release ]]; then CARGO_ARGS=(--release); else CARGO_ARGS=(); fi
  cargo build --manifest-path game/crates/formula90-core/Cargo.toml "${CARGO_ARGS[@]}"
  CORE_DIR="game/crates/target/release"; [[ "$CONFIG" == release ]] || CORE_DIR="game/crates/target/debug"
  CORE_DEST="formula90_core.linux.$TARGET.x86_64.so"
  cp -f "$CORE_DIR/libformula90_core.so" "game/addons/formula90s/bin/$CORE_DEST"
  echo "formula90_core copiada a game/addons/formula90s/bin/$CORE_DEST"
else
  echo "cargo no encontrado; omitiendo build de formula90_core (solo extension C++)." >&2
fi

