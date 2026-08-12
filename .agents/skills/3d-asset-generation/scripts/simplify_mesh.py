#!/usr/bin/env python3
"""Deterministically decimate a mesh with one explicit PyMeshLab filter."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
import tempfile

import numpy as np
import pymeshlab
import trimesh

from mesh_utils import analyze_scene, combined_mesh, load_scene, scene_meshes, sha256_file


FILTER_NAME = "meshing_decimation_quadric_edge_collapse"


def _world_mesh(scene: trimesh.Scene) -> tuple[trimesh.Trimesh, bool]:
    meshes = scene_meshes(scene)
    flattened = combined_mesh(meshes)
    colors = []
    can_preserve_flat_materials = bool(meshes)
    for mesh in meshes:
        material = getattr(mesh.visual, "material", None)
        factor = getattr(material, "baseColorFactor", None)
        image = getattr(material, "image", None)
        if factor is None or image is not None:
            can_preserve_flat_materials = False
            break
        color = np.asarray(factor, dtype=np.float64)
        if color.size and color.max() <= 1.0:
            color *= 255.0
        color = np.clip(np.round(color), 0, 255).astype(np.uint8)
        colors.append(np.tile(color, (len(mesh.faces), 1)))
    if can_preserve_flat_materials and colors:
        flattened.visual = trimesh.visual.ColorVisuals(
            mesh=flattened,
            face_colors=np.concatenate(colors, axis=0),
        )
    return flattened, can_preserve_flat_materials and bool(colors)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    target = parser.add_mutually_exclusive_group(required=True)
    target.add_argument("--target-faces", type=int)
    target.add_argument("--target-ratio", type=float)
    parser.add_argument("--preserve-boundary", action="store_true")
    parser.add_argument("--report", type=Path)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    source = args.input.resolve()
    output = args.output.resolve()
    if not source.is_file():
        print(f"error: input does not exist: {source}", file=sys.stderr)
        return 2
    if source == output:
        print("error: output must differ from input; originals are never overwritten", file=sys.stderr)
        return 2
    if args.target_faces is not None and args.target_faces < 4:
        print("error: --target-faces must be at least 4", file=sys.stderr)
        return 2
    if args.target_ratio is not None and not 0.0 < args.target_ratio < 1.0:
        print("error: --target-ratio must be greater than 0 and less than 1", file=sys.stderr)
        return 2

    source_analysis = analyze_scene(load_scene(source), source, sha256_file(source))
    output.parent.mkdir(parents=True, exist_ok=True)
    glb_output = output.suffix.lower() in {".glb", ".gltf"}
    normalize_scene = source.suffix.lower() in {".glb", ".gltf"}
    flat_materials_preserved = False
    warnings = []
    with tempfile.TemporaryDirectory(prefix="pymeshlab-", dir=output.parent) as temp_dir:
        pymeshlab_input = source
        if normalize_scene:
            # Flatten scene-node transforms before PyMeshLab, whose GLB loader
            # merges instances without reliably preserving those transforms.
            pymeshlab_input = Path(temp_dir) / "source_world.ply"
            world_mesh, flat_materials_preserved = _world_mesh(load_scene(source))
            world_mesh.export(pymeshlab_input)

        mesh_set = pymeshlab.MeshSet()
        mesh_set.load_new_mesh(str(pymeshlab_input))
        before_mesh = mesh_set.current_mesh()
        faces_before = int(before_mesh.face_number())
        vertices_before = int(before_mesh.vertex_number())
        target_faces = args.target_faces
        if target_faces is None:
            target_faces = max(4, int(faces_before * args.target_ratio))
        target_faces = min(target_faces, faces_before)

        applied = False
        if faces_before and target_faces < faces_before:
            mesh_set.meshing_decimation_quadric_edge_collapse(
                targetfacenum=target_faces,
                targetperc=0.0,
                qualitythr=0.3,
                preserveboundary=args.preserve_boundary,
                boundaryweight=1.0,
                preservenormal=False,
                preservetopology=False,
                optimalplacement=True,
                planarquadric=False,
                planarweight=0.001,
                qualityweight=False,
                autoclean=True,
                selected=False,
            )
            applied = True
        if mesh_set.current_mesh().face_number():
            mesh_set.compute_normal_per_vertex()

        export_bridge = None
        if glb_output:
            # PyMeshLab performs the mesh operation; Trimesh bridges its mesh-only
            # output to the requested runtime container format.
            export_bridge = Path(temp_dir) / "decimated.ply"
            mesh_set.save_current_mesh(str(export_bridge))
            combined_mesh(scene_meshes(load_scene(export_bridge))).export(output)
        else:
            mesh_set.save_current_mesh(str(output))
        after_mesh = mesh_set.current_mesh()
        vertices_after = int(after_mesh.vertex_number())
        faces_after = int(after_mesh.face_number())

    if normalize_scene and not flat_materials_preserved and (
        source_analysis["has_uv"] or source_analysis["materials"]
    ):
        warnings.append(
            "complex materials or UV data were not preserved through the PLY bridge; "
            "finish this asset in Blender headless"
        )

    report = {
        "input": str(source),
        "output": str(output),
        "input_sha256": sha256_file(source),
        "filter": FILTER_NAME,
        "parameters": {
            "target_faces": target_faces,
            "target_ratio": args.target_ratio,
            "preserve_boundary": args.preserve_boundary,
            "autoclean": True,
            "optimalplacement": True,
            "glb_bridge": "Trimesh export from temporary PLY" if glb_output else None,
            "flat_materials_to_vertex_colors": flat_materials_preserved,
        },
        "applied": applied,
        "warnings": warnings,
        "vertices_before": vertices_before,
        "vertices_after": vertices_after,
        "faces_before": faces_before,
        "faces_after": faces_after,
        "input_analysis": source_analysis,
        "output_analysis": analyze_scene(load_scene(output), output, sha256_file(output)),
    }
    report_path = (args.report or output.with_suffix(output.suffix + ".report.json")).resolve()
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
