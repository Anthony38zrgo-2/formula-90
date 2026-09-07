"""Validate scoop/static partition: spin symmetry, scoop forward + inboard.

Run: blender --background --factory-startup --python-exit-code 1 --python tools/blender/validate_duct_split.py
Reimports the actual new GLBs and checks geometry in Blender space
(X lateral/right, Y forward, Z up). Fails the process on any violation.

Design: only the disconnected brake-scoop shell is static (front axle,
144 tris). Rim shells spin. The rear axle models no scoop: its hub spins
whole and rear DuctStatic scene nodes stay empty by design.
"""
import bpy
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / 'game/assets/models/vehicles/f1-2026-2008'
REPORT = ROOT / 'reports/brake-duct-split'

LEGACY_TRIS = {'front': 5898, 'rear': 5754}
SCOOP_TRIS = 144
SCOOP_VERTS = 105

failures = []


def check(condition, message):
    if not condition:
        failures.append(message)
        print('[FAIL] ' + message)


def load(path):
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(path))
    bpy.context.view_layer.update()
    return list(set(bpy.data.objects) - before)


def unload(obs):
    for o in list(obs):
        try:
            bpy.data.objects.remove(o, do_unlink=True)
        except ReferenceError:
            pass


def rot_misses(obj, rounding=3):
    vs = [(v.co.x, v.co.y, v.co.z) for v in obj.data.vertices]
    seen = set((round(x, rounding), round(y, rounding), round(z, rounding)) for x, y, z in vs)
    return [(x, y, z) for x, y, z in vs
            if (round(x, rounding), round(-y, rounding), round(-z, rounding)) not in seen]


def centroid(points):
    n = len(points)
    return (sum(p[0] for p in points) / n, sum(p[1] for p in points) / n, sum(p[2] for p in points) / n)


result = {}

# Spin files: symmetric HUB+TIRE, no duct meshes.
for axle in ('front', 'rear'):
    spin_file = f'f1_2026_2008_wheel_{axle}_spin.glb'
    obs = load(ASSETS / spin_file)
    names = sorted(o.name for o in obs if o.type == 'MESH')
    check(any('HUB' in n for n in names), f'{spin_file}: missing spin HUB, got {names}')
    check(any('TIRE' in n for n in names), f'{spin_file}: missing TIRE, got {names}')
    check(not any('DUCT' in n for n in names), f'{spin_file}: duct leaked into spin file')
    hub = next(o for o in obs if o.type == 'MESH' and 'HUB' in o.name)
    tire = next(o for o in obs if o.type == 'MESH' and 'TIRE' in o.name)
    check(len(rot_misses(hub)) == 0, f'{spin_file}: spin HUB not symmetric')
    check(len(rot_misses(tire)) == 0, f'{spin_file}: TIRE not symmetric')
    spin_tris = len(hub.data.polygons)
    expected = LEGACY_TRIS[axle] - (SCOOP_TRIS if axle == 'front' else 0)
    check(spin_tris == expected, f'{spin_file}: {spin_tris} tris, expected {expected}')
    unload(obs)
    result[axle] = {'spin_tris': spin_tris}

# Front ducts: scoop shell only, intake forward, inboard side per corner.
duct_info = {}
for side, inboard_sign in (('FR', -1.0), ('FL', 1.0)):
    obs = load(ASSETS / f'f1_2026_2008_duct_{side}.glb')
    dnames = [o.name for o in obs if o.type == 'MESH']
    check(len(dnames) == 1 and 'DUCT' in dnames[0],
          f'duct_{side}: expected single DUCT mesh, got {dnames}')
    duct = next(o for o in obs if o.type == 'MESH')
    check(len(duct.data.polygons) == SCOOP_TRIS,
          f'duct_{side}: {len(duct.data.polygons)} tris, expected {SCOOP_TRIS}')
    check(len(duct.data.vertices) == SCOOP_VERTS,
          f'duct_{side}: {len(duct.data.vertices)} verts, expected {SCOOP_VERTS}')
    scoop = rot_misses(duct)
    check(len(scoop) == SCOOP_VERTS, f'duct_{side}: scoop must be fully asymmetric')
    if scoop:
        _sx, sy, _sz = centroid(scoop)
        check(sy > 0.03, f'duct_{side}: scoop intake not forward, y={sy:.4f}')
    vs = [(v.co.x, v.co.y, v.co.z) for v in duct.data.vertices]
    cx, _cy, _cz = centroid(vs)
    check((cx > 0) == (inboard_sign > 0),
          f'duct_{side}: centroid x={cx:.4f} on wrong lateral side')
    outer = max(v[0] * inboard_sign for v in vs)
    check(outer > 0.20, f'duct_{side}: scoop outer |x|={outer:.4f}, expected protruding scoop')
    duct_info[side] = {'tris': len(duct.data.polygons), 'centroid': [cx, sy, _sz] if scoop else [cx, 0, 0]}
    unload(obs)

check(result['front']['spin_tris'] + duct_info['FR']['tris'] == LEGACY_TRIS['front'],
      'front tri conservation broken')
result['front']['ducts'] = duct_info

REPORT.mkdir(parents=True, exist_ok=True)
(REPORT / 'validate.json').write_text(json.dumps(result, indent=2) + '\n')
print('DUCT_VALIDATE ' + json.dumps(result))
if failures:
    print(f'DUCT_VALIDATE_FAILED {len(failures)} failure(s)')
    raise SystemExit(1)
print('DUCT_VALIDATE_PASS')
