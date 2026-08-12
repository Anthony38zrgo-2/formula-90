"""Deterministically add flat, region-based materials to the active Jordan K3 GLBs.

Input files are never overwritten. Output is written under assets-lowpoly-python first;
the runtime replacement is a separate, explicit publish step.
"""
from __future__ import annotations

import argparse
import json
import shutil
from collections import Counter
from pathlib import Path

import numpy as np
import trimesh
from trimesh.visual.material import PBRMaterial


PALETTE = {
    "body_gold_base": {"rgba": [197, 154, 74, 255], "metallic": 0.0, "roughness": 0.84},
    "body_gold_light": {"rgba": [244, 204, 120, 255], "metallic": 0.0, "roughness": 0.82},
    "body_gold_shadow": {"rgba": [139, 107, 61, 255], "metallic": 0.0, "roughness": 0.88},
    "accent_red": {"rgba": [217, 39, 30, 255], "metallic": 0.0, "roughness": 0.86},
    "tire_black": {"rgba": [23, 25, 29, 255], "metallic": 0.0, "roughness": 0.94},
    "rim_gold": {"rgba": [185, 134, 5, 255], "metallic": 0.12, "roughness": 0.48},
}


def load_meshes_with_transforms(path: Path) -> list[tuple[str, trimesh.Trimesh, np.ndarray]]:
    """Return every local mesh and its source scene transform.

    The candidate GLBs store Godot's axis conversion on the root node. Splitting
    materials must preserve that transform rather than baking it into vertices.
    """
    loaded = trimesh.load(path, force="scene", process=False)
    result = []
    for node in loaded.graph.nodes_geometry:
        transform, geometry_name = loaded.graph.get(node)
        mesh = loaded.geometry[geometry_name].copy()
        if isinstance(mesh, trimesh.Trimesh) and len(mesh.faces):
            result.append((node, mesh, transform))
    if not result:
        raise RuntimeError(f"No mesh geometry found in {path}")
    return result


def chassis_regions(mesh: trimesh.Trimesh) -> np.ndarray:
    c = mesh.triangles_center
    n = mesh.face_normals
    names = np.full(len(mesh.faces), "body_gold_base", dtype=object)
    # The source GLB consumed by Godot uses X=lateral, Y=vertical, Z=longitudinal.
    # Faceted lighting language used by the supplied PSX reference.
    names[n[:, 1] > 0.35] = "body_gold_light"
    names[n[:, 1] < -0.30] = "body_gold_shadow"
    # Three visibly red, centreline accents, selected spatially rather than by IDs.
    front_inset = (c[:, 2] < -1.82) & (c[:, 1] > -0.58) & (c[:, 1] < -0.30) & (np.abs(c[:, 0]) < 0.68)
    airbox_cap = (c[:, 2] > 0.32) & (c[:, 2] < 0.88) & (c[:, 1] > 0.02) & (np.abs(c[:, 0]) < 0.28)
    diffuser_tip = (c[:, 2] > 1.58) & (c[:, 1] < -0.30) & (np.abs(c[:, 0]) < 0.27)
    names[front_inset | airbox_cap | diffuser_tip] = "accent_red"
    return names


def wheel_regions(mesh: trimesh.Trimesh) -> np.ndarray:
    c = mesh.triangles_center
    bounds = mesh.bounds
    yz_center = (bounds[0, 1:] + bounds[1, 1:]) * 0.5
    radius = np.linalg.norm(c[:, 1:] - yz_center, axis=1)
    # A rim is the central, recessed portion of a wheel; the outer ring remains rubber.
    names = np.full(len(mesh.faces), "tire_black", dtype=object)
    names[radius < 0.225] = "rim_gold"
    return names


def add_split_regions(scene: trimesh.Scene, mesh: trimesh.Trimesh, names: np.ndarray, transform: np.ndarray, prefix: str) -> None:
    for material_name in sorted(set(names.tolist())):
        face_indices = np.flatnonzero(names == material_name)
        if not len(face_indices):
            continue
        submesh = mesh.submesh([face_indices], append=True, repair=False)
        spec = PALETTE[material_name]
        submesh.visual.material = PBRMaterial(
            name=material_name,
            baseColorFactor=spec["rgba"],
            metallicFactor=spec["metallic"],
            roughnessFactor=spec["roughness"],
        )
        unique = f"{prefix}_{material_name}"
        scene.add_geometry(submesh, node_name=unique, geom_name=unique, transform=transform)


def colorize(source: Path, output: Path, kind: str) -> dict:
    scene = trimesh.Scene()
    all_names = []
    vertices = 0
    faces = 0
    for node, mesh, transform in load_meshes_with_transforms(source):
        world_mesh = mesh.copy()
        world_mesh.apply_transform(transform)
        regions = chassis_regions(world_mesh) if kind == "chassis" else wheel_regions(world_mesh)
        add_split_regions(scene, mesh, regions, transform, node)
        all_names.extend(regions.tolist())
        vertices += len(mesh.vertices)
        faces += len(mesh.faces)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(scene.export(file_type="glb"))
    return {
        "source": str(source),
        "output": str(output),
        "kind": kind,
        "vertices_before": int(vertices),
        "faces_before": int(faces),
        "faces_by_material": dict(Counter(all_names)),
        "materials": sorted(set(all_names)),
        "classification": "geometry position + face normal; no object names or vertex IDs used",
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, required=True)
    args = parser.parse_args()
    root = args.repo.resolve()
    runtime = root / "game/assets/models/vehicles/f1_90s_canonical_1997/candidate_k3_historical"
    staging = root / "assets-lowpoly-python/vehicles/f1_psx/active_jordan_k3_colorized"
    input_dir = staging / "input"
    output_dir = staging / "output"
    input_dir.mkdir(parents=True, exist_ok=True)

    specs = [
        ("jordan_191_candidate_chassis.glb", "chassis"),
        ("jordan_191_candidate_wheel_front.glb", "wheel"),
        ("jordan_191_candidate_wheel_rear.glb", "wheel"),
    ]
    report = {"pipeline": "flat_region_materials_v1", "assets": []}
    for filename, kind in specs:
        source = runtime / filename
        staged_input = input_dir / filename
        shutil.copy2(source, staged_input)
        report["assets"].append(colorize(staged_input, output_dir / filename, kind))
    (staging / "colorization_report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
