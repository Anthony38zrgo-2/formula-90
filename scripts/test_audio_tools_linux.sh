#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)";cd "$ROOT"
[[ -x .venv/bin/python ]]||{ echo 'Ejecute setup_audio_tools_linux.sh.' >&2;exit 1;}
.venv/bin/python -m pytest tools/audio/tests;.venv/bin/python -m ruff check tools/audio
