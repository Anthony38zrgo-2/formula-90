#!/usr/bin/env python3
"""Build a Formula-90 runtime while preserving the source assembly exactly.

The fully assembled source GLB is the only geometric golden reference.  One
rigid T_vehicle maps SourceSpace to RuntimeSpace and is applied to every fixed
vehicle component and datum.  Wheel assets may be hub-local, but their runtime
placements are T_vehicle(H); this is a coordinate factorization, not an
independent recentering operation.

Annotation contract:
- wheel nodes: {prefix}_{TIRE|TYRE|RIM|HUB|SPOKE|BRAKE_DISC}_{LF|RF|LR|RR}[_N]
- datum markers: DATUM_NOSE / DATUM_TAIL / DATUM_ORIGIN /
  DATUM_FRONT_AXLE_CENTER / DATUM_REAR_AXLE_CENTER / DATUM_HUB_{FL,FR,RL,RR}
- vertex-color sentinels: red=nose green=tail blue=origin
  yellow=hub_fl magenta=hub_rl cyan=hub_fr white=hub_rr
- sidecar: <candidate_id>_datums.json

See .agents/prompts/normalizar_coche_para_godot_prompt.txt for the full spec.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import tempfile
from pathlib import Path

import numpy as np
import trimesh

from add_gltf_normals import normal_coverage
from validate_assembly_equivalence import validate as validate_assembly_equivalence
from validate_assembly_equivalence import write_source_assembly

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SOURCE = REPO_ROOT / "assets-lowpoly-python/vehicles/canonical/formula_reference_livery_contract_annotated.glb"
DEFAULT_OUTPUT = REPO_ROOT / "game/assets/models/vehicles/jordan_197"

WHEEL_PARTS = ("TIRE", "TYRE", "RIM", "HUB", "SPOKE", "BRAKE_DISC", "BRAKE")
WHEEL_CORNERS = ("LF", "RF", "LR", "RR")

SENTINEL_COLORS = {
    (255, 0, 0, 255): "nose",
    (0, 255, 0, 255): "tail",
    (0, 0, 255, 255): "origin",
    (255, 255, 0, 255): "hub_fl",
    (255, 0, 255, 255): "hub_rl",
    (0, 255, 255, 255): "hub_fr",
    (255, 255, 255, 255): "hub_rr",
}

DATUM_NODE_KEYS = {
    "DATUM_NOSE": "nose",
    "DATUM_TAIL": "tail",
    "DATUM_ORIGIN": "origin",
    "DATUM_FRONT_AXLE_CENTER": "front_axle",
    "DATUM_REAR_AXLE_CENTER": "rear_axle",
    "DATUM_HUB_FL": "hub_fl",
    "DATUM_HUB_FR": "hub_fr",
    "DATUM_HUB_RL": "hub_rl",
    "DATUM_HUB_RR": "hub_rr",
}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def load_scene(path: Path) -> trimesh.Scene:
    if not path.is_file():
        raise FileNotFoundError(f"source GLB not found: {path}")
    coverage = normal_coverage(path)
    if coverage["missing_normals"]:
        raise ValueError(
            "source GLB is missing vertex normals on "
            f"{coverage['missing_normals']}/{coverage['triangle_primitives']} triangle primitives; "
            "run add_gltf_normals.py before generating runtime assets"
        )
    scene = trimesh.load(path, force="scene", process=False)
    if not isinstance(scene, trimesh.Scene) or not scene.geometry:
        raise ValueError(f"source is not a scene with geometry: {path}")
    return scene


def node_centroid(scene: trimesh.Scene, node: str) -> np.ndarray:
    transform, geometry_name = scene.graph[node]
    mesh = scene.geometry[geometry_name].copy()
    mesh.apply_transform(transform)
    return mesh.bounds.mean(axis=0)


def datum_markers(scene: trimesh.Scene) -> dict[str, np.ndarray]:
    found: dict[str, np.ndarray] = {}
    for node in scene.graph.nodes_geometry:
        if node in DATUM_NODE_KEYS:
            found[DATUM_NODE_KEYS[node]] = node_centroid(scene, node)
    return found


def vertex_sentinels(scene: trimesh.Scene) -> dict[str, np.ndarray]:
    clusters: dict[str, list[np.ndarray]] = {}
    for node in scene.graph.nodes_geometry:
        transform, geometry_name = scene.graph[node]
        mesh = scene.geometry[geometry_name]
        colors = getattr(mesh.visual, "vertex_colors", None)
        if colors is None:
            continue
        colors = np.asarray(colors, dtype=np.uint8)
        vertices = mesh.vertices.copy()
        vertices = trimesh.transform_points(vertices, transform)
        for sentinel, key in SENTINEL_COLORS.items():
            mask = np.all(colors == np.array(sentinel, dtype=np.uint8), axis=1)
            if mask.any():
                clusters.setdefault(key, []).append(vertices[mask].mean(axis=0))
    return {key: np.mean(points, axis=0) for key, points in clusters.items() if points}


def read_datums_file(path: Path | None) -> dict[str, object] | None:
    if not path:
        return None
    if not path.is_file():
        raise FileNotFoundError(f"datums sidecar not found: {path}")
    data = json.loads(path.read_text(encoding="utf-8"))
    return data


def sidecar_datums(data: dict[str, object]) -> dict[str, np.ndarray]:
    found: dict[str, np.ndarray] = {}
    for key in ("nose", "tail", "origin"):
        if key in data and data[key] is not None:
            found[key] = np.array(data[key], dtype=float)
    hubs = data.get("wheel_hubs")
    if isinstance(hubs, dict):
        for corner in ("FL", "FR", "RL", "RR"):
            if corner in hubs and hubs[corner] is not None:
                found[f"hub_{corner.lower()}"] = np.array(hubs[corner], dtype=float)
    return found


def axle_datums(
    datums: dict[str, np.ndarray], sidecar: dict[str, object] | None, tolerance: float
) -> tuple[np.ndarray, np.ndarray]:
    axles: dict[str, np.ndarray] = {}
    for key, scalar_key in (("front_axle", "front_axle_z"), ("rear_axle", "rear_axle_z")):
        point = datums.get(key)
        if point is not None and sidecar and scalar_key in sidecar:
            delta = abs(float(point[2]) - float(sidecar[scalar_key]))
            if delta > tolerance:
                raise ValueError(
                    f"{scalar_key} disagrees between markers and sidecar by {delta:.4f} m "
                    f"(tolerance {tolerance} m)"
                )
        if point is None:
            if not sidecar or scalar_key not in sidecar:
                raise ValueError(f"missing datum {key}; provide a DATUM marker or the sidecar scalar")
            point = np.array([0.0, 0.0, float(sidecar[scalar_key])])
        axles[key] = point
    return axles["front_axle"], axles["rear_axle"]


def merge_datums(*sources: dict[str, np.ndarray], tolerance: float) -> dict[str, np.ndarray]:
    merged: dict[str, np.ndarray] = {}
    for source in sources:
        for key, value in source.items():
            if key not in merged:
                merged[key] = value
                continue
            delta = float(np.linalg.norm(value - merged[key]))
            if delta > tolerance:
                raise ValueError(
                    f"datum {key} disagrees across sources by {delta:.4f} m "
                    f"(tolerance {tolerance} m)"
                )
    return merged


def wheel_corner(node: str, prefix: str) -> str | None:
    base = prefix.rstrip("_")
    pattern = re.compile(
        rf"^{re.escape(base)}_({'|'.join(WHEEL_PARTS)})_({'|'.join(WHEEL_CORNERS)})(?:_\d+)?$"
    )
    match = pattern.match(node)
    if not match:
        return None
    return match.group(2)


def select_nodes(scene: trimesh.Scene, prefix: str) -> tuple[list[str], dict[str, list[str]]]:
    chassis: list[str] = []
    wheels: dict[str, list[str]] = {corner: [] for corner in WHEEL_CORNERS}
    for node in sorted(scene.graph.nodes_geometry):
        if node.startswith("DATUM_"):
            continue
        corner = wheel_corner(node, prefix)
        if corner is None:
            chassis.append(node)
        else:
            wheels[corner].append(node)
    return chassis, wheels


def correction_matrix(datums: dict[str, np.ndarray]) -> np.ndarray:
    forward = datums["nose"] - datums["tail"]
    if float(np.linalg.norm(forward)) == 0.0:
        raise ValueError("nose and tail datums coincide; cannot derive forward axis")
    forward = forward / np.linalg.norm(forward)
    if abs(float(np.dot(forward, [0, 0, 1]))) > 0.98:
        if forward[2] > 0:
            # Source contract: +X left / +Z forward. Runtime: +X right / -Z forward.
            # A proper 180-degree Y rotation changes both signs with det(R)=+1.
            return np.array([[-1, 0, 0, 0], [0, 1, 0, 0], [0, 0, -1, 0], [0, 0, 0, 1]], dtype=float)
        return np.eye(4)
    raise ValueError(
        f"unsupported forward axis {np.round(forward, 3)}; expected +Z or -Z (glTF contract)"
    )


def validate_selection(
    scene: trimesh.Scene,
    chassis: list[str],
    wheels: dict[str, list[str]],
    datums: dict[str, np.ndarray],
    cluster_tolerance: float,
) -> None:
    if not chassis:
        raise ValueError("no chassis geometry selected")
    for corner in ("LF", "RF", "LR", "RR"):
        if not wheels[corner]:
            raise ValueError(f"no wheel geometry for corner {corner}")
    for corner, hub_key in (("LF", "hub_fl"), ("LR", "hub_rl"), ("RF", "hub_fr"), ("RR", "hub_rr")):
        if hub_key not in datums:
            raise ValueError(f"missing datum {hub_key} needed to recenter wheel {corner}")
        origin = datums[hub_key]
        for node in wheels[corner]:
            center = node_centroid(scene, node)
            offset = float(np.linalg.norm(center - origin))
            if offset > cluster_tolerance:
                raise ValueError(
                    f"wheel node {node} centroid is {offset:.4f} m from hub {hub_key}; "
                    f"tolerance {cluster_tolerance} m"
                )


def build_scene(
    source: trimesh.Scene, nodes: list[str], source_origin: np.ndarray, rotation: np.ndarray
) -> trimesh.Scene:
    """Export R * (P - source_origin).

    For the chassis source_origin is the one vehicle origin. For a wheel it is
    that wheel's SOURCE hub H, paired with runtime placement T_vehicle(H).
    """
    output = trimesh.Scene()
    source_to_local = np.eye(4)
    source_to_local[:3, :3] = rotation[:3, :3]
    source_to_local[:3, 3] = -(rotation[:3, :3] @ source_origin)
    for node in nodes:
        transform, geometry_name = source.graph[node]
        mesh = source.geometry[geometry_name].copy()
        mesh.apply_transform(source_to_local @ transform)
        output.add_geometry(mesh, geom_name=geometry_name, node_name=node)
    return output


def validate_export(path: Path, label: str, expected_nodes: int) -> dict[str, object]:
    coverage = normal_coverage(path)
    if coverage["missing_normals"]:
        raise ValueError(
            f"{label} export lost vertex normals on "
            f"{coverage['missing_normals']}/{coverage['triangle_primitives']} triangle primitives"
        )
    scene = trimesh.load(path, force="scene", process=False)
    geometry_count = len(scene.geometry)
    vertex_count = sum(len(mesh.vertices) for mesh in scene.geometry.values())
    materials = sorted({str(getattr(getattr(m.visual, "material", None), "name", None)) for m in scene.geometry.values()})
    if geometry_count != expected_nodes:
        raise ValueError(f"{label} export has {geometry_count} geometries, expected {expected_nodes}")
    if vertex_count == 0:
        raise ValueError(f"{label} export has no vertices")
    if "None" in materials:
        raise ValueError(f"{label} export lost material assignments")
    if not np.isfinite(scene.bounds).all():
        raise ValueError(f"{label} export has non-finite bounds")
    return {
        "geometries": geometry_count,
        "vertices": vertex_count,
        "materials": materials,
        "bounds": scene.bounds.tolist(),
        "normal_coverage": coverage,
    }


def publish(candidate: Path, target: Path) -> None:
    try:
        os.replace(candidate, target)
    except PermissionError as error:
        raise PermissionError(
            f"target is locked by another process; close it before rebuilding: {target}"
        ) from error


def build(args: argparse.Namespace) -> dict[str, object]:
    source_path: Path = args.source.resolve()
    output_dir: Path = args.output_dir.resolve()
    candidate_id = args.candidate_id or output_dir.name

    scene = load_scene(source_path)
    sidecar = read_datums_file(args.datums.resolve() if args.datums else None)
    sidecar_data = sidecar_datums(sidecar) if sidecar else {}
    markers = datum_markers(scene)
    sentinels = vertex_sentinels(scene)
    datums = merge_datums(markers, sentinels, sidecar_data, tolerance=args.datum_tolerance)

    if args.wheel_prefix:
        prefix = args.wheel_prefix
    elif sidecar and sidecar.get("wheel_name_prefix"):
        prefix = str(sidecar["wheel_name_prefix"])
    else:
        prefix = "LP_"
    chassis, wheels = select_nodes(scene, prefix)
    validate_selection(scene, chassis, wheels, datums, args.cluster_tolerance)
    rotation = correction_matrix(datums)
    source_vehicle_origin = np.mean(
        [datums[f"hub_{corner}"] for corner in ("fl", "fr", "rl", "rr")], axis=0
    )
    translation = -(rotation[:3, :3] @ source_vehicle_origin)
    t_vehicle = np.eye(4)
    t_vehicle[:3, :3] = rotation[:3, :3]
    t_vehicle[:3, 3] = translation

    products = {
        "chassis": (f"{candidate_id}_chassis.glb", source_vehicle_origin, chassis),
        "wheel_fl": (f"{candidate_id}_wheel_fl.glb", datums["hub_fl"], wheels["LF"]),
        "wheel_fr": (f"{candidate_id}_wheel_fr.glb", datums["hub_fr"], wheels["RF"]),
        "wheel_rl": (f"{candidate_id}_wheel_rl.glb", datums["hub_rl"], wheels["LR"]),
        "wheel_rr": (f"{candidate_id}_wheel_rr.glb", datums["hub_rr"], wheels["RR"]),
    }

    output_dir.mkdir(parents=True, exist_ok=True)
    reports: dict[str, object] = {}
    with tempfile.TemporaryDirectory(prefix=f"{candidate_id}_build_", dir=output_dir.parent) as temporary:
        temp_dir = Path(temporary)
        for label, (filename, origin, nodes) in products.items():
            candidate = temp_dir / filename
            build_scene(scene, nodes, origin, rotation).export(candidate, include_normals=True)
            reports[label] = validate_export(candidate, label, len(nodes))

        assets = {
            label: f"res://assets/models/vehicles/{candidate_id}/{filename}"
            for label, (filename, _, _) in products.items()
        }
        source_hubs = {
            corner.upper(): datums[f"hub_{corner}"].tolist()
            for corner in ("fl", "fr", "rl", "rr")
        }
        runtime_hubs = {
            corner: trimesh.transform_points(
                np.asarray([datums[f"hub_{corner.lower()}"]]), t_vehicle
            )[0].tolist()
            for corner in ("FL", "FR", "RL", "RR")
        }
        front_axle, rear_axle = axle_datums(datums, sidecar, args.datum_tolerance)
        front_axle_runtime = trimesh.transform_points(np.asarray([front_axle]), t_vehicle)[0]
        rear_axle_runtime = trimesh.transform_points(np.asarray([rear_axle]), t_vehicle)[0]
        front_bounds = np.asarray(reports["wheel_fl"]["bounds"], dtype=float)
        rear_bounds = np.asarray(reports["wheel_rl"]["bounds"], dtype=float)

        manifest = {
            "schema_version": 2,
            "schema": "formula90s/vehicle-runtime/v2",
            "vehicle_id": candidate_id,
            "source": {
                "asset": source_path.relative_to(REPO_ROOT).as_posix(),
                "sha256": sha256(source_path),
                "forward": "+Z" if (datums["nose"] - datums["tail"])[2] > 0 else "-Z",
                "up": "+Y",
                "left": "+X",
                "units": "meters",
                "golden_reference": "original_fully_assembled_source_glb",
            },
            "runtime": {
                "forward": "-Z",
                "up": "+Y",
                "right": "+X",
                "units": "meters",
                "conversion_owner": "export_pipeline_single_T_vehicle",
                "source_origin_used_for_normalization": source_vehicle_origin.tolist(),
                "source_to_runtime_rotation_3x3": rotation[:3, :3].tolist(),
                "translation_after_rotation": translation.tolist(),
            },
            "wheel_hubs_source_m": source_hubs,
            "wheel_hubs_runtime_m": runtime_hubs,
            "front_axle_center_runtime_m": front_axle_runtime.tolist(),
            "rear_axle_center_runtime_m": rear_axle_runtime.tolist(),
            "wheelbase_m": float(np.linalg.norm(front_axle - rear_axle)),
            "front_track_m": float(np.linalg.norm(datums["hub_fl"] - datums["hub_fr"])),
            "rear_track_m": float(np.linalg.norm(datums["hub_rl"] - datums["hub_rr"])),
            "front_tire": {
                "radius_m": float(max(abs(front_bounds[0, 1]), abs(front_bounds[1, 1]))),
                "width_mm": float((front_bounds[1, 0] - front_bounds[0, 0]) * 1000.0),
            },
            "rear_tire": {
                "radius_m": float(max(abs(rear_bounds[0, 1]), abs(rear_bounds[1, 1]))),
                "width_mm": float((rear_bounds[1, 0] - rear_bounds[0, 0]) * 1000.0),
            },
            "assets": assets,
            "export_validation": reports,
        }
        manifest_path = temp_dir / "vehicle_runtime_manifest.json"
        manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        source_assembly_path = temp_dir / "source_assembly.json"
        write_source_assembly(source_path, source_assembly_path)
        overlay_path = temp_dir / "assembly_equivalence_overlay.glb"
        equivalence_report = validate_assembly_equivalence(
            source_path,
            temp_dir,
            manifest_path,
            args.assembly_tolerance_mm,
            overlay_path,
        )
        if equivalence_report["status"] != "PASS":
            raise ValueError(
                "assembly equivalence failed; candidate was not published: "
                + json.dumps(equivalence_report["maxima"], sort_keys=True)
            )
        report_path = temp_dir / "assembly_equivalence_report.json"
        report_path.write_text(json.dumps(equivalence_report, indent=2) + "\n", encoding="utf-8")

        manifest["assembly_equivalence"] = {
            "status": equivalence_report["status"],
            "tolerance_mm": args.assembly_tolerance_mm,
            "maxima": equivalence_report["maxima"],
            "report": "assembly_equivalence_report.json",
            "overlay": "assembly_equivalence_overlay.glb",
            "source_snapshot": "source_assembly.json",
        }
        manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")

        publication = [
            *(filename for filename, _, _ in products.values()),
            "vehicle_runtime_manifest.json",
            "source_assembly.json",
            "assembly_equivalence_report.json",
            "assembly_equivalence_overlay.glb",
        ]
        for filename in publication:
            publish(temp_dir / filename, output_dir / filename)

    return manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--datums", type=Path, default=None)
    parser.add_argument("--output-dir", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--candidate-id", type=str, default=None)
    parser.add_argument("--wheel-prefix", type=str, default=None)
    parser.add_argument("--datum-tolerance", type=float, default=0.02)
    parser.add_argument("--cluster-tolerance", type=float, default=0.15)
    parser.add_argument("--assembly-tolerance-mm", type=float, default=2.0)
    args = parser.parse_args()
    manifest = build(args)
    print(json.dumps(manifest, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
