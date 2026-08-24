"""Read-only deterministic GLB scan adapter for Vehicle Studio."""

from __future__ import annotations

from copy import deepcopy
import importlib.util
from pathlib import Path
from types import ModuleType
from typing import Any

from .canonical import canonical_json_bytes


ADAPTER_VERSION = 1


class GLBScanError(ValueError):
    pass


def load_existing_inspector(repo_root: Path) -> ModuleType:
    root = Path(repo_root).resolve()
    source = root / "blender" / "vehicle_pipeline" / "inspect_glb.py"
    if not source.is_file():
        raise GLBScanError(
            "reuse boundary broken: blender/vehicle_pipeline/inspect_glb.py not found"
        )
    spec = importlib.util.spec_from_file_location(
        "formula90_vehicle_pipeline_inspect_glb", source
    )
    if spec is None or spec.loader is None:
        raise GLBScanError("could not load existing GLB inspector")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def scan_glb(
    source: Path,
    *,
    repo_root: Path,
    full: bool = True,
    normal_tolerance: float = 0.01,
) -> dict[str, Any]:
    root = Path(repo_root).resolve()
    path = Path(source)
    if not path.is_absolute():
        path = root / path
    path = path.resolve()
    relative = _relative_repo_path(path, root)
    if path.suffix.lower() != ".glb":
        raise GLBScanError("source must use .glb extension")
    if not path.is_file():
        raise GLBScanError(f"GLB source does not exist: {relative}")
    if not isinstance(normal_tolerance, (int, float)) or normal_tolerance < 0:
        raise GLBScanError("normal_tolerance must be a non-negative number")

    inspector = load_existing_inspector(root)
    try:
        evidence = inspector.inspect(
            str(path),
            full=bool(full),
            normal_tolerance=float(normal_tolerance),
        )
    except (OSError, ValueError) as exc:
        raise GLBScanError(f"GLB structural error: {exc}") from exc

    normalized = deepcopy(evidence)
    normalized["file"] = relative
    report = {
        "scan_schema_version": 1,
        "adapter": {
            "id": "formula90-vehicle-studio-glb-scan",
            "version": ADAPTER_VERSION,
            "reuse_owner": "blender/vehicle_pipeline/inspect_glb.py",
        },
        "source": {
            "path": relative,
            "size_bytes": evidence["size_bytes"],
            "sha256": evidence["sha256"],
        },
        "findings": _findings(normalized),
        "evidence": normalized,
    }
    return report


def scan_glb_bytes(
    source: Path,
    *,
    repo_root: Path,
    full: bool = True,
    normal_tolerance: float = 0.01,
) -> bytes:
    return canonical_json_bytes(
        scan_glb(
            source,
            repo_root=repo_root,
            full=full,
            normal_tolerance=normal_tolerance,
        )
    )


def _relative_repo_path(path: Path, root: Path) -> str:
    try:
        return path.relative_to(root).as_posix()
    except ValueError as exc:
        raise GLBScanError("source must remain inside the repository root") from exc


def _findings(evidence: dict[str, Any]) -> list[dict[str, Any]]:
    findings: list[dict[str, Any]] = []
    if not evidence.get("finite_positions_normals", False):
        findings.append(
            {
                "code": "GLB_NON_FINITE_GEOMETRY",
                "severity": "ERROR",
                "message": "positions or normals contain non-finite values",
            }
        )
    primitive_count = int(evidence.get("attribute_counts", {}).get("POSITION", {}).get("primitive_count", 0))
    uv_count = int(evidence.get("attribute_counts", {}).get("TEXCOORD_0", {}).get("primitive_count", 0))
    normal_count = int(evidence.get("attribute_counts", {}).get("NORMAL", {}).get("primitive_count", 0))
    if uv_count != primitive_count:
        findings.append(
            {
                "code": "GLB_UV0_INCOMPLETE",
                "severity": "ERROR",
                "message": f"UV0 covers {uv_count} of {primitive_count} primitives",
            }
        )
    if normal_count != primitive_count:
        findings.append(
            {
                "code": "GLB_NORMALS_INCOMPLETE",
                "severity": "WARNING",
                "message": f"normals cover {normal_count} of {primitive_count} primitives",
            }
        )
    if evidence.get("negative_scale_nodes"):
        findings.append(
            {
                "code": "GLB_NEGATIVE_SCALE",
                "severity": "ERROR",
                "message": "one or more nodes use negative scale",
            }
        )
    if evidence.get("negative_world_determinant_nodes"):
        findings.append(
            {
                "code": "GLB_NEGATIVE_WORLD_DETERMINANT",
                "severity": "ERROR",
                "message": "one or more nodes have negative world determinant",
            }
        )
    if evidence.get("counts", {}).get("images", 0):
        findings.append(
            {
                "code": "GLB_EMBEDDED_IMAGES",
                "severity": "INFO",
                "message": "GLB contains embedded image resources",
            }
        )
    return sorted(
        findings,
        key=lambda item: (item["code"], item["severity"], item["message"]),
    )

