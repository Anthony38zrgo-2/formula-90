# Blender 4.x script for the canonical Formula-90 source blend.
# Goal: 2010-ish cockpit placement + Red Bull RB6-inspired 2010 sidepod/floor width.
# Run headless:
# blender -b source/f1_2026_2008_source.blend -P source/apply_2010_cockpit_rework.py
# NOTE (2026-09-06): Blender 5.2 re-export churns the committed GLB (extra scenes,
# flat hierarchy without the 'world' root, KHR_texture_transform on carbon), so the
# tracked binary is patched directly with the same math instead; keep this script's
# constants in sync as the reproducible source of truth.

import bpy
import shutil
from mathutils import Vector
from pathlib import Path

SHIFT = 0.30              # metres rearward, cockpit package (0.36 -> 0.30: 60 mm forward toward centre)
SIDEPOD_OUTBOARD = 0.095  # metres per side at maximum RB6-like body width (0.075 -> 0.095: +20 mm/side)
FLOOR_OUTBOARD = 0.090    # metres per side at maximum flat-floor width (0.070 -> 0.090: +20 mm/side)
ROOT = Path(bpy.data.filepath).resolve().parent.parent if bpy.data.filepath else Path.cwd()
OUT_BLEND = ROOT / "source" / "f1_2026_2008_source_cockpit2010_rb6wide.blend"
OUT_GLB = ROOT / "f1_2026_2008_chassis_cockpit2010_rb6wide.glb"
OUT_GLB_CANON = ROOT / "f1_2026_2008_chassis.glb"


def smoothstep(a, b, x):
    if b == a:
        return 0.0
    t = max(0.0, min(1.0, (x - a) / (b - a)))
    return t * t * (3.0 - 2.0 * t)


def body_shift(glb_z, x=0.0, glb_y=0.0):
    # GLB convention: +Z rear. Blender source convention: +Y front,
    # therefore glb_z = -blender_world_y and glb_y = blender_world_z.
    z = glb_z
    if z <= -1.25:
        s = 0.0
    elif z < -0.68:
        s = SHIFT * smoothstep(-1.25, -0.68, z)
    elif z <= 0.32:
        s = SHIFT
    elif z < 1.48:
        s = SHIFT * (1.0 - smoothstep(0.32, 1.48, z))
    else:
        s = 0.0

    side = smoothstep(0.28, 0.52, abs(x))
    zwin = smoothstep(-0.95, -0.45, z) * (1.0 - smoothstep(0.30, 0.85, z))
    ywin = 1.0 - smoothstep(0.38, 0.58, glb_y)
    return s + 0.07 * side * zwin * ywin


def coke_bottle_scale(glb_z, x, glb_y):
    z_in = smoothstep(0.15, 0.65, glb_z)
    z_out = 1.0 - smoothstep(1.05, 1.48, glb_z)
    ywin = smoothstep(-0.12, 0.08, glb_y)
    return 1.0 - 0.055 * z_in * z_out * ywin


def sidepod_widen_delta(glb_z, x, glb_y):
    # Broad RB6-like radiator/sidepod volume while preserving centerline monocoque,
    # engine cover and the narrow rear coke-bottle termination.
    zwin = smoothstep(-0.72, -0.28, glb_z) * (1.0 - smoothstep(0.88, 1.42, glb_z))
    lateral = smoothstep(0.26, 0.58, abs(x))
    vertical = 1.0 - smoothstep(0.42, 0.58, glb_y)
    lower = smoothstep(-0.24, -0.08, glb_y)
    return SIDEPOD_OUTBOARD * zwin * lateral * vertical * lower


def floor_widen_delta(glb_z, x):
    # 2010-like broad flat-floor footprint with gradual front and diffuser tapers.
    zwin = smoothstep(-0.95, -0.52, glb_z) * (1.0 - smoothstep(1.12, 1.55, glb_z))
    lateral = smoothstep(0.30, 0.62, abs(x))
    return FLOOR_OUTBOARD * zwin * lateral


def steer_column_shift(glb_z):
    if glb_z <= -1.52:
        return 0.0
    if glb_z >= -0.78:
        return SHIFT
    return SHIFT * smoothstep(-1.52, -0.78, glb_z)


def deform_mesh_world(obj, mode):
    if obj.type != 'MESH':
        return
    mw = obj.matrix_world.copy()
    imw = mw.inverted()
    me = obj.data
    for v in me.vertices:
        w = mw @ v.co
        x = float(w.x)
        glb_y = float(w.z)
        glb_z = float(-w.y)

        if mode == 'body':
            dz = body_shift(glb_z, x, glb_y)
            w.y -= dz
            if obj.name == 'GEO_CHASSIS_BODY':
                w.x *= coke_bottle_scale(glb_z, x, glb_y)
                # Recompute coordinates after longitudinal/coke-bottle edit; widening is lateral only.
                sign = -1.0 if w.x < 0.0 else (1.0 if w.x > 0.0 else 0.0)
                w.x += sign * sidepod_widen_delta(glb_z, float(w.x), glb_y)
        elif mode == 'rigid':
            w.y -= SHIFT
        elif mode == 'steer_column':
            w.y -= steer_column_shift(glb_z)
        elif mode == 'floor':
            side = smoothstep(0.42, 0.64, abs(x))
            top = smoothstep(-0.12, 0.02, glb_y)
            zw = smoothstep(-0.95, -0.35, glb_z) * (1.0 - smoothstep(0.55, 1.15, glb_z))
            w.y -= 0.08 * side * top * zw
            sign = -1.0 if w.x < 0.0 else (1.0 if w.x > 0.0 else 0.0)
            w.x += sign * floor_widen_delta(glb_z, float(w.x))
        v.co = imw @ w
    me.update()


body_names = {
    'GEO_CHASSIS_BODY',
    'GEO_CHASSIS_INTERIOR',
    'GEO_CHASSIS_CARBON4',
}
rigid_names = {
    'GEO_CHASSIS_MIRRORS',
    'GEO_CHASSIS_SCREEN',
    'GEO_CHASSIS_SEAT',
    'GEO_CHASSIS_SHIFTERBRAK',
    'GEO_CHASSIS_SHIFTERTHRO',
    'GEO_CHASSIS_STEER',
    'GEO_CHASSIS_CYLINDER_001',
    'GEO_CHASSIS_CARBON3',
    'GEO_CHASSIS_LATERALCAM',
    'GEO_CHASSIS_ONBOARDCAM',
}

missing = []
for name in sorted(body_names):
    ob = bpy.data.objects.get(name)
    if ob: deform_mesh_world(ob, 'body')
    else: missing.append(name)
for name in sorted(rigid_names):
    ob = bpy.data.objects.get(name)
    if ob: deform_mesh_world(ob, 'rigid')
    else: missing.append(name)
ob = bpy.data.objects.get('GEO_CHASSIS_STEERCOLUM')
if ob: deform_mesh_world(ob, 'steer_column')
else: missing.append('GEO_CHASSIS_STEERCOLUM')
ob = bpy.data.objects.get('GEO_CHASSIS_FLOOR')
if ob: deform_mesh_world(ob, 'floor')
else: missing.append('GEO_CHASSIS_FLOOR')

# Move only the driver reference datum. Axle centers, wheel joints and wing mounts stay fixed.
head = bpy.data.objects.get('JNT_DRIVER_HEAD')
if head:
    w = head.matrix_world.translation.copy()
    w.y -= SHIFT
    m = head.matrix_world.copy()
    m.translation = w
    head.matrix_world = m

# Save editable reworked blend.
bpy.ops.wm.save_as_mainfile(filepath=str(OUT_BLEND))

# Export chassis + interface datums, leaving wheel assets separate and unchanged.
for o in bpy.context.scene.objects:
    o.select_set(False)
export_names = {
    'DATUM_VEHICLE_ORIGIN', 'DATUM_FRONT_AXLE_CENTER', 'DATUM_REAR_AXLE_CENTER',
    'JNT_WHEEL_FL', 'JNT_WHEEL_FR', 'JNT_WHEEL_RL', 'JNT_WHEEL_RR',
    'JNT_DRIVER_HEAD', 'JNT_FRONT_WING_MOUNT', 'JNT_REAR_WING_MOUNT'
}
for o in bpy.context.scene.objects:
    if o.name.startswith('GEO_CHASSIS_') or o.name in export_names:
        o.select_set(True)

bpy.ops.export_scene.gltf(
    filepath=str(OUT_GLB),
    export_format='GLB',
    use_selection=True,
    export_apply=True,
)
shutil.copy2(OUT_GLB, OUT_GLB_CANON)

print(f'[2010-rb6] cockpit shift={SHIFT:.3f} m rearward')
print(f'[2010-rb6] sidepod outboard max={SIDEPOD_OUTBOARD:.3f} m/side')
print(f'[2010-rb6] floor outboard max={FLOOR_OUTBOARD:.3f} m/side')
print(f'[2010-rb6] saved: {OUT_BLEND}')
print(f'[2010-rb6] exported: {OUT_GLB}')
print(f'[2010-rb6] canonical copy: {OUT_GLB_CANON}')
if missing:
    print('[2010-rb6] missing objects:', ', '.join(missing))
