#!/usr/bin/env python3
"""Materialize the repaired F1-94 bundle under the Formula-90 import standard."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import struct
import tempfile
from pathlib import Path

import numpy as np
import trimesh
from scipy.spatial import cKDTree


REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_BUNDLE = REPO_ROOT / "assets-lowpoly-python/vehicles/F194_GEVP_READY"
DEFAULT_CANONICAL = REPO_ROOT / "assets-lowpoly-python/vehicles/canonical/f1_94_gevp_ready.glb"
DEFAULT_OUTPUT = REPO_ROOT / "game/assets/models/vehicles/f1_94"
SOURCE_FILES = {
    "vehicle_metadata.json": "vehicle_metadata.json",
}
SOURCE_TEXTURE = ("textures/F1_94_texture.png", "textures/F1_94_texture.png")
WHEEL_SOURCES = {
    "FL": ("F1_94_wheel_front.glb", "f1_94_wheel_fl.glb"),
    "FR": ("F1_94_wheel_front.glb", "f1_94_wheel_fr.glb"),
    "RL": ("F1_94_wheel_rear.glb", "f1_94_wheel_rl.glb"),
    "RR": ("F1_94_wheel_rear.glb", "f1_94_wheel_rr.glb"),
}
RETIRED_FILES = (
    "f1_94_wheel_front.glb",
    "f1_94_wheel_rear.glb",
    "f1_94_wheel_front.glb.import",
    "f1_94_wheel_rear.glb.import",
    "f1_94_wheel_front_0.png",
    "f1_94_wheel_rear_0.png",
    "f1_94_wheel_front_0.png.import",
    "f1_94_wheel_rear_0.png.import",
    "f1_94_gevp_metadata.json",
    "source_assembly.json",
    "assembly_equivalence_report.json",
    "assembly_equivalence_overlay.glb",
    "assembly_equivalence_overlay.glb.import",
)
NEAREST = 9728
REQUIRED_GEOMETRY = {
    "GEO_CHASSIS",
    "GEO_AERO_FRONT_WING",
    "GEO_AERO_REAR_WING",
    "GEO_SUSPENSION_FL",
    "GEO_SUSPENSION_FR",
    "GEO_SUSPENSION_RL",
    "GEO_SUSPENSION_RR",
}
REQUIRED_DATUMS = {
    "DATUM_VEHICLE_ORIGIN",
    "DATUM_FRONT_AXLE_CENTER",
    "DATUM_REAR_AXLE_CENTER",
    "JNT_WHEEL_FL",
    "JNT_WHEEL_FR",
    "JNT_WHEEL_RL",
    "JNT_WHEEL_RR",
    "JNT_FRONT_WING_MOUNT",
    "JNT_REAR_WING_MOUNT",
    "JNT_SUSP_FL_CHASSIS",
    "JNT_SUSP_FL_HUB",
    "JNT_SUSP_FR_CHASSIS",
    "JNT_SUSP_FR_HUB",
    "JNT_SUSP_RL_CHASSIS",
    "JNT_SUSP_RL_HUB",
    "JNT_SUSP_RR_CHASSIS",
    "JNT_SUSP_RR_HUB",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_glb_document(path: Path) -> dict[str, object]:
    payload = path.read_bytes()
    if payload[:4] != b"glTF":
        raise ValueError(f"not a GLB: {path}")
    chunk_length, chunk_type = struct.unpack_from("<II", payload, 12)
    if chunk_type != 0x4E4F534A:
        raise ValueError(f"GLB JSON chunk missing: {path}")
    return json.loads(payload[20 : 20 + chunk_length])


def write_glb_document(path: Path, document: dict[str, object]) -> None:
    payload = path.read_bytes()
    offset = 12
    json_length, json_type = struct.unpack_from("<II", payload, offset)
    offset += 8
    if json_type != 0x4E4F534A:
        raise ValueError(f"GLB JSON chunk missing: {path}")
    offset += json_length
    bin_length, bin_type = struct.unpack_from("<II", payload, offset)
    offset += 8
    if bin_type != 0x004E4942:
        raise ValueError(f"GLB BIN chunk missing: {path}")
    binary = payload[offset : offset + bin_length]
    encoded = json.dumps(document, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    encoded += b" " * ((-len(encoded)) % 4)
    binary += b"\x00" * ((-len(binary)) % 4)
    total = 12 + 8 + len(encoded) + 8 + len(binary)
    result = bytearray(struct.pack("<4sII", b"glTF", 2, total))
    result.extend(struct.pack("<II", len(encoded), 0x4E4F534A))
    result.extend(encoded)
    result.extend(struct.pack("<II", len(binary), 0x004E4942))
    result.extend(binary)
    path.write_bytes(result)


def enforce_nearest_sampling(path: Path) -> None:
    document = read_glb_document(path)
    samplers = document.setdefault("samplers", [])
    textures = document.get("textures", [])
    if textures and not samplers:
        samplers.append({"magFilter": NEAREST, "minFilter": NEAREST, "wrapS": 10497, "wrapT": 10497})
    for sampler in samplers:
        sampler["magFilter"] = NEAREST
        sampler["minFilter"] = NEAREST
    for texture in textures:
        texture["sampler"] = 0
    write_glb_document(path, document)


def world_meshes(path: Path) -> dict[str, trimesh.Trimesh]:
    scene = trimesh.load(path, force="scene", process=False)
    result: dict[str, trimesh.Trimesh] = {}
    for node in scene.graph.nodes_geometry:
        transform, geometry_name = scene.graph[node]
        mesh = scene.geometry[geometry_name].copy()
        mesh.apply_transform(transform)
        result[node] = mesh
    return result


def scene_nodes(path: Path) -> set[str]:
    scene = trimesh.load(path, force="scene", process=False)
    return {str(node) for node in scene.graph.nodes}


def vertices(meshes: dict[str, trimesh.Trimesh], predicate=lambda _name: True) -> np.ndarray:
    selected = [mesh.vertices for name, mesh in meshes.items() if predicate(name)]
    if not selected:
        raise ValueError("geometry selection is empty")
    return np.vstack(selected)


def bidirectional_max_error(expected: np.ndarray, actual: np.ndarray) -> float:
    expected_tree = cKDTree(expected)
    actual_tree = cKDTree(actual)
    return float(
        max(
            expected_tree.query(actual, workers=-1)[0].max(),
            actual_tree.query(expected, workers=-1)[0].max(),
        )
    )


def mesh_quality(meshes: dict[str, trimesh.Trimesh]) -> dict[str, object]:
    rows: dict[str, object] = {}
    total_degenerate = 0
    total_duplicate_faces = 0
    total_duplicate_vertices = 0
    uv_min = [float("inf"), float("inf")]
    uv_max = [float("-inf"), float("-inf")]
    for name, mesh in meshes.items():
        if not np.isfinite(mesh.vertices).all() or not np.isfinite(mesh.faces).all():
            raise ValueError(f"non-finite geometry in {name}")
        degenerate = int(np.count_nonzero(mesh.area_faces <= 1e-12))
        faces = np.sort(np.asarray(mesh.faces, dtype=np.int64), axis=1)
        duplicate_faces = int(len(faces) - len(np.unique(faces, axis=0)))
        duplicate_vertices = int(len(mesh.vertices) - len(np.unique(np.round(mesh.vertices, 8), axis=0)))
        if degenerate or duplicate_faces:
            raise ValueError(f"invalid faces in {name}: degenerate={degenerate}, duplicate={duplicate_faces}")
        uv = getattr(mesh.visual, "uv", None)
        uv_bounds = None
        if uv is not None and len(uv):
            uv = np.asarray(uv, dtype=float)
            uv_bounds = [uv.min(axis=0).tolist(), uv.max(axis=0).tolist()]
            uv_min = np.minimum(uv_min, uv.min(axis=0))
            uv_max = np.maximum(uv_max, uv.max(axis=0))
            if uv.min() < -1e-6 or uv.max() > 1.0 + 1e-6:
                raise ValueError(f"UV outside [0,1] in {name}: {uv_bounds}")
        if not np.isfinite(mesh.face_normals).all():
            raise ValueError(f"invalid normals in {name}")
        rows[name] = {
            "vertices": int(len(mesh.vertices)),
            "faces": int(len(mesh.faces)),
            "degenerate_faces": degenerate,
            "duplicate_faces": duplicate_faces,
            "duplicate_vertices": duplicate_vertices,
            "uv_bounds": uv_bounds,
        }
        total_degenerate += degenerate
        total_duplicate_faces += duplicate_faces
        total_duplicate_vertices += duplicate_vertices
    return {
        "meshes": rows,
        "degenerate_faces": total_degenerate,
        "duplicate_faces": total_duplicate_faces,
        "duplicate_vertices": total_duplicate_vertices,
        "uv_bounds": [uv_min.tolist(), uv_max.tolist()] if np.isfinite(uv_min).all() else None,
    }


def validate_material_contract(paths: list[Path]) -> None:
    for path in paths:
        document = read_glb_document(path)
        for material in document.get("materials", []):
            if material.get("doubleSided", False):
                raise ValueError(f"Cull Disabled/double-sided material in {path.name}")
            if material.get("alphaMode") not in (None, "MASK"):
                raise ValueError(f"unsupported alpha mode in {path.name}")
        samplers = document.get("samplers", [])
        textures = document.get("textures", [])
        if textures and not samplers:
            raise ValueError(f"textured asset lacks explicit sampler: {path.name}")
        if any(sampler.get("magFilter") != NEAREST or sampler.get("minFilter") != NEAREST for sampler in samplers):
            raise ValueError(f"non-nearest texture sampler in {path.name}")


def validate_source_contract(canonical: Path) -> tuple[dict[str, trimesh.Trimesh], dict[str, object]]:
    full = world_meshes(canonical)
    missing_geometry = sorted(name for name in REQUIRED_GEOMETRY if name not in full)
    wheel_names = [name for name in full if name.startswith("GEO_WHEEL_")]
    if missing_geometry or not all(any(name.startswith(f"GEO_WHEEL_{corner}_") for name in wheel_names) for corner in ("FL", "FR", "RL", "RR")):
        raise ValueError(f"canonical geometry names do not satisfy import standard: {missing_geometry}")
    missing_datums = sorted(REQUIRED_DATUMS - scene_nodes(canonical))
    if missing_datums:
        raise ValueError(f"canonical datums missing: {missing_datums}")
    quality = mesh_quality(full)
    validate_material_contract([canonical])
    return full, quality


def validate_bundle(bundle: Path, canonical: Path, tolerance_mm: float) -> dict[str, object]:
    if sha256(bundle / "F1_94_GEVP_READY.glb") != sha256(canonical):
        raise ValueError("canonical F1-94 GLB does not match the supplied ready bundle")
    metadata = json.loads((bundle / "F1_94_gevp_metadata.json").read_text(encoding="utf-8"))
    full, source_quality = validate_source_contract(canonical)
    chassis = world_meshes(bundle / "F1_94_chassis.glb")
    front = world_meshes(bundle / "F1_94_wheel_front.glb")
    rear = world_meshes(bundle / "F1_94_wheel_rear.glb")
    part_quality = {
        "chassis": mesh_quality(chassis),
        "wheel_front": mesh_quality(front),
        "wheel_rear": mesh_quality(rear),
    }
    validate_material_contract([bundle / "F1_94_chassis.glb", bundle / "F1_94_wheel_front.glb", bundle / "F1_94_wheel_rear.glb"])
    wheel_centers = metadata["wheel_centers_m"]
    errors = {"chassis": bidirectional_max_error(vertices(full, lambda name: not name.startswith("GEO_WHEEL_")), vertices(chassis))}
    for corner in ("FL", "FR"):
        expected = vertices(full, lambda name, c=corner: name.startswith(f"GEO_WHEEL_{c}_"))
        actual = vertices(front)
        if corner == "FR":
            actual = actual @ np.diag([-1.0, 1.0, -1.0]).T
        actual += np.asarray(wheel_centers[corner], dtype=float)
        errors[f"wheel_{corner.lower()}"] = bidirectional_max_error(expected, actual)
    for corner in ("RL", "RR"):
        expected = vertices(full, lambda name, c=corner: name.startswith(f"GEO_WHEEL_{c}_"))
        actual = vertices(rear)
        if corner == "RR":
            actual = actual @ np.diag([-1.0, 1.0, -1.0]).T
        actual += np.asarray(wheel_centers[corner], dtype=float)
        errors[f"wheel_{corner.lower()}"] = bidirectional_max_error(expected, actual)
    max_error_mm = max(errors.values()) * 1000.0
    if max_error_mm > tolerance_mm:
        raise ValueError(f"source split differs from canonical source by {max_error_mm:.6f} mm")
    source_faces = sum(len(mesh.faces) for mesh in full.values())
    split_faces = sum(len(mesh.faces) for mesh in chassis.values()) + 2 * sum(len(mesh.faces) for mesh in front.values()) + 2 * sum(len(mesh.faces) for mesh in rear.values())
    if split_faces != source_faces:
        raise ValueError(f"source face partition mismatch: canonical={source_faces} split={split_faces}")
    return {
        "status": "PASS",
        "tolerance_mm": tolerance_mm,
        "max_bidirectional_vertex_error_mm": max_error_mm,
        "per_part_error_mm": {key: value * 1000.0 for key, value in errors.items()},
        "faces": {
            "canonical": source_faces,
            "chassis": sum(len(mesh.faces) for mesh in chassis.values()),
            "wheel_front": sum(len(mesh.faces) for mesh in front.values()),
            "wheel_rear": sum(len(mesh.faces) for mesh in rear.values()),
            "split_instanced_total": split_faces,
        },
        "source_quality": source_quality,
        "part_quality": part_quality,
        "right_side_transform_determinant": 1.0,
    }


def write_standard_wheel(source_path: Path, corner: str, output_path: Path) -> None:
    source = trimesh.load(source_path, force="scene", process=False)
    result = trimesh.Scene(base_frame=f"F1_94_WHEEL_{corner}")
    right = corner in {"FR", "RR"}
    rotation = np.eye(4)
    rotation[:3, :3] = np.diag([-1.0, 1.0, -1.0])
    for node in source.graph.nodes_geometry:
        transform, geometry_name = source.graph[node]
        mesh = source.geometry[geometry_name].copy()
        mesh.apply_transform(transform)
        if right:
            mesh.apply_transform(rotation)
        mesh.merge_vertices(merge_tex=True, merge_norm=True, digits_vertex=8)
        suffix = geometry_name.removeprefix("GEO_WHEEL_")
        name = f"GEO_WHEEL_{corner}_{suffix}"
        result.add_geometry(mesh, geom_name=name, node_name=name)
    result.export(output_path, include_normals=True)
    enforce_nearest_sampling(output_path)


def write_clean_scene(source_path: Path, output_path: Path) -> None:
    source = trimesh.load(source_path, force="scene", process=False)
    result = trimesh.Scene(base_frame=source.graph.base_frame)
    for node in source.graph.nodes_geometry:
        transform, geometry_name = source.graph[node]
        mesh = source.geometry[geometry_name].copy()
        mesh.apply_transform(transform)
        mesh.merge_vertices(merge_tex=True, merge_norm=True, digits_vertex=8)
        result.add_geometry(mesh, geom_name=node, node_name=node)
    result.export(output_path, include_normals=True)
    enforce_nearest_sampling(output_path)


def validate_standard_wheels(canonical: Path, staging: Path, metadata: dict[str, object], tolerance_mm: float) -> dict[str, object]:
    full = world_meshes(canonical)
    wheel_centers = metadata["wheel_centers_m"]
    errors: dict[str, float] = {}
    quality: dict[str, object] = {}
    for corner, (_, filename) in WHEEL_SOURCES.items():
        path = staging / filename
        actual_meshes = world_meshes(path)
        expected = vertices(full, lambda name, c=corner: name.startswith(f"GEO_WHEEL_{c}_"))
        actual = vertices(actual_meshes) + np.asarray(wheel_centers[corner], dtype=float)
        errors[corner] = bidirectional_max_error(expected, actual)
        bounds = np.vstack([mesh.bounds for mesh in actual_meshes.values()])
        local_center = (bounds.min(axis=0) + bounds.max(axis=0)) * 0.5
        if not np.allclose(local_center, np.zeros(3), atol=1e-6):
            raise ValueError(f"wheel {corner} origin is not its rotation center: {local_center}")
        quality[corner] = mesh_quality(actual_meshes)
        expected_names = {f"GEO_WHEEL_{corner}_HUB", f"GEO_WHEEL_{corner}_TIRE_INNER", f"GEO_WHEEL_{corner}_TIRE_OUTER", f"GEO_WHEEL_{corner}_TREAD"}
        if set(actual_meshes) != expected_names:
            raise ValueError(f"wheel {corner} geometry names mismatch: {sorted(actual_meshes)}")
    validate_material_contract([staging / filename for _, filename in WHEEL_SOURCES.values()])
    max_error_mm = max(errors.values()) * 1000.0
    if max_error_mm > tolerance_mm:
        raise ValueError(f"standard wheel assets differ from canonical source by {max_error_mm:.6f} mm")
    return {
        "status": "PASS",
        "tolerance_mm": tolerance_mm,
        "max_bidirectional_vertex_error_mm": max_error_mm,
        "per_corner_error_mm": {corner: error * 1000.0 for corner, error in errors.items()},
        "quality": quality,
        "right_side_yaw_baked_degrees": 180.0,
        "right_side_transform_determinant": 1.0,
    }


def build_vehicle_metadata(bundle: Path, canonical: Path) -> dict[str, object]:
    source = json.loads((bundle / "F1_94_gevp_metadata.json").read_text(encoding="utf-8"))
    full = world_meshes(canonical)
    return {
        "schema": "formula90s/vehicle-metadata/v1",
        "vehicle_id": "f1_94",
        "format": "glb",
        "source_vehicle_glb": "assets-lowpoly-python/vehicles/canonical/f1_94_gevp_ready.glb",
        "source_sha256": sha256(canonical),
        "coordinate_contract": {
            "right": "+X",
            "left": "-X",
            "up": "+Y",
            "front": "-Z",
            "rear": "+Z",
            "units": "meters",
            "origin": "centered longitudinally between axle centers at mean wheel-center height",
            "transformations_applied": True,
            "conversion_owner": "canonical_source_bundle",
        },
        "dimensions_m": source["dimensions_m"],
        "validation_datums": {
            name: source["joints_and_datums_m"][name]
            for name in sorted(REQUIRED_DATUMS)
        },
        "geometry_names": sorted(full),
        "wheel_assets": {
            corner: f"f1_94_wheel_{corner.lower()}.glb" for corner in ("FL", "FR", "RL", "RR")
        },
        "hierarchy": {
            "wheel_node": "WheelFrontLeft/FrontLeftWheel/Visual",
            "wheel_node_pattern": "Wheel{Corner}/{Corner}Wheel/Visual",
            "raycast_owns_position": True,
            "visual_offsets": False,
        },
        "materials": {
            "alpha_mode": "MASK",
            "double_sided": False,
            "texture_filter": "NEAREST",
            "texture_path": "textures/F1_94_texture.png",
        },
        "source_node_names": sorted(scene_nodes(canonical)),
    }


def build_manifest(bundle: Path, canonical: Path, output_dir: Path, source_validation: dict[str, object], wheel_validation: dict[str, object]) -> dict[str, object]:
    metadata = json.loads((bundle / "F1_94_gevp_metadata.json").read_text(encoding="utf-8"))
    standard = json.loads((bundle / "vehicle_metadata.json").read_text(encoding="utf-8"))
    return {
        "schema_version": 4,
        "schema": "formula90s/vehicle-import/v1",
        "vehicle_id": "f1_94",
        "source": {
            "vehicle_glb": "assets-lowpoly-python/vehicles/canonical/f1_94_gevp_ready.glb",
            "sha256": sha256(canonical),
            "metadata": "assets-lowpoly-python/vehicles/F194_GEVP_READY/vehicle_metadata.json",
            "repair_bundle": "assets-lowpoly-python/vehicles/F194_GEVP_READY",
        },
        "coordinate_contract": standard["coordinate_contract"],
        "validation_datums": standard["validation_datums"],
        "geometry_names": standard["geometry_names"],
        "runtime": {
            "forward": "-Z",
            "up": "+Y",
            "right": "+X",
            "units": "meters",
            "conversion_owner": "canonical_source_bundle",
            "source_to_runtime_rotation_3x3": [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            "translation_after_rotation": [0.0, 0.0, 0.0],
        },
        "wheel_hubs_runtime_m": metadata["wheel_centers_m"],
        "wheelbase_m": metadata["dimensions_m"]["wheelbase"],
        "front_track_m": metadata["dimensions_m"]["front_track_center"],
        "rear_track_m": metadata["dimensions_m"]["rear_track_center"],
        "front_tire": {
            "radius_m": metadata["dimensions_m"]["front_tire_radius_visual"],
            "width_m": metadata["dimensions_m"]["front_tire_width_visual"],
        },
        "rear_tire": {
            "radius_m": metadata["dimensions_m"]["rear_tire_radius_visual"],
            "width_m": metadata["dimensions_m"]["rear_tire_width_visual"],
        },
        "assets": {
            "chassis": "res://assets/models/vehicles/f1_94/f1_94_chassis.glb",
            "wheel_fl": "res://assets/models/vehicles/f1_94/f1_94_wheel_fl.glb",
            "wheel_fr": "res://assets/models/vehicles/f1_94/f1_94_wheel_fr.glb",
            "wheel_rl": "res://assets/models/vehicles/f1_94/f1_94_wheel_rl.glb",
            "wheel_rr": "res://assets/models/vehicles/f1_94/f1_94_wheel_rr.glb",
            "metadata": "res://assets/models/vehicles/f1_94/vehicle_metadata.json",
        },
        "hierarchy": {
            "wheel_node": "Wheel{Corner}/{Corner}Wheel",
            "visual": "Wheel{Corner}/{Corner}Wheel/Visual",
            "right_side_yaw_baked": True,
            "negative_scale_used": False,
        },
        "export_validation": {
            filename: {"sha256": sha256(output_dir / filename)}
            for filename in ("f1_94_chassis.glb", "f1_94_wheel_fl.glb", "f1_94_wheel_fr.glb", "f1_94_wheel_rl.glb", "f1_94_wheel_rr.glb")
        },
        "source_split_equivalence": source_validation,
        "standard_asset_validation": wheel_validation,
    }


def materialize(bundle: Path, canonical: Path, output_dir: Path, tolerance_mm: float) -> dict[str, object]:
    required = ["F1_94_GEVP_READY.glb", "F1_94_gevp_metadata.json", "F1_94_chassis.glb", "F1_94_wheel_front.glb", "F1_94_wheel_rear.glb", "vehicle_metadata.json"]
    for name in required:
        if not (bundle / name).is_file():
            raise FileNotFoundError(bundle / name)
    if not canonical.is_file():
        raise FileNotFoundError(canonical)
    source_validation = validate_bundle(bundle, canonical, tolerance_mm)
    metadata = json.loads((bundle / "F1_94_gevp_metadata.json").read_text(encoding="utf-8"))
    output_dir.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="f1_94_runtime_", dir=output_dir.parent) as temporary:
        staging = Path(temporary)
        for source_name, output_name in SOURCE_FILES.items():
            shutil.copy2(bundle / source_name, staging / output_name)
        texture_source, texture_output = SOURCE_TEXTURE
        (staging / Path(texture_output).parent).mkdir(parents=True, exist_ok=True)
        shutil.copy2(bundle / texture_source, staging / texture_output)
        write_clean_scene(bundle / "F1_94_chassis.glb", staging / "f1_94_chassis.glb")
        for corner, (source_name, output_name) in WHEEL_SOURCES.items():
            write_standard_wheel(bundle / source_name, corner, staging / output_name)
        wheel_validation = validate_standard_wheels(canonical, staging, metadata, tolerance_mm)
        for path in staging.glob("*.glb"):
            enforce_nearest_sampling(path)
        for path in staging.iterdir():
            destination = output_dir / path.name
            if path.is_dir():
                shutil.copytree(path, destination, dirs_exist_ok=True)
            else:
                os.replace(path, destination)
    for retired_name in RETIRED_FILES:
        (output_dir / retired_name).unlink(missing_ok=True)
    manifest = build_manifest(bundle, canonical, output_dir, source_validation, wheel_validation)
    (output_dir / "vehicle_runtime_manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle", type=Path, default=DEFAULT_BUNDLE)
    parser.add_argument("--canonical", type=Path, default=DEFAULT_CANONICAL)
    parser.add_argument("--output-dir", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--assembly-tolerance-mm", type=float, default=0.002)
    args = parser.parse_args()
    manifest = materialize(args.bundle, args.canonical, args.output_dir, args.assembly_tolerance_mm)
    print(json.dumps(manifest, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
