

"""Strict staging-only Blender materializer for VehicleBuildIR v1."""
from __future__ import annotations

from hashlib import sha256
import json
from pathlib import Path
import subprocess
import tempfile
from typing import Any

from .blend_scan import DEFAULT_BLENDER_EXE
from .build_ir import validate_build_ir
from .canonical import canonical_json_bytes

SENTINEL = "__FORMULA90_VEHICLE_STUDIO_MATERIALIZE_JSON__"
INFRASTRUCTURE_KINDS = {"assert_source_hash", "validate_vehicle", "stage_variant_export"}
SUPPORTED_CHANGED_ROLES = {"wheelbase", "front_track", "rear_track", "front_tire_radius", "rear_tire_radius", "front_tire_width", "rear_tire_width"}


class MaterializationError(ValueError):
    def __init__(self, code: str, message: str):
        self.code = code
        super().__init__(message)


def materialize_build_ir(build_ir: dict[str, Any], *, repo_root: Path, staging_root: Path,
                         blender_executable: Path = DEFAULT_BLENDER_EXE,
                         timeout: float = 300.0) -> dict[str, Any]:
    validate_build_ir(build_ir)
    source_spec = build_ir["operations"][0].get("inputs", {}).get("materialization_source")
    if not isinstance(source_spec, dict):
        raise MaterializationError("MATERIALIZER_SOURCE_MISSING", "BuildIR has no concrete materialization source")
    _preflight_operations(build_ir)
    repo = Path(repo_root).resolve()
    source = (repo / str(source_spec.get("path", ""))).resolve()
    _require_inside(source, repo, "MATERIALIZER_SOURCE_OUTSIDE_ROOT")
    if not source.is_file():
        raise MaterializationError("MATERIALIZER_SOURCE_MISSING", "materialization source does not exist")
    expected_hash = source_spec.get("sha256")
    before_hash = _sha256(source)
    if before_hash != expected_hash:
        raise MaterializationError("MATERIALIZER_SOURCE_HASH_MISMATCH", "materialization source hash does not match BuildIR")
    modular_specs = build_ir["operations"][0].get("inputs", {}).get("materialization_sources", [])
    sources = []
    for spec in modular_specs:
        path = (repo / str(spec.get("path", ""))).resolve()
        _require_inside(path, repo, "MATERIALIZER_SOURCE_OUTSIDE_ROOT")
        if not path.is_file() or _sha256(path) != spec.get("sha256"):
            raise MaterializationError("MATERIALIZER_SOURCE_HASH_MISMATCH", f"modular source mismatch: {spec.get('path')}")
        sources.append({"path": spec["path"], "absolute_path": str(path), "sha256": spec["sha256"]})
    material_specs = build_ir["operations"][0].get("inputs", {}).get("materialization_material_sources", [])
    material_sources = []
    for spec in material_specs:
        path = (repo / str(spec.get("path", ""))).resolve()
        _require_inside(path, repo, "MATERIALIZER_SOURCE_OUTSIDE_ROOT")
        if not path.is_file() or _sha256(path) != spec.get("sha256"):
            raise MaterializationError("MATERIALIZER_SOURCE_HASH_MISMATCH", f"material source mismatch: {spec.get('path')}")
        material_sources.append({
            "path": spec["path"], "absolute_path": str(path), "sha256": spec["sha256"],
            "material_slot": spec.get("material_slot"),
        })
    executable = Path(blender_executable).resolve()
    if not executable.is_file():
        raise MaterializationError("MATERIALIZER_BLENDER_MISSING", f"Blender executable not found: {executable}")
    build_hash = sha256(canonical_json_bytes(build_ir)).hexdigest().upper()
    root = Path(staging_root).resolve()
    output_dir = root / build_hash.lower()
    _require_inside(output_dir, root, "MATERIALIZER_STAGING_OUTSIDE_ROOT")
    output_dir.mkdir(parents=True, exist_ok=True)
    output_blend = output_dir / "vehicle.blend"
    output_chassis_glb = output_dir / "vehicle.glb"
    output_front_wheel_glb = output_dir / "wheel-front.glb"
    output_rear_wheel_glb = output_dir / "wheel-rear.glb"
    output_preview_glb = output_dir / "vehicle-preview.glb"
    report_path = output_dir / "materialization-report.json"
    worker = repo / "blender" / "vehicle_studio" / "materialize_build_ir.py"
    if not worker.is_file():
        raise MaterializationError("MATERIALIZER_WORKER_MISSING", "Blender materializer worker is missing")
    config = {
        "source": str(source), "source_sha256": expected_hash, "sources": sources,
        "material_sources": material_sources,
        "build_ir": build_ir, "output_blend": str(output_blend),
        "output_chassis_glb": str(output_chassis_glb),
        "output_front_wheel_glb": str(output_front_wheel_glb),
        "output_rear_wheel_glb": str(output_rear_wheel_glb),
        "output_preview_glb": str(output_preview_glb),
    }
    with tempfile.TemporaryDirectory(prefix="formula90_materialize_") as temporary:
        config_path = Path(temporary) / "config.json"
        config_path.write_text(json.dumps(config, sort_keys=True, separators=(",", ":")), encoding="utf-8")
        try:
            completed = subprocess.run(
                [str(executable), "--background", "--factory-startup", "--python", str(worker),
                 "--", "--config", str(config_path)],
                capture_output=True, text=True, encoding="utf-8", errors="replace",
                timeout=timeout, check=False)
        except (OSError, subprocess.TimeoutExpired) as exc:
            raise MaterializationError("MATERIALIZER_BLENDER_FAILED", str(exc)) from exc
    payload = _sentinel_payload(completed.stdout or "")
    if payload is None:
        raise MaterializationError("MATERIALIZER_NO_REPORT", f"Blender returned no report (exit {completed.returncode})")
    report = json.loads(payload)
    if not report.get("ok"):
        raise MaterializationError(report.get("code", "MATERIALIZER_WORKER_FAILED"), report.get("error", "worker failed"))
    if _sha256(source) != before_hash:
        raise MaterializationError("MATERIALIZER_SOURCE_MUTATED", "source changed during materialization")
    artifacts = {
        "blend": output_blend, "chassis_glb": output_chassis_glb,
        "front_wheel_glb": output_front_wheel_glb, "rear_wheel_glb": output_rear_wheel_glb,
        "preview_glb": output_preview_glb,
    }
    if any(not path.is_file() for path in artifacts.values()):
        raise MaterializationError("MATERIALIZER_ARTIFACT_MISSING", "worker did not produce every required artifact")
    report.update({"build_ir_sha256": build_hash, "output_blend": output_blend.as_posix(),
                   "output_glb": output_preview_glb.as_posix(), "source_unchanged": True,
                   "artifacts": {name: {"path": path.as_posix(), "sha256": _sha256(path)}
                                 for name, path in sorted(artifacts.items())}})
    report_path.write_bytes(canonical_json_bytes(report))
    return report


def _preflight_operations(build_ir: dict[str, Any]) -> None:
    for operation in build_ir["operations"]:
        if operation.get("kind") in INFRASTRUCTURE_KINDS:
            continue
        inputs = operation.get("inputs", {})
        changed = inputs.get("target_value") != inputs.get("baseline_value")
        if changed and inputs.get("semantic_role") not in SUPPORTED_CHANGED_ROLES:
            raise MaterializationError(
                "MATERIALIZER_UNSUPPORTED_CHANGED_OPERATION",
                f"operation {operation.get('kind')!r} is not implemented for changed values",
            )


def _require_inside(path: Path, root: Path, code: str) -> None:
    try:
        path.relative_to(root)
    except ValueError as exc:
        raise MaterializationError(code, "path escapes its allowed root") from exc


def _sha256(path: Path) -> str:
    digest = sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest().upper()


def _sentinel_payload(stdout: str) -> str | None:
    for line in stdout.splitlines():
        if line.startswith(SENTINEL):
            return line[len(SENTINEL):]
    return None
