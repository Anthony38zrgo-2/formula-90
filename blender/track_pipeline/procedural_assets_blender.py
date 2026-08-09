from __future__ import annotations

import math

import bpy
from mathutils import Vector

from procedural_catalog import get_region_profile


def godot_xz_to_blender(x: float, z: float, height: float = 0.0) -> Vector:
    return Vector((float(x), -float(z), float(height)))


def bank_degrees_at_fraction(config: dict, fraction: float) -> float:
    result = 0.0
    f = fraction % 1.0
    for zone in config.get("banking", []):
        center = float(zone["center_fraction"]) % 1.0
        half = max(float(zone["half_width_fraction"]), 1e-6)
        d = abs((f - center + 0.5) % 1.0 - 0.5)
        if d <= half:
            weight = 0.5 * (1.0 + math.cos(math.pi * d / half))
            result += float(zone["degrees"]) * weight
    return result


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
    nx = tz
    nz = -tx
    return (x, z), (tx, tz), (nx, nz)


def effective_far_ground_z(config: dict) -> float:
    road_half = float(config["road"]["width_m"]) * 0.5
    surface_z = float(config["road"].get("surface_elevation_m", 0.025))
    clearance = float(config.get("terrain", {}).get("road_clearance_m", 0.012))
    requested = float(config.get("terrain", {}).get("far_ground_z_m", -0.05))
    max_bank = max((abs(float(zone.get("degrees", 0.0))) for zone in config.get("banking", [])), default=0.0)
    lowest_banked_edge = surface_z - math.tan(math.radians(max_bank)) * road_half
    return min(requested, lowest_banked_edge - clearance - 0.02)


def terrain_height(config: dict, fraction: float, side: float, distance_from_center_m: float) -> float:
    road_half = float(config["road"]["width_m"]) * 0.5
    surface_z = float(config["road"].get("surface_elevation_m", 0.025))
    terrain = config.get("terrain", {})
    clearance = float(terrain.get("road_clearance_m", 0.012))
    shoulder_width = max(float(terrain.get("shoulder_width_m", 26.0)), 0.1)
    far_z = effective_far_ground_z(config)
    bank = math.radians(bank_degrees_at_fraction(config, fraction))
    road_edge_height = surface_z + math.tan(bank) * road_half * side
    inner_ground = road_edge_height - clearance
    outside = max(0.0, float(distance_from_center_m) - road_half)
    t = min(1.0, outside / shoulder_width)
    return inner_ground * (1.0 - t) + far_z * t


def _mesh_object(name: str, verts, faces, material=None, uvs=None):
    mesh = bpy.data.meshes.new(name + "Mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(obj)
    if material:
        obj.data.materials.append(material)
    if uvs is not None:
        uv_layer = mesh.uv_layers.new(name="UVMap")
        for polygon in mesh.polygons:
            for loop_index in polygon.loop_indices:
                vertex_index = mesh.loops[loop_index].vertex_index
                uv_layer.data[loop_index].uv = uvs[vertex_index]
    return obj


def _card(name: str, width: float, height: float, material, axis: str = "X"):
    if axis == "X":
        verts = [(-width/2, 0, 0), (width/2, 0, 0), (width/2, 0, height), (-width/2, 0, height)]
    else:
        verts = [(0, -width/2, 0), (0, width/2, 0), (0, width/2, height), (0, -width/2, height)]
    faces = [(0, 1, 2, 3)]
    uvs = [(0, 0), (1, 0), (1, 1), (0, 1)]
    return _mesh_object(name, verts, faces, material, uvs)


def _cube(name: str, size_xyz, location, material=None):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=location)
    obj = bpy.context.object
    obj.name = name
    obj.scale = tuple(float(v) for v in size_xyz)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if material:
        obj.data.materials.append(material)
    return obj


def _cone(name: str, radius1: float, radius2: float, depth: float, z: float, material):
    bpy.ops.mesh.primitive_cone_add(vertices=6, radius1=radius1, radius2=radius2, depth=depth, location=(0, 0, z))
    obj = bpy.context.object
    obj.name = name
    obj.data.materials.append(material)
    return obj


def _ico(name: str, radius: float, location, scale, material):
    bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=1, radius=radius, location=location)
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    obj.data.materials.append(material)
    return obj


def _hide_prototype(objects):
    for obj in objects:
        obj.hide_render = True
        obj.hide_viewport = True
        obj.hide_set(True)
    return objects


def _tree_broadleaf(materials):
    trunk = _cone("Proto_SA_Broadleaf_Trunk", 0.34, 0.24, 3.8, 1.9, materials["trunk"])
    crown = _ico("Proto_SA_Broadleaf_Crown", 2.15, (0, 0, 4.4), (1.0, 0.92, 0.78), materials["foliage_green"])
    crown2 = _ico("Proto_SA_Broadleaf_Crown2", 1.45, (0.85, -0.25, 4.0), (1.0, 0.85, 0.72), materials["foliage_green"])
    return _hide_prototype([trunk, crown, crown2])


def _tree_dry(materials):
    trunk = _cone("Proto_SA_Dry_Trunk", 0.30, 0.20, 3.1, 1.55, materials["trunk"])
    crown = _ico("Proto_SA_Dry_Crown", 1.85, (0, 0, 3.7), (1.18, 0.95, 0.58), materials["foliage_dry"])
    crown2 = _ico("Proto_SA_Dry_Crown2", 1.25, (-1.0, 0.25, 3.45), (1.0, 0.85, 0.52), materials["foliage_dry"])
    return _hide_prototype([trunk, crown, crown2])


def _palm_fronds(material):
    verts = [(0, 0, 0.0)]
    faces = []
    uv = [(0.5, 0.0)]
    count = 8
    for i in range(count):
        a = math.tau * i / count
        length = 2.2 + 0.25 * (i % 2)
        width = 0.36
        center = Vector((math.cos(a) * length * 0.56, math.sin(a) * length * 0.56, 0.02 - 0.12 * (i % 3)))
        side = Vector((-math.sin(a), math.cos(a), 0.0)) * width
        tip = Vector((math.cos(a) * length, math.sin(a) * length, -0.24))
        verts.extend([tuple(center - side), tuple(center + side), tuple(tip)])
        base = 1 + i * 3
        faces.append((0, base, base + 2, base + 1))
        uv.extend([(0.0, 0.35), (1.0, 0.35), (0.5, 1.0)])
    return _mesh_object("Proto_SA_Palm_Fronds", verts, faces, material, uv)


def _tree_palm(materials):
    trunk = _cone("Proto_SA_Palm_Trunk", 0.28, 0.18, 5.8, 2.9, materials["trunk"])
    fronds = _palm_fronds(materials["foliage_green"])
    fronds.location.z = 5.7
    return _hide_prototype([trunk, fronds])


def _cross_cards(prefix: str, width: float, height: float, material):
    a = _card(prefix + "_A", width, height, material, "X")
    b = _card(prefix + "_B", width, height, material, "Y")
    return _hide_prototype([a, b])


def _building(prefix: str, width: float, height: float, material):
    card = _card(prefix, width, height, material, "X")
    return _hide_prototype([card])


def create_prototypes(materials: dict, region: str) -> dict[str, list[bpy.types.Object]]:
    get_region_profile(region)
    return {
        "sa_broadleaf": _tree_broadleaf(materials),
        "sa_dry_canopy": _tree_dry(materials),
        "sa_palm": _tree_palm(materials),
        "sa_bush_round": _cross_cards("Proto_SA_BushRound", 2.2, 1.65, materials["bush_green"]),
        "sa_bush_dry": _cross_cards("Proto_SA_BushDry", 1.9, 1.35, materials["bush_dry"]),
        "sa_grass_clump": _cross_cards("Proto_SA_Grass", 0.75, 0.75, materials["grass_green"]),
        "sa_grass_dry": _cross_cards("Proto_SA_GrassDry", 0.68, 0.68, materials["grass_dry"]),
        "sa_building_low": _building("Proto_SA_BuildingLow", 11.0, 6.5, materials["building_low"]),
        "sa_building_warehouse": _building("Proto_SA_Warehouse", 17.0, 7.5, materials["building_warehouse"]),
    }


def instantiate_prototype(source_objects, name: str, x: float, z: float, height: float, yaw: float, scale: float):
    root = bpy.data.objects.new(name, None)
    bpy.context.scene.collection.objects.link(root)
    root.location = godot_xz_to_blender(x, z, height)
    root.rotation_euler[2] = -float(yaw)
    root.scale = (scale, scale, scale)
    for src in source_objects:
        copy = src.copy()
        if src.data is not None:
            copy.data = src.data
        copy.hide_render = False
        copy.hide_viewport = False
        copy.hide_set(False)
        bpy.context.scene.collection.objects.link(copy)
        copy.parent = root
    return root


def create_guardrail_prototype(config: dict, materials: dict):
    settings = config["guardrails"]
    length = float(settings.get("module_length_m", 4.0))
    rail_depth = float(settings.get("visual_depth_m", 0.11))
    rail_height = float(settings.get("beam_height_m", 0.34))
    center_z = float(settings.get("beam_center_height_m", 0.58))
    post_height = float(settings.get("post_height_m", 0.82))
    post_width = float(settings.get("post_width_m", 0.10))
    post_depth = float(settings.get("post_depth_m", 0.12))
    mat = materials["guardrail"]

    objects = []
    strip_h = rail_height / 3.0
    for i, offset in enumerate((-0.07, 0.0, 0.07)):
        strip = _cube(
            f"Proto_Guardrail_Beam_{i}",
            (length, rail_depth, strip_h),
            (0.0, offset, center_z + (i - 1) * strip_h * 0.82),
            mat,
        )
        objects.append(strip)

    post_spacing = max(1.0, float(settings.get("post_spacing_m", 2.0)))
    count = max(2, int(math.floor(length / post_spacing)) + 1)
    for i in range(count):
        x = -length * 0.5 + i * (length / max(1, count - 1))
        post = _cube(
            f"Proto_Guardrail_Post_{i}",
            (post_width, post_depth, post_height),
            (x, 0.0, post_height * 0.5),
            mat,
        )
        objects.append(post)

    return _hide_prototype(objects), length


def create_guardrail_collision(name: str, pos, tangent, length: float, ground_z: float, config: dict):
    settings = config["guardrails"]
    height = float(settings["collision_height_m"])
    thickness = float(settings["collision_thickness_m"])
    x, z = pos
    bpy.ops.mesh.primitive_cube_add(location=godot_xz_to_blender(x, z, ground_z + height * 0.5))
    obj = bpy.context.object
    obj.name = name + "-colonly"
    tx, tz = tangent
    obj.rotation_euler[2] = math.atan2(-tz, tx)
    obj.scale = (length * 0.5, thickness * 0.5, height * 0.5)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    obj.hide_render = True
    obj.display_type = "WIRE"
    return obj
