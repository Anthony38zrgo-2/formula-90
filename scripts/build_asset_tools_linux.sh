#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
export PYTHONPATH="$root/.tools/python-packages"
python3 -m SCons -f tools/sprites/SConstruct -j"$(nproc)" \
  build/tools/prepare_sprite build/tools/inspect_sprite build/tools/build_sprite_sheet \
  build/tools/sprite_tools_tests build/tools/analyze_audio build/tools/wav_analyzer_tests build/tools/validate_assets
