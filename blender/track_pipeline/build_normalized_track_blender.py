"""Deterministic Blender compiler for a normalized F90 Track JSON contract.

Consumes ``track.normalized.json`` (as produced by ``svg_normalizer.normalize``)
plus the resolved Asset Registry and emits, beneath the requested output dir:

* ``track.blend`` — inspection/interchange artifact (atomic save);
* ``track_runtime_environment.glb`` — physical terrain/road/collision runtime;
* ``track_runtime_vegetation.glb`` — collision-free cards/vegetation runtime;
* ``build.manifest.json`` — provenance hashes + stats.

It deliberately reuses the contract-preserving functions from
``build_track_blender.py`` (continuous terrain heightfield, +Z winding, exact
roadside collision bridge, safety floor) and ``terrain_grid.py`` so every
mandatory collision invariant is produced by the exact same code path as the
legacy La Chutana pipeline.

Run inside Blender headless:

    blender --background --factory-startup --python build_normalized_track_blender.py -- \\
        --normalized track.normalized.json --registry configs/asset_registry.json \\
        --output-dir builds/<build_sha>
"""

from __future__ import annotations

import argparse
from pathlib import Path
import hashlib
import json
import math
import sys

import bpy
from mathutils import Matrix, Vector

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from asset_registry import PROCEDURAL_KINDS, load_registry, validate_registry
from build_normalized_common import (
    BLEND_FILE,
    BUILD_MANIFEST_FILE,
    BUILD_MANIFEST_SCHEMA_VERSION,
    COMPILER_NAME,
    COMPILER_VERSION,
    ENVIRONMENT_GLB,
    VEGETATION_GLB,
    build_sha256,
    compiler_sha256,
    validate_runtime_split,
)
from build_track_blender import (
    build_ribbon,
    build_roadside_collision,
    build_roadside_shoulder,
    build_safety_floor,
    build_terrain,
    clear_scene,
    godot_xz_to_blender,
)
from blender_output import atomic_export_glb, atomic_save_blend
from compile_svg_track import (
    CANONICAL_ARTIFACT,
    MANIFEST_ARTIFACT,
    NORMALIZED_ARTIFACT,
)
from procedural_materials_blender import flat_material
from svg_normalizer import DEFAULT_TERRAIN

_REPO_ROOT = Path(__file__).resolve().parents[2]


def args_after_double_dash() -> list[str]:
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def read_json(path: str | Path) -> dict:
    return json.loads(Path(path).read_text(encoding="utf-8"))


def _sha256_file(path: str | Path) -> str:
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def coerce_track_config(normalized: dict) -> dict:
    """Return the terrain-grid compatible config for this normalized track."""
    config = normalized.get("track_config")
    if isinstance(config, dict):
        return config
    return {
        "track_id": normalized.get("track_id", "untitled"),
        "sample_spacing_m": normalized.get("centerline", {}).get("sample_spacing_m", 2.0),
        "road": dict(normalized.get("road", {"width_m": 12.0, "surface_elevation_m": 0.025})),
        "terrain": dict(DEFAULT_TERRAIN),
        "banking": list(normalized.get("banking", [])),
    }


def _is_collision_proxy(obj: bpy.types.Object) -> bool:
    return bool(obj.name.endswith("-colonly") or obj.get("formula90s_collision"))


def build_barrier(name: str, points_xz: list, config: dict, material) -> tuple:
    """Build one simplified barrier visual + collision proxy box."""
    a = godot_xz_to_blender(float(points_xz[0][0]), float(points_xz[0][1]), 0.0)
    b = godot_xz_to_blender(float(points_xz[1][0]), float(points_xz[1][1]), 0.0)
    span = b - a
    length = span.length
    if length <= 1e-6:
        raise RuntimeError(f"barrier {name}: degenerate zero-length segment")
    direction = span / length
    center = (a + b) * 0.5
    half_len = max(0.05, length * 0.5)
    half_thick = float(config.get("barrier_thickness_half_m", 0.15))
    half_h = float(config.get("barrier_height_half_m", 0.55))
    bpy.ops.mesh.primitive_cube_add(
        location=tuple(center),
        scale=(half_len, half_thick, half_h),
    )
    visual = bpy.context.object
    visual.name = f"Barrier_{name}"
    visual.rotation_euler = (0.0, 0.0, math.atan2(direction.y, direction.x))
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
    visual.data.materials.append(material)
    visual["formula90s_barrier_visual"] = True

    collision = visual.copy()
    collision.data = visual.data
    bpy.context.scene.collection.objects.link(collision)
    collision.name = f"Barrier_{name}-colonly"
    collision.hide_render = True
    collision.display_type = "WIRE"
    collision["formula90s_collision"] = True
    collision["formula90s_simplified_barrier_collision"] = True
    return visual, collision


def _load_glb_template(spec, repo_root: Path) -> tuple[object, Matrix]:
    """Import a registry GLB once and return ``(mesh_data, normalized_matrix)``.

    The returned mesh data is reused by every instance of this asset; only the
    shared mesh is kept (the temporary import wrapper objects are removed), so
    placing thousands of instances does not re-run the glTF importer per object.
    """
    source = repo_root / spec.source
    if not source.is_file():
        raise RuntimeError(f"asset {spec.id}: missing source {source}")
    before = set(bpy.data.objects)
    try:
        bpy.ops.import_scene.gltf(filepath=str(source))
    except Exception as exc:  # noqa: BLE001 - surface which asset failed to import
        raise RuntimeError(f"asset {spec.id}: glTF import failed for {source}: {exc}") from exc
    imported = [obj for obj in bpy.data.objects if obj not in before]
    meshes = [obj for obj in imported if obj.type == "MESH"]
    if len(meshes) != 1:
        for obj in imported:
            bpy.data.objects.remove(obj, do_unlink=True)
        raise RuntimeError(
            f"asset {spec.id}: GLB must contain exactly one mesh, got {len(meshes)}"
        )
    source_mesh = meshes[0]
    corners = [source_mesh.matrix_world @ Vector(corner) for corner in source_mesh.bound_box]
    min_z = min(v.z for v in corners)
    center_x = (min(v.x for v in corners) + max(v.x for v in corners)) * 0.5
    center_y = (min(v.y for v in corners) + max(v.y for v in corners)) * 0.5
    normalized = Matrix.Translation((-center_x, -center_y, -min_z)) @ source_mesh.matrix_world.copy()
    mesh_data = source_mesh.data
    for obj in imported:
        bpy.data.objects.remove(obj, do_unlink=True)
    return mesh_data, normalized


def _instance_glb(placement: dict, spec, mesh_data, normalized: Matrix) -> bpy.types.Object:
    """Create one collision-free instance sharing a pre-imported mesh."""
    root = bpy.data.objects.new(
        f"Asset_{placement['instance_id']}_{spec.id}", mesh_data
    )
    bpy.context.scene.collection.objects.link(root)
    x, z = placement["position_xz"]
    position = godot_xz_to_blender(float(x), float(z), 0.0)
    transform = Matrix.Translation(position) @ Matrix.Rotation(
        -float(placement.get("yaw_rad", 0.0)), 4, "Z"
    )
    scale = float(placement.get("scale", 1.0))
    transform @= Matrix.Diagonal((scale, scale, scale, 1.0))
    root.matrix_world = transform @ normalized
    root["formula90s_asset_id"] = spec.id
    root["formula90s_instance_id"] = placement["instance_id"]
    root["formula90s_collision"] = False
    return root


def import_glb_asset(placement: dict, spec, repo_root: Path) -> bpy.types.Object:
    """Import one registry asset GLB and instance it collision-free."""
    return _instance_glb(
        placement, spec, *_load_glb_template(spec, repo_root)
    )


def build_procedural_card(placement: dict, spec, material) -> bpy.types.Object:
    """Build a collision-free billboard card for a procedural registry asset."""
    name = f"Asset_{placement['instance_id']}_{spec.id}"
    x, z = placement["position_xz"]
    width = float(spec.dimensions_m.get("width", 0.8))
    height = float(spec.dimensions_m.get("height", 1.8))
    yaw = -float(placement.get("yaw_rad", 0.0))
    half = width * 0.5
    center = godot_xz_to_blender(float(x), float(z), 0.0)
    rotation = Matrix.Rotation(yaw, 4, "Z")
    base = [(-half, 0.0, 0.0), (half, 0.0, 0.0), (half, 0.0, height), (-half, 0.0, height)]
    verts = [tuple(center + rotation @ Vector(v)) for v in base]
    mesh = bpy.data.meshes.new(name + "Mesh")
    mesh.from_pydata(verts, [], [(0, 1, 2, 3)])
    mesh.uv_layers.new(name="UVMap")
    for loop, uv in zip(mesh.polygons[0].loop_indices, ((0, 0), (1, 0), (1, 1), (0, 1))):
        mesh.uv_layers[0].data[loop].uv = uv
    mesh.materials.append(material)
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(obj)
    obj["formula90s_asset_id"] = spec.id
    obj["formula90s_instance_id"] = placement["instance_id"]
    obj["formula90s_collision"] = False
    return obj


def _is_glb_source(spec) -> bool:
    return bool(spec.source and str(spec.source).lower().endswith(".glb"))


def import_asset(placement: dict, spec, repo_root: Path, flag_material) -> bpy.types.Object:
    if _is_glb_source(spec):
        return import_glb_asset(placement, spec, repo_root)
    if spec.kind in PROCEDURAL_KINDS or spec.kind in {"card"}:
        return build_procedural_card(placement, spec, flag_material)
    raise RuntimeError(
        f"asset {spec.id} ({placement['instance_id']}): no GLB source and kind "
        f"{spec.kind!r} has no procedural builder"
    )


def export_runtime_part(target: Path, objects: list) -> int:
    bpy.ops.object.select_all(action="DESELECT")
    selected = 0
    for obj in objects:
        if obj.hide_get():
            continue
        obj.select_set(True)
        selected += 1
    if selected == 0:
        raise RuntimeError(f"Runtime GLB part has no visible objects: {target}")
    atomic_export_glb(target, use_selection=True, export_extras=False)
    bpy.ops.object.select_all(action="DESELECT")
    return selected


def build_manifest(
    compile_manifest: dict,
    output: Path,
    normalized_path: Path,
    track_id: str,
    stats: dict,
    barrier_count: int,
    asset_count: int,
    env_count: int,
    veg_count: int,
    collision_names: set[str],
    asset_names: set[str],
) -> dict:
    source_sha = compile_manifest["artifacts"]["source"]["sha256"]
    registry_sha = compile_manifest["artifacts"]["registry"]["sha256"]
    normalized_sha = compile_manifest["artifacts"]["normalized"]["sha256"]
    compiler_sha = compiler_sha256()
    artifact_names = (
        CANONICAL_ARTIFACT,
        NORMALIZED_ARTIFACT,
        MANIFEST_ARTIFACT,
        BLEND_FILE,
        ENVIRONMENT_GLB,
        VEGETATION_GLB,
    )
    artifacts = {name: {"file": name, "sha256": _sha256_file(output / name)} for name in artifact_names}
    return {
        "schema_version": BUILD_MANIFEST_SCHEMA_VERSION,
        "build_sha256": build_sha256(source_sha, registry_sha, compiler_sha, normalized_sha),
        "track_id": track_id,
        "inputs": {
            "source": {
                "file": compile_manifest["artifacts"]["source"]["file"],
                "sha256": source_sha,
            },
            "registry": {
                "file": compile_manifest["artifacts"]["registry"]["file"],
                "sha256": registry_sha,
            },
            "normalized": {"file": NORMALIZED_ARTIFACT, "sha256": normalized_sha},
            "compiler": {"name": COMPILER_NAME, "version": COMPILER_VERSION, "sha256": compiler_sha},
        },
        "artifacts": artifacts,
        "blender_version": bpy.app.version_string,
        "stats": {
            "terrain_vertices": stats["vertices"],
            "terrain_triangles": stats["triangles"],
            "terrain_cell_m": stats["cell_m"],
            "barrier_segments": barrier_count,
            "asset_instances": asset_count,
            "environment_objects": env_count,
            "vegetation_objects": veg_count,
            "collision_proxies": len(collision_names),
            "collision_proxy_names": sorted(collision_names),
            "asset_root_names": sorted(asset_names),
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Build Blender/GLB artifacts from a normalized F90 Track JSON."
    )
    parser.add_argument("--normalized", required=True)
    parser.add_argument("--registry", required=True)
    parser.add_argument("--output-dir", required=True)
    ns = parser.parse_args(args_after_double_dash())

    output = Path(ns.output_dir).resolve()
    output.mkdir(parents=True, exist_ok=True)
    normalized_path = Path(ns.normalized).resolve()
    normalized = read_json(normalized_path)
    registry_path = Path(ns.registry).resolve()
    registry = load_registry(registry_path, repo_root=_REPO_ROOT)
    registry_errors = validate_registry(registry)
    if registry_errors:
        raise RuntimeError("registry invalid: " + "; ".join(registry_errors))

    config = coerce_track_config(normalized)
    points = normalized["centerline"]["points_xz"]
    track_id = normalized["track_id"]
    compile_manifest = read_json(output / MANIFEST_ARTIFACT)

    clear_scene()
    materials = {
        "ground": flat_material("F90_GrassGround", (0.24, 0.42, 0.20), roughness=1.0),
        "asphalt": flat_material("F90_AsphaltFlat", (0.20, 0.20, 0.21), roughness=0.90),
        "shoulder": flat_material("F90_ShoulderFlat", (0.30, 0.36, 0.22), roughness=1.0),
        "guardrail": flat_material("F90_BarrierGray", (0.55, 0.55, 0.58), roughness=0.50, metallic=0.50),
        "flag": flat_material("F90_ProceduralFlag", (0.72, 0.055, 0.035), roughness=0.92),
    }

    stats = build_terrain(points, config, materials["ground"])
    build_safety_floor(stats, config)
    half = float(config["road"]["width_m"]) * 0.5
    surface_z = float(config["road"].get("surface_elevation_m", 0.025))
    build_ribbon(points, half, surface_z, "RoadVisual", materials["asphalt"], "RoadCollision-colonly", config)
    shoulder_left = build_roadside_shoulder(points, "left", config, materials["shoulder"])
    shoulder_right = build_roadside_shoulder(points, "right", config, materials["shoulder"])
    build_roadside_collision(points, "left", config)
    build_roadside_collision(points, "right", config)

    barrier_visuals: list[bpy.types.Object] = []
    barrier_collisions: list[bpy.types.Object] = []
    for index, barrier in enumerate(normalized["barriers"]):
        name = f"{barrier.get('kind', 'barrier')}_{index:02d}"
        visual, collision = build_barrier(name, barrier["points_xz"], config, materials["guardrail"])
        barrier_visuals.append(visual)
        barrier_collisions.append(collision)

    asset_roots: list[bpy.types.Object] = []
    glb_templates: dict[str, tuple[object, Matrix]] = {}
    for placement in normalized["assets"]:
        spec = registry.lookup(placement["asset_id"])
        if spec is None:
            raise RuntimeError(
                f"asset {placement['asset_id']} ({placement['instance_id']}): not in registry"
            )
        if _is_glb_source(spec):
            if spec.id not in glb_templates:
                glb_templates[spec.id] = _load_glb_template(spec, _REPO_ROOT)
            asset_roots.append(
                _instance_glb(placement, spec, *glb_templates[spec.id])
            )
        elif spec.kind in PROCEDURAL_KINDS or spec.kind in {"card"}:
            asset_roots.append(build_procedural_card(placement, spec, materials["flag"]))
        else:
            raise RuntimeError(
                f"asset {spec.id} ({placement['instance_id']}): no GLB source and kind "
                f"{spec.kind!r} has no procedural builder"
            )

    env_objects = [
        bpy.data.objects["GrassTerrainVisual"],
        bpy.data.objects["GrassTerrainCollision-colonly"],
        bpy.data.objects["RoadVisual"],
        bpy.data.objects["RoadCollision-colonly"],
        bpy.data.objects["GrassSafetyFloor-colonly"],
        bpy.data.objects["GrassEdgeCollisionLeft-colonly"],
        bpy.data.objects["GrassEdgeCollisionRight-colonly"],
    ]
    if shoulder_left is not None:
        env_objects.append(shoulder_left)
    if shoulder_right is not None:
        env_objects.append(shoulder_right)
    env_objects += barrier_visuals + barrier_collisions
    collision_names = {obj.name for obj in env_objects if _is_collision_proxy(obj)}
    asset_names = {root.name for root in asset_roots}

    atomic_save_blend(output / BLEND_FILE, output / "backups")
    env_count = export_runtime_part(output / ENVIRONMENT_GLB, env_objects)
    veg_count = export_runtime_part(output / VEGETATION_GLB, asset_roots)
    validate_runtime_split(
        output / ENVIRONMENT_GLB,
        output / VEGETATION_GLB,
        collision_names,
        asset_names,
    )

    manifest = build_manifest(
        compile_manifest,
        output,
        normalized_path,
        track_id,
        stats,
        barrier_count=len(barrier_visuals),
        asset_count=len(asset_roots),
        env_count=env_count,
        veg_count=veg_count,
        collision_names=collision_names,
        asset_names=asset_names,
    )
    (output / BUILD_MANIFEST_FILE).write_text(
        json.dumps(manifest, indent=2, sort_keys=True, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )
    print(json.dumps({
        "track_id": track_id,
        "build_sha256": manifest["build_sha256"],
        "terrain_vertices": stats["vertices"],
        "terrain_triangles": stats["triangles"],
        "barrier_segments": len(barrier_visuals),
        "asset_instances": len(asset_roots),
        "environment_objects": env_count,
        "vegetation_objects": veg_count,
        "collision_proxies": len(collision_names),
        "environment_glb": str(output / ENVIRONMENT_GLB),
        "vegetation_glb": str(output / VEGETATION_GLB),
        "blend": str(output / BLEND_FILE),
    }, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
