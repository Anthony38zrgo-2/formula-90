#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$ROOT"
[[ -f third_party/godot-cpp/SConstruct ]] || { echo 'godot-cpp ausente; ejecute bootstrap_linux.sh.' >&2; exit 1; }
[[ -x .tools/venv/bin/scons ]] || { echo 'SCons ausente; ejecute bootstrap_linux.sh.' >&2; exit 1; }
command -v g++ >/dev/null || command -v clang++ >/dev/null || { echo 'Compilador C++ ausente.' >&2; exit 1; }
CONFIG="${1:-debug}"; [[ "$CONFIG" == release ]] && TARGET=template_release || TARGET=template_debug
.tools/venv/bin/scons platform=linux target="$TARGET" arch=x86_64 api_version=4.7 build_profile=build_profile.json -j"$(nproc)"

