from __future__ import annotations

import math

import bpy
from mathutils import Vector

from procedural_catalog import biome_from_config, specs_for_biome
from terrain_grid import bank_degrees_at_fraction, effective_far_ground_z


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
