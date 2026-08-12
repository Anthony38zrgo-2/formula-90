"""Validate & Build gateway for the F90 SVG Track Authoring System.

Explicit human-approved build flow:

1.  Reject dirty/invalid/stale or unapproved source, an invalid registry, or a
    hash mismatch against a previously stored build of the same id.
2.  Recheck the validated hashes by recompiling immediately before Blender.
3.  Compile the normalized JSON through Blender headless into a staging dir,
    then validate every artifact (manifest hashes + environment/vegetation
    GLB split) before installing it at ``builds/<build_sha>/``.
4.  With ``--activate``, atomically activate the validated pair into a
    ``--runtime-root`` (temp + ``os.replace``, two-phase with rollback). A
    failed candidate never replaces the prior active build.

``build_sha`` is deterministic: SHA-256 over the source, registry, compiler
and normalized-JSON hashes. A failed run writes nothing and never touches the
runtime.

Determinism note: the canonical SVG, normalized JSON, GLB runtime parts and
``build_sha`` are byte-deterministic for identical input. The generated
``.blend`` is a Blender inspection/interchange artifact and carries Blender's
own session metadata, so it is not guaranteed byte-identical across sessions;
it is never a runtime authority (Godot consumes only the validated GLBs).

Run:

    python build_svg_track.py --source track.svg --registry configs/asset_registry.json \\
        --output-root builds --approved [--activate --runtime-root runtime]
"""

from __future__ import annotations

import argparse
from pathlib import Path
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time

from atomic_json import write_json_atomic
from asset_registry import load_registry, validate_registry
from build_normalized_common import (
    BUILD_MANIFEST_FILE,
    ENVIRONMENT_GLB,
    VEGETATION_GLB,
    build_sha256,
    compiler_sha256,
    validate_runtime_split,
)
from compile_svg_track import (
    CANONICAL_ARTIFACT,
    MANIFEST_ARTIFACT,
    NORMALIZED_ARTIFACT,
    CompileError,
    compile_track,
)
from validate_godot_load import GodotNotFoundError, run_godot_load_validation

_REPO_ROOT = Path(__file__).resolve().parents[2]
_DEFAULT_PROJECT = _REPO_ROOT / "game"

BUILDS_DIR = "builds"
STATE_FILE = "_state.json"
COMPILED_ARTIFACTS = (CANONICAL_ARTIFACT, NORMALIZED_ARTIFACT, MANIFEST_ARTIFACT)
ACTIVATION_FILE = "activation.json"

DEFAULT_BLENDER_EXE = Path(r"C:\Program Files\Blender Foundation\Blender 5.2\blender.exe")
_BLENDER_COMPILER = Path(__file__).with_name("build_normalized_track_blender.py")

# (build artifact, runtime file template) pair installed atomically for Godot.
_ACTIVATION_MEMBERS = (
    (ENVIRONMENT_GLB, "{track_id}.glb"),
    (VEGETATION_GLB, "{track_id}_vegetation.glb"),
)


class GatewayError(ValueError):
    """Actionable rejection with zero output written."""


def sha256_file(path: str | Path) -> str:
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load_state(output_root: Path) -> dict:
    path = Path(output_root) / BUILDS_DIR / STATE_FILE
    if not path.is_file():
        return {}
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {}


def save_state(output_root: Path, state: dict) -> None:
    write_json_atomic(Path(output_root) / BUILDS_DIR / STATE_FILE, state)


def plan_build(
    source_bytes: bytes,
    *,
    source_name: str,
    registry,
    registry_path: str | Path,
    output_dir: str | Path,
) -> dict:
    """Compile the source and derive the deterministic build id.

    Returns the compile manifest, the four input hashes and ``build_sha``.
    Raises :class:`GatewayError` when the source cannot be compiled.
    """
    try:
        manifest = compile_track(
            source_bytes,
            source_name=source_name,
            registry=registry,
            registry_path=registry_path,
            output_dir=output_dir,
        )
    except CompileError as exc:
        raise GatewayError("source does not compile:\n- " + "\n- ".join(exc.diagnostics)) from exc
    source_sha = manifest["artifacts"]["source"]["sha256"]
    registry_sha = manifest["artifacts"]["registry"]["sha256"]
    normalized_sha = manifest["artifacts"]["normalized"]["sha256"]
    compiler_sha = compiler_sha256()
    return {
        "manifest": manifest,
        "build_sha": build_sha256(source_sha, registry_sha, compiler_sha, normalized_sha),
        "source_sha": source_sha,
        "registry_sha": registry_sha,
        "normalized_sha": normalized_sha,
        "compiler_sha": compiler_sha,
    }


def check_existing_build(build_dir: Path, staged: Path) -> None:
    """Reject when a stored build at the same id diverges from the fresh compile."""
    build_dir = Path(build_dir)
    if not build_dir.exists():
        return
    for name in COMPILED_ARTIFACTS:
        expected = staged / name
        existing = build_dir / name
        if not existing.is_file():
            raise GatewayError(f"existing build {build_dir.name} is missing {name}")
        if expected.read_bytes() != existing.read_bytes():
            raise GatewayError(
                f"hash mismatch: existing build {build_dir.name} {name} differs from "
                "fresh compile (stale, dirty or tampered source/artifacts)"
            )


def validate_build_dir(build_dir: Path, build_sha: str, track_id: str) -> None:
    """Recompute manifest hashes and verify the environment/vegetation GLB split."""
    build_dir = Path(build_dir)
    manifest_path = build_dir / BUILD_MANIFEST_FILE
    if not manifest_path.is_file():
        raise GatewayError("build manifest missing")
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise GatewayError(f"build manifest unreadable: {exc}") from exc
    if manifest.get("build_sha256") != build_sha:
        raise GatewayError(
            f"build manifest build_sha mismatch: {manifest.get('build_sha256')!r} != {build_sha!r}"
        )
    if manifest.get("track_id") != track_id:
        raise GatewayError(
            f"build manifest track mismatch: {manifest.get('track_id')!r} != {track_id!r}"
        )
    for name, entry in manifest.get("artifacts", {}).items():
        path = build_dir / name
        if not path.is_file():
            raise GatewayError(f"artifact missing: {name}")
        if sha256_file(path) != entry.get("sha256"):
            raise GatewayError(f"artifact hash mismatch: {name}")
    if sha256_file(build_dir / NORMALIZED_ARTIFACT) != manifest.get("inputs", {}).get("normalized", {}).get("sha256"):
        raise GatewayError("stored normalized does not match manifest provenance")
    stats = manifest.get("stats", {})
    collision_names = set(stats.get("collision_proxy_names", []))
    asset_names = set(stats.get("asset_root_names", []))
    if not collision_names:
        raise GatewayError("build manifest records no collision proxies")
    try:
        validate_runtime_split(
            build_dir / ENVIRONMENT_GLB,
            build_dir / VEGETATION_GLB,
            collision_names,
            asset_names,
        )
    except Exception as exc:  # noqa: BLE001 - surface contract violation as rejection
        raise GatewayError(f"GLB split validation failed: {exc}") from exc


def activate_pair(build_dir: Path, runtime_root: Path, track_id: str, build_sha: str) -> None:
    """Atomically activate the validated environment/vegetation pair.

    Copies both members into a staging dir first, then moves the previous pair
    aside and replaces each target via ``os.replace``. On any failure the new
    targets are removed and the previous pair is restored, so the runtime never
    holds a partial pair.
    """
    build_dir = Path(build_dir)
    runtime_root = Path(runtime_root)
    runtime_root.mkdir(parents=True, exist_ok=True)
    members = [
        (build_dir / source, runtime_root / target.format(track_id=track_id))
        for source, target in _ACTIVATION_MEMBERS
    ]
    for source, _ in members:
        if not source.is_file():
            raise GatewayError(f"activation member missing: {source}")

    staging = runtime_root / f".activation-{track_id}-{build_sha[:12]}"
    shutil.rmtree(staging, ignore_errors=True)
    staging.mkdir(parents=True)
    for source, _ in members:
        shutil.copy2(source, staging / source.name)

    backups: dict[str, Path] = {}
    try:
        for source, target in members:
            if target.exists():
                backup = runtime_root / f".{target.name}.prev"
                if backup.exists():
                    backup.unlink()
                os.replace(target, backup)
                backups[target.name] = backup
        for source, target in members:
            os.replace(staging / source.name, target)
        activation = {
            "schema_version": 1,
            "track_id": track_id,
            "build_sha256": build_sha,
            "activated_at_unix_ns": time.time_ns(),
            "members": [
                {"runtime_file": target.name, "sha256": sha256_file(source)}
                for source, target in members
            ],
        }
        write_json_atomic(runtime_root / ACTIVATION_FILE, activation)
    except Exception:
        for _, target in members:
            target.unlink(missing_ok=True)
        for name, backup in backups.items():
            os.replace(backup, runtime_root / name)
        raise
    finally:
        shutil.rmtree(staging, ignore_errors=True)
        for backup in backups.values():
            if backup.exists():
                backup.unlink()


def _reject(message: str) -> int:
    print("REJECT " + message, file=sys.stderr)
    return 2


def _tail(output: str, lines: int = 20) -> str:
    return "\n".join(output.splitlines()[-lines:])


def _rmtree(path: Path) -> None:
    if path.exists():
        shutil.rmtree(path)


def gateway_main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Validate and build an approved F90 SVG track through Blender headless."
    )
    parser.add_argument("--source", required=True, help="SVG track source to compile.")
    parser.add_argument("--registry", required=True, help="Asset Registry JSON path.")
    parser.add_argument("--output-root", required=True, help="Base directory for builds/.")
    parser.add_argument("--approved", action="store_true", help="Explicit human approval flag.")
    parser.add_argument("--activate", action="store_true", help="Atomically activate the pair.")
    parser.add_argument("--runtime-root", default=None, help="Directory for the active runtime pair.")
    parser.add_argument("--blender-exe", default=str(DEFAULT_BLENDER_EXE), help="Blender executable.")
    parser.add_argument("--blender-timeout", type=int, default=600, help="Blender run timeout (s).")
    parser.add_argument(
        "--godot",
        nargs="?",
        const="",
        default=None,
        metavar="EXE",
        help="Run the Godot-load gate on the validated build GLBs before any "
             "activation. Pass an explicit Godot executable, or omit the value "
             "to auto-detect the console build under .tools/godot/. The "
             "operation fails (and the prior active pair is kept) when the "
             "candidate does not load in Godot.",
    )
    args = parser.parse_args(argv)

    source = Path(args.source)
    registry_path = Path(args.registry)
    output_root = Path(args.output_root)
    if not source.is_file():
        return _reject(f"source not found: {source}")
    if not registry_path.is_file():
        return _reject(f"registry not found: {registry_path}")
    if not args.approved:
        return _reject("build requires the explicit --approved human approval flag")

    try:
        registry = load_registry(registry_path, repo_root=_REPO_ROOT)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        return _reject(f"registry load failed: {exc}")
    registry_errors = validate_registry(registry)
    if registry_errors:
        return _reject("registry invalid:\n- " + "\n- ".join(registry_errors))

    output_root.mkdir(parents=True, exist_ok=True)
    builds_root = output_root / BUILDS_DIR
    staging_root = builds_root / ".staging"
    staging_root.mkdir(parents=True, exist_ok=True)
    staged = Path(tempfile.mkdtemp(prefix="candidate_", dir=str(staging_root)))
    try:
        try:
            plan = plan_build(
                source.read_bytes(),
                source_name=source.name,
                registry=registry,
                registry_path=registry_path,
                output_dir=staged,
            )
        except GatewayError as exc:
            return _reject(str(exc))
        build_sha = plan["build_sha"]
        build_dir = builds_root / build_sha
        track_id = plan["manifest"]["track_id"]

        try:
            check_existing_build(build_dir, staged)
        except GatewayError as exc:
            return _reject(str(exc))

        reused = False
        if build_dir.exists():
            try:
                validate_build_dir(build_dir, build_sha, track_id)
                reused = True
                print(f"[build] reusing validated build {build_sha}")
            except GatewayError:
                pass

        if not reused:
            recheck_dir = Path(tempfile.mkdtemp(prefix="recheck_", dir=str(staging_root)))
            try:
                try:
                    recheck = plan_build(
                        source.read_bytes(),
                        source_name=source.name,
                        registry=registry,
                        registry_path=registry_path,
                        output_dir=recheck_dir,
                    )
                except GatewayError as exc:
                    return _reject(f"recheck compile failed: {exc}")
                if (recheck_dir / NORMALIZED_ARTIFACT).read_bytes() != (staged / NORMALIZED_ARTIFACT).read_bytes():
                    return _reject(
                        "source changed between preflight and build "
                        "(recompiled normalized JSON is not byte-identical)"
                    )
                if recheck["build_sha"] != build_sha:
                    return _reject("build id changed between preflight and build")
            finally:
                _rmtree(recheck_dir)

            blender = Path(args.blender_exe)
            if not blender.is_file():
                return _reject(f"blender not found: {blender}")
            try:
                proc = subprocess.run(
                    [
                        str(blender),
                        "--background",
                        "--factory-startup",
                        "--python",
                        str(_BLENDER_COMPILER),
                        "--",
                        "--normalized", str(staged / NORMALIZED_ARTIFACT),
                        "--registry", str(registry_path),
                        "--output-dir", str(staged),
                    ],
                    capture_output=True,
                    text=True,
                    timeout=args.blender_timeout,
                )
            except subprocess.TimeoutExpired:
                return _reject(f"blender timed out after {args.blender_timeout}s")
            if proc.returncode != 0:
                return _reject(
                    f"blender failed rc={proc.returncode}\n"
                    f"--- stdout ---\n{_tail(proc.stdout)}\n--- stderr ---\n{_tail(proc.stderr)}"
                )
            try:
                validate_build_dir(staged, build_sha, track_id)
            except GatewayError as exc:
                return _reject(f"build validation failed: {exc}")
            if build_dir.exists():
                shutil.rmtree(build_dir)
            os.rename(staged, build_dir)
            staged = None

        state = load_state(output_root)
        state[track_id] = {
            **state.get(track_id, {}),
            "last_build_sha256": build_sha,
            "last_source_sha256": plan["source_sha"],
            "last_normalized_sha256": plan["normalized_sha"],
        }
        save_state(output_root, state)

        if args.godot is not None:
            godot_exe = args.godot or None
            try:
                godot_rc = run_godot_load_validation(
                    build_dir / ENVIRONMENT_GLB,
                    build_dir / VEGETATION_GLB,
                    godot_exe=godot_exe,
                    project=_DEFAULT_PROJECT,
                    track_id=track_id,
                )
            except GodotNotFoundError as exc:
                return _reject(str(exc))
            if godot_rc != 0:
                return _reject(
                    f"godot-load validation failed (rc={godot_rc}); candidate "
                    f"{build_sha} is not activated and the active build is untouched"
                )
            print(f"[godot] candidate {build_sha} passes Godot-load validation")

        if args.activate:
            if not args.runtime_root:
                return _reject("--activate requires --runtime-root")
            runtime_root = Path(args.runtime_root)
            try:
                activate_pair(build_dir, runtime_root, track_id, build_sha)
            except Exception as exc:  # noqa: BLE001 - report and keep prior active pair
                return _reject(f"activation failed: {exc}")
            state[track_id]["last_activated_build_sha256"] = build_sha
            save_state(output_root, state)
            print(f"[activate] activated {build_sha} into {runtime_root}")

        print(f"track: {track_id}")
        print(f"build_sha: {build_sha}")
        print(f"build_dir: {build_dir}")
        return 0
    finally:
        if staged is not None:
            _rmtree(staged)


if __name__ == "__main__":
    raise SystemExit(gateway_main())
