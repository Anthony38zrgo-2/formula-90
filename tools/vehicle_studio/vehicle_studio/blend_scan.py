"""Read-only deterministic Blend scan adapter."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
from typing import Any

from .canonical import canonical_json_bytes


DEFAULT_BLENDER_EXE = Path(r"C:\Program Files\Blender Foundation\Blender 5.2\blender.exe")
SENTINEL = "__FORMULA90_VEHICLE_STUDIO_BLEND_JSON__"
ADAPTER_VERSION = 1


class BlendScanError(ValueError):
    pass


def scan_blend(
    source: Path,
    *,
    repo_root: Path,
    project_root: Path | None = None,
    blender_executable: Path = DEFAULT_BLENDER_EXE,
    objects: list[str] | None = None,
    timeout: float = 300.0,
) -> dict[str, Any]:
    repo = Path(repo_root).resolve()
    project = Path(project_root).resolve() if project_root is not None else repo
    path = Path(source)
    if not path.is_absolute():
        path = project / path
    path = path.resolve()
    relative = _relative_path(path, project)
    if path.suffix.lower() != ".blend":
        raise BlendScanError("source must use .blend extension")
    if not path.is_file():
        raise BlendScanError(f"Blend source does not exist: {relative}")
    executable = Path(blender_executable).resolve()
    if not executable.is_file():
        raise BlendScanError(f"Blender executable not found: {executable}")
    requested = objects or []
    if not isinstance(requested, list) or not all(isinstance(item, str) for item in requested):
        raise BlendScanError("objects must be an array of names")
    probe = repo / "blender" / "vehicle_studio" / "probe_blend.py"
    if not probe.is_file():
        raise BlendScanError("Blend probe not found: blender/vehicle_studio/probe_blend.py")

    before_hash = _sha256(path)
    config = {"source": str(path), "objects": sorted(set(requested))}
    with tempfile.TemporaryDirectory(prefix="formula90_vehicle_scan_") as temporary:
        config_path = Path(temporary) / "probe_config.json"
        config_path.write_text(
            json.dumps(config, sort_keys=True, separators=(",", ":")),
            encoding="utf-8",
        )
        command = [
            str(executable),
            "--background",
            "--factory-startup",
            "--python",
            str(probe),
            "--",
            "--config",
            str(config_path),
        ]
        try:
            completed = subprocess.run(
                command,
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                timeout=timeout,
                check=False,
            )
        except (OSError, subprocess.TimeoutExpired) as exc:
            raise BlendScanError(f"Blender probe failed to start or timed out: {exc}") from exc

    payload = _sentinel_payload(completed.stdout or "")
    if payload is None:
        raise BlendScanError(
            f"Blend probe produced no JSON result (exit {completed.returncode})"
        )
    try:
        evidence = json.loads(payload)
    except json.JSONDecodeError as exc:
        raise BlendScanError(f"Blend probe returned malformed JSON: {exc}") from exc
    if not isinstance(evidence, dict):
        raise BlendScanError("Blend probe returned a non-object result")
    if not evidence.get("ok"):
        raise BlendScanError(
            f"Blend probe {evidence.get('class', 'error')}: {evidence.get('error', 'unknown error')}"
        )
    after_hash = _sha256(path)
    if before_hash != after_hash:
        raise BlendScanError("source Blend changed during read-only scan")

    report = {
        "scan_schema_version": 1,
        "adapter": {
            "id": "formula90-vehicle-studio-blend-scan",
            "version": ADAPTER_VERSION,
            "probe_owner": "blender/vehicle_studio/probe_blend.py",
        },
        "source": {
            "path": relative,
            "size_bytes": path.stat().st_size,
            "sha256": before_hash,
        },
        "findings": _findings(evidence),
        "evidence": evidence,
    }
    return report


def scan_blend_bytes(source: Path, **kwargs: Any) -> bytes:
    return canonical_json_bytes(scan_blend(source, **kwargs))


def _relative_path(path: Path, root: Path) -> str:
    try:
        return path.relative_to(root).as_posix()
    except ValueError as exc:
        raise BlendScanError("source must remain inside the project root") from exc


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest().upper()


def _sentinel_payload(stdout: str) -> str | None:
    for line in stdout.splitlines():
        if line.startswith(SENTINEL):
            return line[len(SENTINEL) :]
    return None


def _findings(evidence: dict[str, Any]) -> list[dict[str, str]]:
    findings: list[dict[str, str]] = []
    for obj in evidence.get("objects", []):
        name = obj.get("name", "<unnamed>")
        if not obj.get("exists", True):
            findings.append(
                {
                    "code": "BLEND_OBJECT_MISSING",
                    "severity": "ERROR",
                    "message": f"requested object is missing: {name}",
                }
            )
            continue
        if obj.get("world_determinant", 0) < 0:
            findings.append(
                {
                    "code": "BLEND_NEGATIVE_WORLD_DETERMINANT",
                    "severity": "ERROR",
                    "message": f"object has negative world determinant: {name}",
                }
            )
        mesh = obj.get("mesh")
        if isinstance(mesh, dict):
            if not mesh.get("finite_vertices", False):
                findings.append(
                    {
                        "code": "BLEND_NON_FINITE_VERTICES",
                        "severity": "ERROR",
                        "message": f"object has non-finite vertices: {name}",
                    }
                )
            if not mesh.get("uv_layers"):
                findings.append(
                    {
                        "code": "BLEND_UV_MISSING",
                        "severity": "WARNING",
                        "message": f"mesh has no UV layer: {name}",
                    }
                )
    for material in evidence.get("materials", []):
        for image_path in material.get("image_paths", []):
            if image_path and not image_path.startswith("//"):
                findings.append(
                    {
                        "code": "BLEND_IMAGE_PATH_NON_RELATIVE",
                        "severity": "WARNING",
                        "message": f"material image path is not Blend-relative: {material.get('name', '<unnamed>')}",
                    }
                )
    return sorted(findings, key=lambda item: (item["code"], item["severity"], item["message"]))

