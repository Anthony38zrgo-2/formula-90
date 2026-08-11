from __future__ import annotations

import math
from pathlib import Path

import bpy
import numpy as np
from mathutils import Vector

from procedural_catalog import biome_from_config, specs_for_biome
from terrain_grid import bank_degrees_at_fraction, effective_far_ground_z


def _closed_polyline_length(points) -> float:
    total = 0.0
    for index, point in enumerate(points):
        next_point = points[(index + 1) % len(points)]
        total += math.hypot(float(next_point[0]) - float(point[0]), float(next_point[1]) - float(point[1]))
    return total


def godot_xz_to_blender(x: float, z: float, height: float = 0.0) -> Vector:
    return Vector((float(x), -float(z), float(height)))


def sample_centerline(points, fraction):
    n = len(points)
    f = (fraction % 1.0) * n
    i = int(math.floor(f)) % n
    t = f - math.floor(f)
    ax, az = points[i]
    bx, bz = points[(i + 1) % n]
    x = ax + (bx - ax) * t
    z = az + (bz - az) * t
    tx = bx - ax
    tz = bz - az
    length = max(math.hypot(tx, tz), 1e-9)
    tx /= length
    tz /= length
    return (x, z), (tx, tz), (tz, -tx)


def terrain_height(config, fraction, side, distance_from_center_m):
    road_half = float(config["road"]["width_m"]) * 0.5
    surface = float(config["road"].get("surface_elevation_m", 0.025))
    falloff = max(float(config.get("terrain", {}).get("shoulder_falloff_m", 18.0)), 0.1)
    far = effective_far_ground_z(config)
    bank = math.radians(bank_degrees_at_fraction(config, fraction))
    edge = surface + math.tan(bank) * road_half * side
    t = min(1.0, max(0.0, (float(distance_from_center_m) - road_half) / falloff))
    t = t * t * (3 - 2 * t)
    return edge * (1 - t) + far * t


def _mesh_object(name, verts, faces, materials=None, face_materials=None, uv_by_face=None):
    mesh = bpy.data.meshes.new(name + "Mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(obj)
    for mat in materials or []:
        obj.data.materials.append(mat)
    if face_materials:
        for i, m in enumerate(face_materials):
            if i < len(mesh.polygons):
                mesh.polygons[i].material_index = int(m)
    if uv_by_face:
        uv = mesh.uv_layers.new(name="UVMap")
        for pi, poly in enumerate(mesh.polygons):
            for li, loop in enumerate(poly.loop_indices):
                uv.data[loop].uv = uv_by_face[pi][li]
    return obj


def _card(name, width, height, material, angle_rad=0.0):
    half = width * 0.5
    ca = math.cos(angle_rad)
    sa = math.sin(angle_rad)

    def p(x, z):
        return (x * ca, x * sa, z)

    verts = [p(-half, 0), p(half, 0), p(half, height), p(-half, height)]
    return _mesh_object(
        name,
        verts,
        [(0, 1, 2, 3)],
        [material],
        uv_by_face=[[(0, 0), (1, 0), (1, 1), (0, 1)]],
    )


def _hide(objs):
    for obj in objs:
        obj.hide_render = True
        obj.hide_viewport = True
        obj.hide_set(True)
    return objs


def _crossed(prefix, width, height, material, planes):
    planes = max(1, int(planes))
    return _hide([
        _card(f"{prefix}_P{i+1}", width, height, material, math.pi * i / planes)
        for i in range(planes)
    ])


def _building(prefix, width, height, depth, facade, roof):
    w = width * 0.5
    d = depth * 0.5
    verts = [
        (-w, -d, 0), (w, -d, 0), (w, d, 0), (-w, d, 0),
        (-w, -d, height), (w, -d, height), (w, d, height), (-w, d, height),
    ]
    faces = [(0,1,5,4), (1,2,6,5), (2,3,7,6), (3,0,4,7), (4,5,6,7)]
    uv = [[(0,0),(1,0),(1,1),(0,1)]] * 5
    return _hide([_mesh_object(prefix, verts, faces, [facade, roof], [0,0,0,0,1], uv)])


def create_prototypes(materials, config):
    biome = biome_from_config(config)
    out = {}
    for category in ("trees", "bushes", "grass", "fake_buildings"):
        for spec in specs_for_biome(biome, category):
            mat = materials[f"asset:{spec.id}"]
            if category == "trees":
                out[spec.id] = _crossed(f"Proto_{spec.id}", spec.width_m, spec.height_m, mat, 3)
            elif category == "bushes":
                out[spec.id] = _crossed(f"Proto_{spec.id}", spec.width_m, spec.height_m, mat, 2)
            elif category == "grass":
                out[spec.id] = _crossed(f"Proto_{spec.id}", spec.width_m, spec.height_m, mat, 1)
            else:
                out[spec.id] = _building(
                    f"Proto_{spec.id}", spec.width_m, spec.height_m, spec.depth_m,
                    mat, materials["roof"],
                )
    return out


def instantiate_prototype(
    source_objects,
    name,
    x,
    z,
    height,
    yaw,
    scale,
    tint_rgb=None,
    width_scale: float = 1.0,
    height_scale: float = 1.0,
):
    root = bpy.data.objects.new(name, None)
    bpy.context.scene.collection.objects.link(root)
    root.location = godot_xz_to_blender(x, z, height)
    root.rotation_euler[2] = -float(yaw)
    horizontal = float(scale) * float(width_scale)
    vertical = float(scale) * float(height_scale)
    root.scale = (horizontal, horizontal, vertical)
    if tint_rgb is not None:
        root["formula90s_tint_rgb"] = list(tint_rgb)
    for src in source_objects:
        copy = src.copy()
        copy.data = src.data if src.data is not None else None
        copy.hide_render = False
        copy.hide_viewport = False
        copy.hide_set(False)
        bpy.context.scene.collection.objects.link(copy)
        copy.parent = root
    return root


def _cube(name, size_xyz, location, material=None):
    bpy.ops.mesh.primitive_cube_add(size=1, location=location)
    obj = bpy.context.object
    obj.name = name
    obj.scale = tuple(float(v) for v in size_xyz)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if material:
        obj.data.materials.append(material)
    return obj


def create_guardrail_prototype(config, materials):
    s = config["guardrails"]
    length = float(s.get("module_length_m", 4))
    depth = float(s.get("visual_depth_m", .11))
    height = float(s.get("beam_height_m", .34))
    center = float(s.get("beam_center_height_m", .58))
    ph = float(s.get("post_height_m", .82))
    pw = float(s.get("post_width_m", .10))
    pd = float(s.get("post_depth_m", .12))
    mat = materials["guardrail"]
    objs = []
    strip = height / 3
    for i, offset in enumerate((-.07, 0, .07)):
        objs.append(_cube(
            f"Proto_Guardrail_Beam_{i}",
            (length, depth, strip),
            (0, offset, center + (i-1) * strip * .82),
            mat,
        ))
    spacing = max(1, float(s.get("post_spacing_m", 2)))
    count = max(2, int(math.floor(length / spacing)) + 1)
    for i in range(count):
        objs.append(_cube(
            f"Proto_Guardrail_Post_{i}",
            (pw, pd, ph),
            (-length*.5 + i*(length/max(1,count-1)), 0, ph*.5),
            mat,
        ))
    return _hide(objs), length


def create_guardrail_collision(name, pos, tangent, length, ground_z, config):
    s = config["guardrails"]
    h = float(s["collision_height_m"])
    t = float(s["collision_thickness_m"])
    x, z = pos
    bpy.ops.mesh.primitive_cube_add(location=godot_xz_to_blender(x, z, ground_z + h*.5))
    obj = bpy.context.object
    obj.name = name + "-colonly"
    tx, tz = tangent
    obj.rotation_euler[2] = math.atan2(-tz, tx)
    obj.scale = (length*.5, t*.5, h*.5)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    obj.hide_render = True
    obj.display_type = "WIRE"
    return obj


def _append_low_poly_tire(vertices, faces, center: Vector, tangent_b: Vector, outward_b: Vector, major: float, minor: float):
    major_segments = 8
    minor_segments = 4
    start = len(vertices)
    tangent_b = tangent_b.normalized()
    outward_b = outward_b.normalized()
    up = Vector((0.0, 0.0, 1.0))
    for i in range(major_segments):
        theta = math.tau * i / major_segments
        radial = tangent_b * math.cos(theta) + up * math.sin(theta)
        for j in range(minor_segments):
            phi = math.tau * j / minor_segments
            point = center + radial * (major + minor * math.sin(phi)) + outward_b * (minor * math.cos(phi))
            vertices.append(tuple(point))
    for i in range(major_segments):
        ni = (i + 1) % major_segments
        for j in range(minor_segments):
            nj = (j + 1) % minor_segments
            a = start + i * minor_segments + j
            b = start + ni * minor_segments + j
            c = start + ni * minor_segments + nj
            d = start + i * minor_segments + nj
            faces.append((a, b, c, d))


def create_tire_barrier_visual(name, points, side, config, material, asset_glb=None):
    tire_cfg = config["tire_barriers"]
    road_half = float(config["road"]["width_m"]) * 0.5
    distance = road_half + float(tire_cfg.get("separation_from_edge_m", 5.0))
    lap = _closed_polyline_length(points)

    if asset_glb:
        # Reusable Jordan 3x2 module: import once, hide as prototype, then
        # linked-duplicate per module along the centerline. The module's X axis
        # runs along the wall; module length is its measured along-track width.
        module_length = float(tire_cfg.get("module_length_m", 1.8))
        count = max(1, int(math.ceil(lap / module_length)))
        root = bpy.data.objects.new(name, None)
        bpy.context.scene.collection.objects.link(root)
        root["formula90s_continuous_tire_barrier"] = True
        root["formula90s_collision"] = False
        for index in range(count):
            fraction = (index + 0.5) / count
            pos, tangent, normal = sample_centerline(points, fraction)
            ground = terrain_height(config, fraction, side, distance)
            center = godot_xz_to_blender(pos[0] + normal[0] * side * distance, pos[1] + normal[1] * side * distance, ground)
            tangent_b = Vector((tangent[0], -tangent[1], 0.0))
            duplicate = _import_asset_instance(asset_glb, f"{name}_m{index:04d}")
            duplicate.parent = root
            duplicate.location = center
            # Rotate the module so its local X (wall direction) aligns with the tangent.
            yaw = math.atan2(tangent_b.y, tangent_b.x)
            duplicate.rotation_euler[2] = yaw
        return root, count

    module_length = max(0.8, float(tire_cfg.get("module_length_m", 2.4)))
    radius = max(0.12, float(tire_cfg.get("tire_major_radius_m", 0.34)))
    minor = max(0.04, float(tire_cfg.get("tire_minor_radius_m", 0.11)))
    rows = max(1, int(tire_cfg.get("stack_rows", 2)))
    per_row = max(1, int(tire_cfg.get("tires_per_row", 2)))
    count = max(1, int(math.ceil(lap / module_length)))
    vertices = []
    faces = []
    for index in range(count):
        fraction = (index + 0.5) / count
        pos, tangent, normal = sample_centerline(points, fraction)
        ground = terrain_height(config, fraction, side, distance)
        center = godot_xz_to_blender(pos[0] + normal[0] * side * distance, pos[1] + normal[1] * side * distance, ground)
        tangent_b = Vector((tangent[0], -tangent[1], 0.0))
        outward_b = Vector((side * normal[0], -side * normal[1], 0.0))
        for row in range(rows):
            height = radius + minor + row * (2.0 * radius * 0.82)
            for col in range(per_row):
                along = (col - (per_row - 1) * 0.5) * radius * 1.62
                tire_center = center + tangent_b * along + Vector((0.0, 0.0, height))
                _append_low_poly_tire(vertices, faces, tire_center, tangent_b, outward_b, radius, minor)
    obj = _mesh_object(name, vertices, faces, [material])
    obj["formula90s_continuous_tire_barrier"] = True
    obj["formula90s_collision"] = False
    return obj, count


def _import_asset_instance(asset_glb, instance_name):
    """Import a GLB asset once into a hidden prototype and return a linked copy
    as a fresh object instance. The imported source objects are removed after
    the prototype is captured so repeated calls stay cheap."""
    cache = getattr(bpy, "_tire_module_prototype_meshes", None)
    if cache is None:
        before = set(bpy.data.objects)
        bpy.ops.import_scene.gltf(filepath=str(Path(asset_glb).resolve()))
        imported = [obj for obj in bpy.data.objects if obj not in before]
        root = None
        for obj in imported:
            if obj.type == "EMPTY" and obj.name.startswith("TireBarrierJordan6"):
                root = obj
                break
        if root is None:
            raise RuntimeError("tire barrier GLB is missing the TireBarrierJordan6 root")
        meshes = [obj for obj in root.children if obj.type == "MESH"]
        if len(meshes) != 1:
            raise RuntimeError(f"tire barrier GLB expected 1 fused module mesh, got {len(meshes)}")
        for obj in imported:
            obj.hide_render = True
            obj.hide_viewport = True
            obj.hide_set(True)
        bpy._tire_module_prototype_meshes = meshes
    meshes = bpy._tire_module_prototype_meshes
    duplicate = bpy.data.objects.new(instance_name, None)
    bpy.context.scene.collection.objects.link(duplicate)
    for child in meshes:
        copy = child.copy()
        copy.data = child.data
        bpy.context.scene.collection.objects.link(copy)
        copy.parent = duplicate
        copy.matrix_basis = child.matrix_basis.copy()
        copy.hide_render = False
        copy.hide_viewport = False
        copy.hide_set(False)
    duplicate.hide_render = False
    duplicate.hide_viewport = False
    duplicate.hide_set(False)
    return duplicate


def create_tire_barrier_collision(name, points, side, config):
    tire_cfg = config["tire_barriers"]
    module_length = max(2.0, float(tire_cfg.get("module_length_m", 2.4)) * 2.0)
    thickness = max(0.08, float(tire_cfg.get("collision_thickness_m", 0.28)))
    height = max(0.5, float(tire_cfg.get("collision_height_m", 1.45)))
    road_half = float(config["road"]["width_m"]) * 0.5
    distance = road_half + float(tire_cfg.get("separation_from_edge_m", 5.0))
    lap = _closed_polyline_length(points)
    count = max(8, int(math.ceil(lap / module_length)))
    vertices = []
    for index in range(count):
        fraction = index / count
        pos, _, normal = sample_centerline(points, fraction)
        ground = terrain_height(config, fraction, side, distance)
        barrier = np.array([pos[0] + normal[0] * side * distance, pos[1] + normal[1] * side * distance], dtype=float)
        outward = np.array([normal[0] * side, normal[1] * side], dtype=float)
        inner = barrier - outward * (thickness * 0.5)
        outer = barrier + outward * (thickness * 0.5)
        vertices.extend([
            tuple(godot_xz_to_blender(inner[0], inner[1], ground)),
            tuple(godot_xz_to_blender(outer[0], outer[1], ground)),
            tuple(godot_xz_to_blender(inner[0], inner[1], ground + height)),
            tuple(godot_xz_to_blender(outer[0], outer[1], ground + height)),
        ])
    faces = []
    for index in range(count):
        next_index = (index + 1) % count
        i = index * 4
        j = next_index * 4
        faces.extend([
            (i, j, j + 2, i + 2),
            (i + 1, i + 3, j + 3, j + 1),
            (i + 2, j + 2, j + 3, i + 3),
            (i, i + 1, j + 1, j),
        ])
    obj = _mesh_object(name + "-colonly", vertices, faces)
    obj.hide_render = True
    obj.display_type = "WIRE"
    obj["formula90s_collision"] = True
    obj["formula90s_collision_kind"] = "continuous_tire_barrier_wall"
    return obj, count


def trackside_card_shape(prop_type: str) -> list[tuple[float, float]]:
    shapes = {
        "spectator": [(-0.18, 0.0), (0.18, 0.0), (0.14, 0.55), (0.09, 0.73), (0.0, 0.93), (-0.09, 0.73), (-0.14, 0.55)],
        "marshal": [(-0.20, 0.0), (0.20, 0.0), (0.16, 0.62), (0.10, 0.84), (0.0, 1.04), (-0.10, 0.84), (-0.16, 0.62)],
        "photographer": [(-0.25, 0.0), (0.25, 0.0), (0.22, 0.35), (0.08, 0.48), (0.18, 0.62), (-0.02, 0.70), (-0.20, 0.58), (-0.08, 0.36)],
        "flag": [(-0.04, 0.0), (0.04, 0.0), (0.04, 1.75), (0.40, 1.62), (0.04, 1.42), (-0.04, 1.42)],
        "sign": [(-0.70, 0.0), (0.70, 0.0), (0.70, 1.15), (-0.70, 1.15)],
    }
    if prop_type not in shapes:
        raise KeyError(prop_type)
    return shapes[prop_type]


def create_trackside_card(name, pos, tangent, normal, side, ground, prop_type, material):
    shape = trackside_card_shape(prop_type)
    tangent_b = Vector((float(tangent[0]), -float(tangent[1]), 0.0)).normalized()
    outward_b = Vector((float(side) * float(normal[0]), -float(side) * float(normal[1]), 0.0)).normalized()
    center = godot_xz_to_blender(pos[0], pos[1], ground)
    vertices = [tuple(center + tangent_b * x + Vector((0.0, 0.0, z))) for x, z in shape]
    face = tuple(range(len(vertices)))
    if side < 0:
        face = tuple(reversed(face))
    obj = _mesh_object(name, vertices, [face], [material])
    obj["formula90s_trackside_card"] = prop_type
    obj["formula90s_collision"] = False
    return obj
