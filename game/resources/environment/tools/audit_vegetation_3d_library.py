from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path

import numpy as np
import trimesh


DIMENSION_TOLERANCE_M = 0.002
MAX_TRIANGLES_PER_ASSET = 10_000


def _geometry_by_role(scene: trimesh.Scene) -> dict[str, trimesh.Trimesh]:
    roles: dict[str, trimesh.Trimesh] = {}
    for name, geometry in scene.geometry.items():
        lowered = str(name).lower()
        role = "foliage" if "foliage" in lowered else "wood" if "wood" in lowered else lowered
        roles[role] = geometry
    return roles


def _has_nonwhite_vertex_colors(mesh: trimesh.Trimesh) -> bool:
    colors = np.asarray(mesh.visual.vertex_colors)
    return len(colors) == len(mesh.vertices) and bool(np.any(colors[:, :3] < 242))


def _is_double_sided_geometry(mesh: trimesh.Trimesh) -> bool:
    counts = Counter(tuple(sorted(int(index) for index in face)) for face in mesh.faces)
    return bool(counts) and all(count == 2 for count in counts.values())


def inspect_glb(path: Path) -> dict:
    scene = trimesh.load(path, force="scene", process=False)
    roles = _geometry_by_role(scene)
    meshes = list(roles.values())
    vertices = np.vstack([mesh.vertices for mesh in meshes])
    mins, maxs = vertices.min(axis=0), vertices.max(axis=0)
    dimensions = maxs - mins
    return {
        "roles": sorted(roles),
        "dimensions_m": {"width": float(dimensions[0]), "height": float(dimensions[1]), "depth": float(dimensions[2])},
        "minimum_y_m": float(mins[1]),
        "triangles": {role: int(len(mesh.faces)) for role, mesh in roles.items()},
        "colored_roles": sorted(role for role, mesh in roles.items() if _has_nonwhite_vertex_colors(mesh)),
        "foliage_double_sided_geometry": "foliage" in roles and _is_double_sided_geometry(roles["foliage"]),
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
    }


def audit_asset(repo: Path, asset: dict) -> tuple[dict, list[str]]:
    report = inspect_glb(repo / asset["glb"])
    failures: list[str] = []
    if report["roles"] != ["foliage", "wood"]:
        failures.append(f"roles={report['roles']}")
    if report["colored_roles"] != ["foliage", "wood"]:
        failures.append(f"colored_roles={report['colored_roles']}")
    if not report["foliage_double_sided_geometry"]:
        failures.append("foliage is not double-sided geometry")
    if report["sha256"] != asset["sha256"]:
        failures.append("sha256 mismatch")
    for key, actual in report["dimensions_m"].items():
        if abs(actual - float(asset["dimensions_m"][key])) > DIMENSION_TOLERANCE_M:
            failures.append(f"{key} declared={asset['dimensions_m'][key]} actual={actual:.6f}")
    total_triangles = sum(report["triangles"].values())
    if total_triangles > MAX_TRIANGLES_PER_ASSET:
        failures.append(f"triangle budget exceeded: {total_triangles}")
    if report["minimum_y_m"] > 0.002 or report["minimum_y_m"] < -float(asset["dimensions_m"]["height"]) * 0.08:
        failures.append(f"invalid ground anchor minimum_y={report['minimum_y_m']:.6f}")
    if asset.get("lod") is not False or asset.get("collision") is not False:
        failures.append("LOD/collision contract mismatch")
    return report, failures


def audit_manifest(repo: Path, manifest_path: Path) -> dict:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    results, failures = [], []
    families = {"tree": set(), "bush": set()}
    for asset in manifest.get("assets", []):
        report, asset_failures = audit_asset(repo, asset)
        results.append({"id": asset["id"], **report, "failures": asset_failures})
        failures.extend(f"{asset['id']}: {failure}" for failure in asset_failures)
        families[asset["kind"]].add(asset.get("family"))
    if len(manifest.get("assets", [])) != 12:
        failures.append(f"manifest asset count={len(manifest.get('assets', []))}, expected=12")
    for kind in ("tree", "bush"):
        if len(families[kind]) != 6 or None in families[kind]:
            failures.append(f"{kind} families are not six explicit unique values")
    return {"ok": not failures, "asset_count": len(results), "failures": failures, "assets": results}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--manifest", type=Path, default=Path("game/resources/environment/assets/manifest.json"))
    args = parser.parse_args()
    manifest_path = args.manifest if args.manifest.is_absolute() else args.repo / args.manifest
    report = audit_manifest(args.repo, manifest_path)
    print(json.dumps(report, indent=2))
    return 0 if report["ok"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
