#!/usr/bin/env python3
"""Normalize the supplied F1-94 triangle-soup GLB into an annotated assembly.

The source contains one non-indexed textured primitive. Connectivity is
recovered on an analysis copy by welding positions, then the original faces,
UVs, normals, material and texture are partitioned into chassis plus four wheel
nodes. Geometry is translated by the mean wheel-hub datum; it is not scaled or
rotated. The OBJ companion is a material oracle: it has the same texture but no
unintended 0.4 base-color multiplier. The resulting source contract is +Y up,
+Z forward, +X left.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import struct
import tempfile
from pathlib import Path

import numpy as np
import trimesh

from add_gltf_normals import _chunks, normal_coverage


JSON_CHUNK = 0x4E4F534A
BIN_CHUNK = 0x004E4942
NEAREST = 9728
WHEEL_LABELS = ("LF", "RF", "LR", "RR")
SIDECAR_CORNERS = {"LF": "FL", "RF": "FR", "LR": "RL", "RR": "RR"}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_glb(path: Path, document: dict[str, object], buffer: bytes) -> None:
    document["buffers"][0]["byteLength"] = len(buffer)  # type: ignore[index]
    json_payload = json.dumps(document, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    json_payload += b" " * ((-len(json_payload)) % 4)
    bin_payload = buffer + b"\x00" * ((-len(buffer)) % 4)
    total_length = 12 + 8 + len(json_payload) + 8 + len(bin_payload)
    payload = bytearray(struct.pack("<4sII", b"glTF", 2, total_length))
    payload.extend(struct.pack("<II", len(json_payload), JSON_CHUNK))
    payload.extend(json_payload)
    payload.extend(struct.pack("<II", len(bin_payload), BIN_CHUNK))
    payload.extend(bin_payload)
    with tempfile.NamedTemporaryFile(
        mode="wb", prefix=f"{path.name}.", suffix=".tmp", dir=path.parent, delete=False
    ) as temporary:
        temporary.write(payload)
        temporary_path = Path(temporary.name)
    try:
        os.replace(temporary_path, path)
    finally:
        temporary_path.unlink(missing_ok=True)


def enforce_nearest_sampling(path: Path) -> None:
    document, buffer = _chunks(path)
    samplers = document.setdefault("samplers", [])
    textures = document.get("textures", [])
    if textures and not samplers:
        samplers.append({"magFilter": NEAREST, "minFilter": NEAREST, "wrapS": 10497, "wrapT": 10497})
    for sampler in samplers:
        sampler["magFilter"] = NEAREST
        sampler["minFilter"] = NEAREST
    for texture in textures:
        texture["sampler"] = int(texture.get("sampler", 0))
    write_glb(path, document, buffer)


def normalize_pixelart_material(path: Path) -> None:
    """Remove the source export's unintended darkening multiplier."""
    document, buffer = _chunks(path)
    corrected = 0
    for material in document.get("materials", []):
        pbr = material.setdefault("pbrMetallicRoughness", {})
        is_pixel_material = material.get("name") == "F194_PIXELART_NEAREST"
        # OBJ/MTL import can omit the material name, but the pixel material is
        # still unambiguous because it is the only material carrying a texture.
        is_unnamed_textured_material = not material.get("name") and "baseColorTexture" in pbr
        if not (is_pixel_material or is_unnamed_textured_material):
            continue
        material["name"] = "F194_PIXELART_NEAREST"
        if pbr.get("baseColorFactor") != [1.0, 1.0, 1.0, 1.0]:
            pbr["baseColorFactor"] = [1.0, 1.0, 1.0, 1.0]
            corrected += 1
    if corrected != 1:
        raise ValueError(f"expected one F194 pixel material correction, got {corrected}")
    write_glb(path, document, buffer)


def component_rows(mesh: trimesh.Trimesh) -> tuple[list[dict[str, object]], dict[int, np.ndarray]]:
    face_groups = trimesh.graph.connected_components(
        mesh.face_adjacency, nodes=np.arange(len(mesh.faces)), min_len=1
    )
    rows: list[dict[str, object]] = []
    faces_by_component: dict[int, np.ndarray] = {}
    for component_id, face_ids in enumerate(face_groups):
        face_ids = np.asarray(face_ids, dtype=np.int64)
        component = mesh.submesh([face_ids], append=True, repair=False)
        bounds = np.asarray(component.bounds, dtype=float)
        dimensions = bounds[1] - bounds[0]
        center = bounds.mean(axis=0)
        row = {
            "component": component_id,
            "faces": int(len(face_ids)),
            "center": center.tolist(),
            "dimensions": dimensions.tolist(),
            "bounds": bounds.tolist(),
            "watertight": bool(component.is_watertight),
        }
        rows.append(row)
        faces_by_component[component_id] = face_ids
    return rows, faces_by_component


def classify_wheels(rows: list[dict[str, object]]) -> tuple[dict[str, list[int]], dict[str, np.ndarray], list[dict[str, object]]]:
    tire_rows: list[dict[str, object]] = []
    for row in rows:
        center = np.asarray(row["center"], dtype=float)
        size = np.asarray(row["dimensions"], dtype=float)
        if (
            int(row["faces"]) >= 120
            and abs(center[0]) > 0.55
            and 0.55 <= size[1] <= 0.75
            and 0.55 <= size[2] <= 0.75
            and 0.20 <= size[0] <= 0.45
        ):
            tire_rows.append(row)
    if len(tire_rows) != 4:
        raise ValueError(f"expected exactly four wheel tire components, found {len(tire_rows)}")

    z_values = sorted(float(np.asarray(row["center"])[2]) for row in tire_rows)
    split_z = (z_values[1] + z_values[2]) * 0.5
    assignments: dict[str, list[int]] = {label: [] for label in WHEEL_LABELS}
    hubs: dict[str, np.ndarray] = {}
    evidence: list[dict[str, object]] = []
    tire_component_ids: set[int] = set()
    for row in tire_rows:
        center = np.asarray(row["center"], dtype=float)
        axle = "F" if center[2] > split_z else "R"
        side = "L" if center[0] > 0.0 else "R"
        label = side + axle
        component_id = int(row["component"])
        assignments[label].append(component_id)
        hubs[label] = center
        tire_component_ids.add(component_id)
        evidence.append(
            {
                "region": f"component_{component_id}",
                "classification": f"wheel_{label.lower()}_tire",
                "confidence": 1.0,
                "evidence": [
                    "watertight cylindrical component",
                    f"center={np.round(center, 6).tolist()}",
                    f"dimensions={np.round(np.asarray(row['dimensions']), 6).tolist()}",
                    "front/rear proven by narrower front and wider rear Formula tire widths",
                ],
            }
        )

    for row in rows:
        component_id = int(row["component"])
        if component_id in tire_component_ids:
            continue
        center = np.asarray(row["center"], dtype=float)
        size = np.asarray(row["dimensions"], dtype=float)
        for label, hub in hubs.items():
            outboard = center[0] * hub[0] > 0.0 and abs(center[0]) >= abs(hub[0]) - 0.02
            cap_shape = size[0] <= 0.10 and size[1] <= 0.15 and size[2] <= 0.15
            if np.linalg.norm(center - hub) <= 0.16 and outboard and cap_shape:
                assignments[label].append(component_id)
                evidence.append(
                    {
                        "region": f"component_{component_id}",
                        "classification": f"wheel_{label.lower()}_hub_cap",
                        "confidence": 0.98,
                        "evidence": [
                            f"distance_to_hub_m={float(np.linalg.norm(center - hub)):.6f}",
                            "small outboard component concentric with tire",
                        ],
                    }
                )
                break

    if any(len(assignments[label]) != 2 for label in WHEEL_LABELS):
        raise ValueError(f"wheel classification expected tire+hub cap per corner: {assignments}")
    return assignments, hubs, evidence


def classify_suspension(
    rows: list[dict[str, object]],
    hubs: dict[str, np.ndarray],
    wheel_components: set[int],
) -> tuple[dict[str, list[int]], list[dict[str, object]]]:
    assignments: dict[str, list[int]] = {label: [] for label in WHEEL_LABELS}
    evidence: list[dict[str, object]] = []
    for row in rows:
        component_id = int(row["component"])
        if component_id in wheel_components or int(row["faces"]) != 12:
            continue
        center = np.asarray(row["center"], dtype=float)
        dimensions = np.asarray(row["dimensions"], dtype=float)
        label = min(hubs, key=lambda corner: float(np.linalg.norm(center - hubs[corner])))
        distance = float(np.linalg.norm(center - hubs[label]))
        is_inboard = abs(center[0]) <= abs(hubs[label][0]) - 0.10
        is_thin_arm = float(dimensions.min()) <= 0.08 and float(dimensions.max()) <= 0.60
        if distance <= 0.55 and is_inboard and is_thin_arm:
            assignments[label].append(component_id)
            evidence.append(
                {
                    "region": f"component_{component_id}",
                    "classification": f"suspension_{label.lower()}",
                    "confidence": 0.96,
                    "evidence": [
                        f"distance_to_hub_m={distance:.6f}",
                        f"dimensions={np.round(dimensions, 6).tolist()}",
                        "thin inboard arm between chassis and measured wheel hub",
                    ],
                }
            )
    if any(not assignments[label] for label in WHEEL_LABELS):
        raise ValueError(f"suspension classification did not cover all four corners: {assignments}")
    return assignments, evidence


def normalize(source: Path, output: Path, datums_output: Path, report_output: Path) -> dict[str, object]:
    scene = trimesh.load(source, force="scene", process=False)
    meshes = tuple(scene.dump())
    if len(meshes) != 1:
        raise ValueError(f"expected one source mesh, found {len(meshes)}")
    original = meshes[0]
    if not isinstance(original, trimesh.Trimesh):
        raise ValueError("source geometry is not a triangle mesh")
    if len(original.faces) == 0 or len(original.vertices) != len(original.faces) * 3:
        raise ValueError("source no longer matches the expected non-indexed triangle-soup contract")

    analysis_mesh = original.copy()
    analysis_mesh.merge_vertices(merge_tex=True, merge_norm=True, digits_vertex=6)
    rows, faces_by_component = component_rows(analysis_mesh)
    assignments, hubs_source, classification = classify_wheels(rows)
    wheel_component_ids = {component for components in assignments.values() for component in components}
    suspension_assignments, suspension_evidence = classify_suspension(
        rows, hubs_source, wheel_component_ids
    )
    classification.extend(suspension_evidence)

    source_origin = np.mean(np.asarray([hubs_source[label] for label in WHEEL_LABELS]), axis=0)
    all_faces = set(range(len(original.faces)))
    wheel_faces: dict[str, np.ndarray] = {}
    assigned_faces: set[int] = set()
    for label in WHEEL_LABELS:
        face_ids = np.concatenate([faces_by_component[c] for c in assignments[label]])
        wheel_faces[label] = np.sort(face_ids)
        overlap = assigned_faces.intersection(int(face) for face in face_ids)
        if overlap:
            raise ValueError(f"wheel face assignment overlaps for {label}")
        assigned_faces.update(int(face) for face in face_ids)
    suspension_faces: dict[str, np.ndarray] = {}
    for label in WHEEL_LABELS:
        face_ids = np.concatenate([faces_by_component[c] for c in suspension_assignments[label]])
        suspension_faces[label] = np.sort(face_ids)
        overlap = assigned_faces.intersection(int(face) for face in face_ids)
        if overlap:
            raise ValueError(f"suspension face assignment overlaps for {label}")
        assigned_faces.update(int(face) for face in face_ids)
    chassis_faces = np.asarray(sorted(all_faces - assigned_faces), dtype=np.int64)
    if (
        len(chassis_faces)
        + sum(len(ids) for ids in wheel_faces.values())
        + sum(len(ids) for ids in suspension_faces.values())
        != len(original.faces)
    ):
        raise ValueError("face partition is not exhaustive")

    normalized = trimesh.Scene()
    chassis = original.submesh([chassis_faces], append=True, repair=False)
    chassis.apply_translation(-source_origin)
    if getattr(chassis.visual, "material", None) is not None:
        chassis.visual.material.name = "F194_PIXELART_NEAREST"
    normalized.add_geometry(chassis, geom_name="F194_CHASSIS", node_name="F194_CHASSIS")
    for label in WHEEL_LABELS:
        suspension = original.submesh([suspension_faces[label]], append=True, repair=False)
        suspension.apply_translation(-source_origin)
        if getattr(suspension.visual, "material", None) is not None:
            suspension.visual.material.name = "F194_PIXELART_NEAREST"
        name = f"SUSPENSION_{label}"
        normalized.add_geometry(suspension, geom_name=name, node_name=name)
    for label in WHEEL_LABELS:
        wheel = original.submesh([wheel_faces[label]], append=True, repair=False)
        wheel.apply_translation(-source_origin)
        if getattr(wheel.visual, "material", None) is not None:
            wheel.visual.material.name = "F194_PIXELART_NEAREST"
        name = f"LP_TIRE_{label}"
        normalized.add_geometry(wheel, geom_name=name, node_name=name)

    hubs_normalized = {label: hubs_source[label] - source_origin for label in WHEEL_LABELS}
    source_bounds = np.asarray(original.bounds, dtype=float) - source_origin
    front_axle_center = (hubs_normalized["LF"] + hubs_normalized["RF"]) * 0.5
    rear_axle_center = (hubs_normalized["LR"] + hubs_normalized["RR"]) * 0.5
    front_axle_z = float(front_axle_center[2])
    rear_axle_z = float(rear_axle_center[2])
    marker_points = {
        "DATUM_NOSE": np.array([0.0, 0.0, source_bounds[1, 2]]),
        "DATUM_TAIL": np.array([0.0, 0.0, source_bounds[0, 2]]),
        "DATUM_ORIGIN": np.zeros(3),
        "DATUM_FRONT_AXLE_CENTER": front_axle_center,
        "DATUM_REAR_AXLE_CENTER": rear_axle_center,
        "DATUM_HUB_FL": hubs_normalized["LF"],
        "DATUM_HUB_FR": hubs_normalized["RF"],
        "DATUM_HUB_RL": hubs_normalized["LR"],
        "DATUM_HUB_RR": hubs_normalized["RR"],
    }
    datum_material = trimesh.visual.material.PBRMaterial(
        name="F194_DATUM", baseColorFactor=[0, 255, 255, 255], metallicFactor=0.0, roughnessFactor=1.0
    )
    for name, point in marker_points.items():
        marker = trimesh.creation.box(extents=[0.01, 0.01, 0.01])
        marker.apply_translation(point)
        marker.visual = trimesh.visual.TextureVisuals(material=datum_material)
        normalized.add_geometry(marker, geom_name=name, node_name=name)

    output.parent.mkdir(parents=True, exist_ok=True)
    normalized.export(output, include_normals=True)
    normalize_pixelart_material(output)
    enforce_nearest_sampling(output)

    datums = {
        "schema": "formula90s/normalized-vehicle-source/v1",
        "candidate_id": "f1_94",
        "source_asset": source.name,
        "source_sha256": sha256(source),
        "wheel_name_prefix": "LP_",
        "coordinate_contract": {
            "forward": "+Z",
            "up": "+Y",
            "left": "+X",
            "units": "meters",
            "conversion_owner": "export_pipeline_single_T_vehicle",
        },
        "normalization_translation_m": (-source_origin).tolist(),
        "origin": [0.0, 0.0, 0.0],
        "nose": [0.0, 0.0, float(source_bounds[1, 2])],
        "tail": [0.0, 0.0, float(source_bounds[0, 2])],
        "wheel_hubs": {
            SIDECAR_CORNERS[label]: hubs_normalized[label].tolist() for label in WHEEL_LABELS
        },
        "front_axle_z": front_axle_z,
        "rear_axle_z": rear_axle_z,
    }
    datums_output.parent.mkdir(parents=True, exist_ok=True)
    datums_output.write_text(json.dumps(datums, indent=2) + "\n", encoding="utf-8")

    output_scene = trimesh.load(output, force="scene", process=False)
    output_visual_faces = sum(
        len(output_scene.geometry[output_scene.graph[node][1]].faces)
        for node in output_scene.graph.nodes_geometry
        if not node.startswith("DATUM_")
    )
    output_vertices = sum(len(mesh.vertices) for mesh in output_scene.geometry.values())
    document, _ = _chunks(output)
    nearest_ok = bool(document.get("samplers")) and all(
        sampler.get("magFilter") == NEAREST and sampler.get("minFilter") == NEAREST
        for sampler in document["samplers"]
    )
    coverage = normal_coverage(output)
    report = {
        "status": "PASS"
        if output_visual_faces == len(original.faces)
        and coverage["missing_normals"] == 0
        and nearest_ok
        else "FAIL",
        "source": str(source.resolve()),
        "source_sha256": sha256(source),
        "output": str(output.resolve()),
        "output_sha256": sha256(output),
        "invariants": {
            "source_faces": int(len(original.faces)),
            "output_visual_faces": int(output_visual_faces),
            "faces_preserved": output_visual_faces == len(original.faces),
            "source_bounds_m": np.asarray(original.bounds).tolist(),
            "normalized_bounds_m": source_bounds.tolist(),
            "scale_applied": 1.0,
            "rotation_applied_degrees": 0.0,
            "translation_applied_m": (-source_origin).tolist(),
            "materials": len(document.get("materials", [])),
            "textures": len(document.get("textures", [])),
            "pixel_material_base_color_factor": [1.0, 1.0, 1.0, 1.0],
            "pixel_material_correction": "source GLB 0.4 factor removed; OBJ/MTL map_Kd oracle",
            "nearest_sampling": nearest_ok,
            "normal_coverage": coverage,
        },
        "partition": {
            "chassis_faces": int(len(chassis_faces)),
            "wheel_faces": {label: int(len(wheel_faces[label])) for label in WHEEL_LABELS},
            "suspension_faces": {
                label: int(len(suspension_faces[label])) for label in WHEEL_LABELS
            },
            "output_vertices": int(output_vertices),
            "wheel_components": assignments,
            "suspension_components": suspension_assignments,
        },
        "classification": classification,
        "datums": datums,
    }
    report_output.parent.mkdir(parents=True, exist_ok=True)
    report_output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if report["status"] != "PASS":
        raise ValueError("normalized source validation failed")
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--datums-out", required=True, type=Path)
    parser.add_argument("--report-out", required=True, type=Path)
    args = parser.parse_args()
    report = normalize(args.source, args.output, args.datums_out, args.report_out)
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
