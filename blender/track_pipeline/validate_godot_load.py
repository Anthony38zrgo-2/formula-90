"""Headless Godot-load validation for generated F90 track GLB pairs.

This wrapper drives :mod:`game.tools.validate_generated_track` (a GDScript
``SceneTree`` validator) through a real Godot 4.7 console binary and relays the
validator's exit code:

* ``0`` — PASS: both GLBs load in Godot and every runtime split invariant held;
* nonzero — FAIL: the environment/vegetation GLBs do not load, a collision
  proxy is missing or leaked, or the scene trees are malformed.

It is deliberately a thin CLI: the actual invariant checks live in the GDScript
so they run inside the real Godot runtime. This module owns argument handling,
Godot executable resolution and exit-code propagation only.

Usage:

    python validate_godot_load.py --env-glb track.glb --veg-glb track_vegetation.glb
        [--godot Godot_v4.7.1-stable_win64_console.exe] [--project <game>]
        [--track-id <id>]

Real smoke test (against a temporary build produced by the gateway):

    blender/track_pipeline/.venv/Scripts/python.exe build_svg_track.py \
        --source ... --registry ... --output-root <temp>/builds --approved \
        --activate --runtime-root <temp>/runtime --godot

which runs this validator on the staged ``track_runtime_*.glb`` before the
atomic activation replaces the previous active pair. A corrupt or missing GLB
must fail with a nonzero exit and leave the prior active build untouched:

    blender/track_pipeline/.venv/Scripts/python.exe validate_godot_load.py \
        --env-glb missing.glb --veg-glb track_vegetation.glb
    echo $LASTEXITCODE   # nonzero

If ``--godot`` is omitted the console build under ``.tools/godot/`` is located
automatically.
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_PROJECT = _REPO_ROOT / "game"
VALIDATION_SCRIPT = "res://tools/validate_generated_track.gd"

EXIT_OK = 0
EXIT_GODOT_NOT_FOUND = 3


class GodotNotFoundError(FileNotFoundError):
    """Raised when no usable Godot executable can be resolved."""


def _search_godot_dir(godot_dir: Path) -> Path | None:
    """Return the preferred Godot binary inside ``godot_dir``.

    Prefers a ``*console*.exe`` build (headless output on Windows), then any
    ``Godot*.exe``. Returns ``None`` when nothing is found.
    """
    godot_dir = Path(godot_dir)
    if not godot_dir.is_dir():
        return None
    console_candidates = sorted(p for p in godot_dir.glob("Godot*console.exe") if p.is_file())
    if console_candidates:
        return console_candidates[0]
    candidates = sorted(p for p in godot_dir.glob("Godot*.exe") if p.is_file())
    return candidates[0] if candidates else None


def find_godot_exe() -> Path | None:
    """Locate the bundled Godot console build under ``.tools/godot/``."""
    return _search_godot_dir(_REPO_ROOT / ".tools" / "godot")


def resolve_godot_exe(explicit: str | Path | None) -> Path:
    """Resolve the Godot executable.

    An explicit path is authoritative: when it is not a file this raises so a
    caller that requested a specific binary never silently runs a different
    one. Auto-detection (``$GODOT_BIN`` then the bundled console build under
    ``.tools/godot/``) only applies when no explicit path is given.
    """
    if explicit:
        path = Path(explicit)
        if not path.is_file():
            raise GodotNotFoundError(f"Godot executable not found: {path}")
        return path.resolve()
    env_var = shutil.which("GODOT_BIN")
    candidates = [Path(env_var)] if env_var else []
    discovered = find_godot_exe()
    if discovered:
        candidates.append(discovered)
    for candidate in candidates:
        path = Path(candidate)
        if path.is_file():
            return path.resolve()
    raise GodotNotFoundError(
        "Godot executable not found. Pass --godot <exe> or set GODOT_BIN."
    )


def build_validation_command(
    env_glb: str | Path,
    veg_glb: str | Path,
    *,
    godot: str | Path,
    project: str | Path,
    track_id: str | None = None,
) -> list[str]:
    """Return the exact Godot headless command line for the validator."""
    command = [
        str(godot),
        "--headless",
        "--path",
        str(project),
        "--script",
        VALIDATION_SCRIPT,
        "--",
        "--env-glb",
        str(Path(env_glb).resolve()),
        "--veg-glb",
        str(Path(veg_glb).resolve()),
    ]
    if track_id:
        command += ["--track-id", track_id]
    return command


def run_godot_load_validation(
    env_glb: str | Path,
    veg_glb: str | Path,
    *,
    godot_exe: str | Path | None = None,
    project: str | Path | None = None,
    track_id: str | None = None,
) -> int:
    """Run the headless Godot validator and propagate its exit code.

    The validator report is printed to stdout/stderr as it is produced.
    Returns ``0`` on PASS and the Godot exit code (normally ``1``) on FAIL.
    """
    godot = resolve_godot_exe(godot_exe)
    project_path = Path(project) if project else DEFAULT_PROJECT
    command = build_validation_command(
        env_glb, veg_glb, godot=godot, project=project_path, track_id=track_id
    )
    proc = subprocess.run(command, capture_output=True, text=True)
    if proc.stdout:
        sys.stdout.write(proc.stdout)
    if proc.stderr:
        sys.stderr.write(proc.stderr)
    return proc.returncode


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Validate generated F90 track GLB pairs by loading them in "
                    "a real headless Godot 4.7 runtime.",
    )
    parser.add_argument("--env-glb", required=True, help="Environment/collision GLB path.")
    parser.add_argument("--veg-glb", required=True, help="Vegetation (collision-free) GLB path.")
    parser.add_argument(
        "--godot",
        default=None,
        help="Godot executable. Omit to auto-detect the console build under "
             ".tools/godot/ (or use $GODOT_BIN).",
    )
    parser.add_argument("--project", default=str(DEFAULT_PROJECT), help="Godot project directory.")
    parser.add_argument("--track-id", default=None, help="Optional track id for the report.")
    args = parser.parse_args(argv)
    try:
        return run_godot_load_validation(
            args.env_glb,
            args.veg_glb,
            godot_exe=args.godot,
            project=args.project,
            track_id=args.track_id,
        )
    except GodotNotFoundError as exc:
        print(str(exc), file=sys.stderr)
        return EXIT_GODOT_NOT_FOUND


if __name__ == "__main__":
    raise SystemExit(main())
