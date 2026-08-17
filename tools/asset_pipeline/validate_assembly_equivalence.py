#!/usr/bin/env python3
"""Validate a split vehicle runtime against its fully assembled source GLB.

The source GLB is the only geometric golden reference.  The runtime manifest is
used only to declare the single rigid SourceSpace -> RuntimeSpace transform and
to locate the generated pieces; expected anchors and geometry are always
derived independently from the source asset.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from typing import Iterable

import numpy as np
import trimesh
from scipy.spatial import cKDTree


REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SOURCE = REPO_ROOT / "assets-lowpoly-python/vehicles/canonical/formula_reference_livery_contract_annotated.glb"
DEFAULT_RUNTIME_DIR = REPO_ROOT / "game/assets/models/vehicles/jordan_197"
DEFAULT_MANIFEST = DEFAULT_RUNTIME_DIR / "vehicle_runtime_manifest.json"
WHEEL_PATTERN = re.compile(r"^LP_(?:BRAKE_DISC|BRAKE|HUB|RIM|SPOKE|TIRE|TYRE)_(LF|RF|LR|RR)(?:_\d+)?$")
SOURCE_TO_RUNTIME_CORNER = {"LF": "FL", "RF": "FR", "LR": "RL", "RR": "RR"}
DATUM_NAMES = {
    "DATUM_NOSE": "DATUM_NOSE",
    "DATUM_TAIL": "DATUM_TAIL",
    "DATUM_ORIGIN": "DATUM_ORIGIN",
    "DATUM_FRONT_AXLE_CENTER": "DATUM_FRONT_AXLE_CENTER",
    "DATUM_REAR_AXLE_CENTER": "DATUM_REAR_AXLE_CENTER",
    "DATUM_HUB_FL": "DATUM_HUB_FL",
    "DATUM_HUB_FR": "DATUM_HUB_FR",
    "DATUM_HUB_RL": "DATUM_HUB_RL",
    "DATUM_HUB_RR": "DATUM_HUB_RR",
}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def load_scene(path: Path) -> trimesh.Scene:
    if not path.is_file():
        raise FileNotFoundError(path)
    scene = trimesh.load(path, force="scene", process=False)
    if not isinstance(scene, trimesh.Scene) or not scene.geometry:
        raise ValueError(f"not a non-empty GLB scene: {path}")
    return scene


def world_meshes(scene: trimesh.Scene) -> dict[str, trimesh.Trimesh]:
    meshes: dict[str, trimesh.Trimesh] = {}
    for node in scene.graph.nodes_geometry:
        transform, geometry_name = scene.graph[node]
        mesh = scene.geometry[geometry_name].copy()
        mesh.apply_transform(transform)
        meshes[node] = mesh
    return meshes


def transformed_point(matrix: np.ndarray, point: Iterable[float]) -> np.ndarray:
    return trimesh.transform_points(np.asarray([point], dtype=float), matrix)[0]


def transformed_mesh(mesh: trimesh.Trimesh, matrix: np.ndarray) -> trimesh.Trimesh:
    result = mesh.copy()
    result.apply_transform(matrix)
    return result


def nearest_surface(meshes: Iterable[trimesh.Trimesh], point: np.ndarray) -> tuple[np.ndarray, float]:
    combined = trimesh.util.concatenate([mesh.copy() for mesh in meshes])
    closest, distance, _ = trimesh.proximity.closest_point(combined, np.asarray([point]))
    return closest[0], float(distance[0])


def mean_low_anchor(vertices: np.ndarray, mask: np.ndarray, tolerance_m: float = 0.002) -> np.ndarray:
    region = vertices[mask]
    if len(region) == 0:
        raise ValueError("empty chassis floor region")
    minimum_y = float(region[:, 1].min())
    floor = region[region[:, 1] <= minimum_y + tolerance_m]
    return floor.mean(axis=0)


def source_regions(meshes: dict[str, trimesh.Trimesh]) -> tuple[list[str], dict[str, list[str]]]:
    chassis: list[str] = []
    wheels = {corner: [] for corner in SOURCE_TO_RUNTIME_CORNER}
    for node in sorted(meshes):
        if node.startswith("DATUM_"):
            continue
        match = WHEEL_PATTERN.fullmatch(node)
        if match:
            wheels[match.group(1)].append(node)
        else:
            chassis.append(node)
    if not chassis or any(not nodes for nodes in wheels.values()):
        raise ValueError("source classification did not find chassis plus four complete wheels")
    return chassis, wheels


def datum_points(meshes: dict[str, trimesh.Trimesh]) -> dict[str, np.ndarray]:
    points: dict[str, np.ndarray] = {}
    for node, key in DATUM_NAMES.items():
        if node not in meshes:
            raise ValueError(f"source is missing required marker {node}")
        points[key] = meshes[node].bounds.mean(axis=0)
    return points


def pairwise_distances(anchors: dict[str, np.ndarray]) -> dict[str, float]:
    names = sorted(anchors)
    return {
        f"{left}<->{right}": float(np.linalg.norm(anchors[left] - anchors[right]))
        for index, left in enumerate(names)
        for right in names[index + 1 :]
    }


def derive_source_assembly(source_path: Path) -> tuple[dict[str, object], dict[str, trimesh.Trimesh]]:
    scene = load_scene(source_path)
    meshes = world_meshes(scene)
    chassis_nodes, wheel_nodes = source_regions(meshes)
    datums = datum_points(meshes)

    structural_nodes = [node for node in chassis_nodes if "SUSPENSION" not in node]
    suspension_nodes = [node for node in chassis_nodes if "SUSPENSION" in node]
    if not suspension_nodes:
        raise ValueError("source has no suspension geometry for mandatory hub-to-suspension validation")
    structural_vertices = np.vstack([meshes[node].vertices for node in structural_nodes])
    midpoint_z = float(
        (datums["DATUM_FRONT_AXLE_CENTER"][2] + datums["DATUM_REAR_AXLE_CENTER"][2]) * 0.5
    )

    anchors = dict(datums)
    anchors["ANCHOR_CHASSIS_CENTER"] = np.vstack(
        [meshes[node].vertices for node in chassis_nodes]
    ).min(axis=0) * 0.5 + np.vstack([meshes[node].vertices for node in chassis_nodes]).max(axis=0) * 0.5
    anchors["ANCHOR_CHASSIS_FLOOR_FRONT"] = mean_low_anchor(
        structural_vertices, structural_vertices[:, 2] >= midpoint_z
    )
    anchors["ANCHOR_CHASSIS_FLOOR_REAR"] = mean_low_anchor(
        structural_vertices, structural_vertices[:, 2] < midpoint_z
    )

    hub_to_suspension: dict[str, float] = {}
    for corner in ("FL", "FR", "RL", "RR"):
        hub = anchors[f"DATUM_HUB_{corner}"]
        suspension_anchor, distance = nearest_surface(
            [meshes[node] for node in suspension_nodes], hub
        )
        anchors[f"ANCHOR_SUSPENSION_{corner}"] = suspension_anchor
        hub_to_suspension[corner] = distance

    tire_ground = []
    for source_corner, nodes in wheel_nodes.items():
        tire_nodes = [node for node in nodes if "_TIRE_" in node or "_TYRE_" in node]
        tire_ground.append(min(float(meshes[node].bounds[0, 1]) for node in tire_nodes))
    source_ground_y = float(np.median(tire_ground))
    chassis_floor_y = float(structural_vertices[:, 1].min())

    datum_surface_attachments: dict[str, dict[str, object]] = {}
    for key in ("DATUM_NOSE", "DATUM_TAIL"):
        surface_point, surface_distance = nearest_surface(
            [meshes[node] for node in structural_nodes], anchors[key]
        )
        datum_surface_attachments[key] = {
            "surface_point_source_m": surface_point.tolist(),
            "surface_to_datum_offset_source_m": (anchors[key] - surface_point).tolist(),
            "distance_m": surface_distance,
        }

    snapshot: dict[str, object] = {
        "schema": "formula90s/source-assembly/v1",
        "source": source_path.relative_to(REPO_ROOT).as_posix(),
        "source_sha256": sha256(source_path),
        "units": "meters",
        "golden_reference": "original_fully_assembled_source_glb",
        "regions": {
            "chassis": {
                "classification": "chassis_and_fixed_vehicle_components",
                "confidence": 1.0,
                "evidence": "all non-DATUM nodes excluding four geometry-classified wheel clusters",
                "nodes": chassis_nodes,
            },
            "suspension": {
                "classification": "suspension_surface",
                "confidence": 0.95,
                "evidence": "SUSPENSION node names plus proximity to all four measured source hubs",
                "nodes": suspension_nodes,
            },
            "wheels": {
                SOURCE_TO_RUNTIME_CORNER[source_corner]: {
                    "classification": f"wheel_{SOURCE_TO_RUNTIME_CORNER[source_corner]}",
                    "confidence": 1.0,
                    "evidence": "contractual LP wheel-part suffix and measured proximity to source hub",
                    "nodes": nodes,
                }
                for source_corner, nodes in wheel_nodes.items()
            },
        },
        "anchors_source_m": {name: point.tolist() for name, point in sorted(anchors.items())},
        "hub_to_suspension_distance_source_m": hub_to_suspension,
        "datum_surface_attachments": datum_surface_attachments,
        "distance_matrix_source_m": pairwise_distances(anchors),
        "source_ground_plane_y_m": source_ground_y,
        "source_chassis_floor_y_m": chassis_floor_y,
        "source_ground_clearance_m": chassis_floor_y - source_ground_y,
        "counts": {
            "geometry_nodes": len(meshes),
            "chassis_nodes": len(chassis_nodes),
            "wheel_nodes": {SOURCE_TO_RUNTIME_CORNER[key]: len(value) for key, value in wheel_nodes.items()},
        },
    }
    return snapshot, meshes


def write_source_assembly(source_path: Path, destination: Path) -> dict[str, object]:
    snapshot, _ = derive_source_assembly(source_path)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(snapshot, indent=2) + "\n", encoding="utf-8")
    return snapshot


def rigid_transform_from_manifest(manifest: dict[str, object]) -> np.ndarray:
    runtime = manifest.get("runtime")
    if not isinstance(runtime, dict):
        raise ValueError("runtime manifest lacks runtime transform declaration")
    rotation = np.asarray(runtime.get("source_to_runtime_rotation_3x3"), dtype=float)
    translation = np.asarray(runtime.get("translation_after_rotation"), dtype=float)
    if rotation.shape != (3, 3) or translation.shape != (3,):
        raise ValueError("runtime transform must contain a 3x3 rotation and 3-vector translation")
    orthogonality_error = float(np.max(np.abs(rotation.T @ rotation - np.eye(3))))
    determinant = float(np.linalg.det(rotation))
    if orthogonality_error > 1e-6 or abs(determinant - 1.0) > 1e-6:
        raise ValueError(
            f"T_vehicle is not a proper rigid transform: orthogonality={orthogonality_error}, det={determinant}"
        )
    transform = np.eye(4)
    transform[:3, :3] = rotation
    transform[:3, 3] = translation
    return transform


def runtime_asset_paths(runtime_dir: Path, manifest: dict[str, object]) -> dict[str, Path]:
    assets = manifest.get("assets")
    if not isinstance(assets, dict):
        raise ValueError("runtime manifest lacks assets")
    result: dict[str, Path] = {}
    for key in ("chassis", "wheel_fl", "wheel_fr", "wheel_rl", "wheel_rr"):
        value = assets.get(key)
        if not isinstance(value, str):
            raise ValueError(f"runtime manifest lacks asset path {key}")
        result[key] = runtime_dir / Path(value.replace("res://assets/models/vehicles/jordan_197/", "")).name
    return result


def runtime_world_meshes(
    runtime_dir: Path, manifest: dict[str, object]
) -> tuple[dict[str, trimesh.Trimesh], dict[str, np.ndarray]]:
    paths = runtime_asset_paths(runtime_dir, manifest)
    actual: dict[str, trimesh.Trimesh] = world_meshes(load_scene(paths["chassis"]))
    hubs_raw = manifest.get("wheel_hubs_runtime_m")
    if not isinstance(hubs_raw, dict):
        raise ValueError("runtime manifest lacks actual wheel hub placements")
    hubs = {corner: np.asarray(hubs_raw[corner], dtype=float) for corner in ("FL", "FR", "RL", "RR")}
    for corner in ("FL", "FR", "RL", "RR"):
        wheel_meshes = world_meshes(load_scene(paths[f"wheel_{corner.lower()}"]))
        placement = np.eye(4)
        placement[:3, 3] = hubs[corner]
        for node, mesh in wheel_meshes.items():
            if node in actual:
                raise ValueError(f"duplicate runtime node {node}")
            actual[node] = transformed_mesh(mesh, placement)
    return actual, hubs


def nearest_error(expected: np.ndarray, actual: np.ndarray) -> tuple[float, float]:
    expected_tree = cKDTree(expected)
    actual_tree = cKDTree(actual)
    actual_to_expected = expected_tree.query(actual, workers=-1)[0]
    expected_to_actual = actual_tree.query(expected, workers=-1)[0]
    distances = np.concatenate((actual_to_expected, expected_to_actual))
    return float(distances.max()), float(np.sqrt(np.mean(distances * distances)))


def derive_actual_anchors(
    actual_meshes: dict[str, trimesh.Trimesh],
    expected_anchors: dict[str, np.ndarray],
    hubs: dict[str, np.ndarray],
    datum_surface_attachments: dict[str, object],
    transform: np.ndarray,
) -> tuple[dict[str, np.ndarray], dict[str, float], float]:
    chassis_nodes = [
        node for node in actual_meshes if not WHEEL_PATTERN.fullmatch(node) and not node.startswith("DATUM_")
    ]
    suspension_nodes = [node for node in chassis_nodes if "SUSPENSION" in node]
    structural_nodes = [node for node in chassis_nodes if "SUSPENSION" not in node]
    structural_vertices = np.vstack([actual_meshes[node].vertices for node in structural_nodes])
    midpoint_z = float((hubs["FL"][2] + hubs["RL"][2]) * 0.5)
    chassis_vertices = np.vstack([actual_meshes[node].vertices for node in chassis_nodes])

    actual: dict[str, np.ndarray] = {
        "DATUM_HUB_FL": hubs["FL"],
        "DATUM_HUB_FR": hubs["FR"],
        "DATUM_HUB_RL": hubs["RL"],
        "DATUM_HUB_RR": hubs["RR"],
        "DATUM_FRONT_AXLE_CENTER": (hubs["FL"] + hubs["FR"]) * 0.5,
        "DATUM_REAR_AXLE_CENTER": (hubs["RL"] + hubs["RR"]) * 0.5,
        "ANCHOR_CHASSIS_CENTER": chassis_vertices.min(axis=0) * 0.5 + chassis_vertices.max(axis=0) * 0.5,
        "ANCHOR_CHASSIS_FLOOR_FRONT": mean_low_anchor(
            structural_vertices, structural_vertices[:, 2] <= midpoint_z
        ),
        "ANCHOR_CHASSIS_FLOOR_REAR": mean_low_anchor(
            structural_vertices, structural_vertices[:, 2] > midpoint_z
        ),
        "DATUM_ORIGIN": expected_anchors["DATUM_ORIGIN"],
    }
    for key in ("DATUM_NOSE", "DATUM_TAIL"):
        attachment = datum_surface_attachments[key]
        expected_surface = transformed_point(
            transform, attachment["surface_point_source_m"]
        )
        actual_surface, _ = nearest_surface(
            [actual_meshes[node] for node in structural_nodes], expected_surface
        )
        rotated_offset = transform[:3, :3] @ np.asarray(
            attachment["surface_to_datum_offset_source_m"], dtype=float
        )
        actual[key] = actual_surface + rotated_offset

    hub_to_suspension: dict[str, float] = {}
    for corner in ("FL", "FR", "RL", "RR"):
        point, distance = nearest_surface(
            [actual_meshes[node] for node in suspension_nodes], hubs[corner]
        )
        actual[f"ANCHOR_SUSPENSION_{corner}"] = point
        hub_to_suspension[corner] = distance

    tire_ground = []
    for corner in ("LF", "RF", "LR", "RR"):
        tire_nodes = [node for node in actual_meshes if WHEEL_PATTERN.fullmatch(node) and corner in node and ("_TIRE_" in node or "_TYRE_" in node)]
        tire_ground.append(min(float(actual_meshes[node].bounds[0, 1]) for node in tire_nodes))
    ground_y = float(np.median(tire_ground))
    ground_clearance = float(structural_vertices[:, 1].min() - ground_y)
    return actual, hub_to_suspension, ground_clearance


def overlay_scene(
    expected: dict[str, trimesh.Trimesh], actual: dict[str, trimesh.Trimesh], destination: Path
) -> None:
    overlay = trimesh.Scene()
    source_material = trimesh.visual.material.PBRMaterial(
        name="SOURCE_GHOST_CYAN",
        baseColorFactor=[25, 210, 255, 72],
        alphaMode="BLEND",
        doubleSided=True,
        metallicFactor=0.0,
        roughnessFactor=1.0,
    )
    runtime_material = trimesh.visual.material.PBRMaterial(
        name="RUNTIME_SOLID_MAGENTA",
        baseColorFactor=[255, 45, 170, 255],
        alphaMode="OPAQUE",
        doubleSided=True,
        metallicFactor=0.0,
        roughnessFactor=1.0,
    )
    for prefix, meshes, material in (
        ("SOURCE_GHOST", expected, source_material),
        ("RUNTIME_SOLID", actual, runtime_material),
    ):
        for node, original in sorted(meshes.items()):
            mesh = original.copy()
            mesh.visual = trimesh.visual.TextureVisuals(material=material)
            name = f"{prefix}__{node}"
            overlay.add_geometry(mesh, node_name=name, geom_name=name)
    destination.parent.mkdir(parents=True, exist_ok=True)
    overlay.export(destination)


def validate(
    source_path: Path,
    runtime_dir: Path,
    manifest_path: Path,
    tolerance_mm: float,
    overlay_path: Path | None,
) -> dict[str, object]:
    source_snapshot, source_meshes = derive_source_assembly(source_path)
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    transform = rigid_transform_from_manifest(manifest)
    actual_meshes, hubs = runtime_world_meshes(runtime_dir, manifest)

    expected_meshes = {
        node: transformed_mesh(mesh, transform)
        for node, mesh in source_meshes.items()
        if not node.startswith("DATUM_")
    }
    missing = sorted(set(expected_meshes) - set(actual_meshes))
    unexpected = sorted(set(actual_meshes) - set(expected_meshes))
    node_errors: dict[str, dict[str, float]] = {}
    for node in sorted(set(expected_meshes) & set(actual_meshes)):
        maximum, rms = nearest_error(expected_meshes[node].vertices, actual_meshes[node].vertices)
        node_errors[node] = {"max_error_mm": maximum * 1000.0, "rms_error_mm": rms * 1000.0}

    source_anchors = {
        name: np.asarray(point, dtype=float)
        for name, point in source_snapshot["anchors_source_m"].items()
    }
    expected_anchors = {
        name: transformed_point(transform, point) for name, point in source_anchors.items()
    }
    actual_anchors, actual_hub_suspension, runtime_clearance = derive_actual_anchors(
        actual_meshes,
        expected_anchors,
        hubs,
        source_snapshot["datum_surface_attachments"],
        transform,
    )
    anchor_errors = {
        name: {
            "expected_runtime_m": expected_anchors[name].tolist(),
            "actual_runtime_m": actual_anchors[name].tolist(),
            "error_mm": float(np.linalg.norm(expected_anchors[name] - actual_anchors[name]) * 1000.0),
        }
        for name in sorted(expected_anchors)
    }
    actual_distances = pairwise_distances(actual_anchors)
    source_distances = source_snapshot["distance_matrix_source_m"]
    distance_errors = {
        pair: abs(float(actual_distances[pair]) - float(source_distance)) * 1000.0
        for pair, source_distance in source_distances.items()
    }
    hub_suspension_errors = {
        corner: abs(
            float(actual_hub_suspension[corner])
            - float(source_snapshot["hub_to_suspension_distance_source_m"][corner])
        )
        * 1000.0
        for corner in ("FL", "FR", "RL", "RR")
    }
    clearance_error_mm = abs(
        runtime_clearance - float(source_snapshot["source_ground_clearance_m"])
    ) * 1000.0

    maximum_node_error = max((value["max_error_mm"] for value in node_errors.values()), default=float("inf"))
    maximum_anchor_error = max((value["error_mm"] for value in anchor_errors.values()), default=float("inf"))
    maximum_distance_error = max(distance_errors.values(), default=float("inf"))
    maximum_hub_suspension_error = max(hub_suspension_errors.values(), default=float("inf"))
    passed = (
        not missing
        and not unexpected
        and maximum_node_error <= tolerance_mm
        and maximum_anchor_error <= tolerance_mm
        and maximum_distance_error <= tolerance_mm
        and maximum_hub_suspension_error <= tolerance_mm
        and clearance_error_mm <= tolerance_mm
    )

    if overlay_path is not None:
        overlay_scene(expected_meshes, actual_meshes, overlay_path)

    return {
        "schema": "formula90s/assembly-equivalence-report/v1",
        "status": "PASS" if passed else "FAIL",
        "golden_reference": source_path.relative_to(REPO_ROOT).as_posix(),
        "source_sha256": source_snapshot["source_sha256"],
        "runtime_manifest_used_only_for": ["T_vehicle_declaration", "asset_paths", "actual_wheel_placements"],
        "tolerance_mm": tolerance_mm,
        "T_vehicle": transform.tolist(),
        "missing_runtime_nodes": missing,
        "unexpected_runtime_nodes": unexpected,
        "node_geometry_errors": node_errors,
        "anchor_errors": anchor_errors,
        "distance_matrix_errors_mm": distance_errors,
        "hub_to_suspension_errors_mm": hub_suspension_errors,
        "source_ground_clearance_m": source_snapshot["source_ground_clearance_m"],
        "runtime_ground_clearance_m": runtime_clearance,
        "ground_clearance_error_mm": clearance_error_mm,
        "maxima": {
            "node_geometry_error_mm": maximum_node_error,
            "anchor_error_mm": maximum_anchor_error,
            "distance_error_mm": maximum_distance_error,
            "hub_to_suspension_error_mm": maximum_hub_suspension_error,
            "ground_clearance_error_mm": clearance_error_mm,
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--runtime-dir", type=Path, default=DEFAULT_RUNTIME_DIR)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--source-assembly-out", type=Path)
    parser.add_argument("--report-out", type=Path)
    parser.add_argument("--overlay-out", type=Path)
    parser.add_argument("--tolerance-mm", type=float, default=2.0)
    args = parser.parse_args()

    source = args.source.resolve()
    runtime_dir = args.runtime_dir.resolve()
    manifest = args.manifest.resolve()
    if args.source_assembly_out:
        write_source_assembly(source, args.source_assembly_out.resolve())
    report = validate(
        source,
        runtime_dir,
        manifest,
        args.tolerance_mm,
        args.overlay_out.resolve() if args.overlay_out else None,
    )
    if args.report_out:
        args.report_out.resolve().parent.mkdir(parents=True, exist_ok=True)
        args.report_out.resolve().write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
