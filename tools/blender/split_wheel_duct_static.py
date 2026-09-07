"""Isolate the brake-scoop shell out of GEO_WHEEL_HUB; export spin + ducts.

Run: blender --background --factory-startup --python tools/blender/split_wheel_duct_static.py
Reads the current canonical wheel GLBs (never its own output) and the
pre-existing topology. Only filters polygons; kept vertices, UVs, normals and
materials stay exact.

The hub mesh contains rim shells (rotationally symmetric, they spin with the
rim) plus, on the front axle, one disconnected brake-scoop shell holding all
asymmetry. Only the scoop goes static. Duct L variants are an exact X-mirror
at GLB buffer level (x->-x, nx->-nx, flipped winding), so the intake stays
forward (+Y Blender / -Z Godot) with the inboard side correct per corner.
The rear axle models no scoop: its hub spins whole and rear DuctStatic nodes
stay empty by design.

Outputs in game/assets/models/vehicles/f1-2026-2008/:
  f1_2026_2008_wheel_{front,rear}_spin.glb   (GEO_WHEEL_HUB spin part + TIRE)
  f1_2026_2008_duct_{FL,FR}.glb              (GEO_WHEEL_DUCT_STATIC scoop only)
Legacy f1_2026_2008_wheel_{front,rear}.glb files are left untouched.
"""
import bpy
import bmesh
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from glb_util import mirror_duct_r_to_l

ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / 'game/assets/models/vehicles/f1-2026-2008'
REPORT = ROOT / 'reports/brake-duct-split'
REPORT.mkdir(parents=True, exist_ok=True)

AXLES = ('front', 'rear')


def write_text(path, text):
    tmp = path.with_name(path.name + '.tmp')
    tmp.write_text(text, encoding='utf-8', newline='\n')
    tmp.replace(path)


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load_fresh(path):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(path))
    bpy.context.view_layer.update()
    obs = list(set(bpy.data.objects) - before)
    # Strip Blender's .001-style suffixes so contract names are stable.
    for o in obs:
        base = o.name
        while True:
            stem, dot, tail = base.rpartition('.')
            if dot and len(tail) == 3 and tail.isdigit():
                base = stem
            else:
                break
        o.name = base
    return obs


def find(obs, name):
    for o in obs:
        if o.name == name:
            return o
    raise KeyError(f'missing object {name}')


def _weld_components(mesh):
    """Union-find components welding exact-duplicate positions (UV seams)."""
    verts = mesh.vertices
    parent = list(range(len(verts)))

    def root(i):
        while parent[i] != i:
            parent[i] = parent[parent[i]]
            i = parent[i]
        return i

    seen = {}
    for v in verts:
        key = (round(v.co.x, 5), round(v.co.y, 5), round(v.co.z, 5))
        if key in seen:
            a, b = root(v.index), root(seen[key])
            if a != b:
                parent[a] = b
        else:
            seen[key] = v.index
    for p in mesh.polygons:
        base = root(p.vertices[0])
        for i in p.vertices[1:]:
            a, b = base, root(i)
            if a != b:
                parent[a] = b
    comp_of = [root(i) for i in range(len(verts))]
    return comp_of


def _asymm_vert_set(mesh, rounding=3):
    seen = set()
    for v in mesh.vertices:
        seen.add((round(v.co.x, rounding), round(v.co.y, rounding), round(v.co.z, rounding)))
    bad = set()
    for v in mesh.vertices:
        if (round(v.co.x, rounding), round(-v.co.y, rounding), round(-v.co.z, rounding)) not in seen:
            bad.add(v.index)
    return bad


def isolate_scoop(hub_obj):
    """Return the face indices of the brake-scoop shell.

    The scoop is the disconnected shell holding (nearly) all vertices that
    lack a rotational counterpart around the axle. Rim shells are symmetric.
    Returns an empty set when the hub has no scoop (e.g. rear axle).
    """
    src = hub_obj.data
    comp_of = _weld_components(src)
    bad = _asymm_vert_set(src)
    from collections import defaultdict
    comp_verts = defaultdict(list)
    for i, c in enumerate(comp_of):
        comp_verts[c].append(i)
    if not bad:
        return set(), {'components': len(comp_verts), 'asymm_total': 0, 'scoop': None}
    owner_count = defaultdict(int)
    for i in bad:
        owner_count[comp_of[i]] += 1
    scoop_comp = max(owner_count, key=lambda c: owner_count[c])
    info = {
        'components': len(comp_verts),
        'asymm_total': len(bad),
        'scoop_asymm': owner_count[scoop_comp],
        'scoop_verts': len(comp_verts[scoop_comp]),
    }
    assert owner_count[scoop_comp] >= 0.9 * len(bad), info
    assert len(comp_verts[scoop_comp]) < 0.3 * len(src.vertices), info
    scoop_faces = set()
    for p in src.polygons:
        if comp_of[p.vertices[0]] == scoop_comp:
            scoop_faces.add(p.index)
    info['scoop_polys'] = len(scoop_faces)
    return scoop_faces, info


def split_hub(hub_obj, scoop_faces):
    """Return (spin_mesh, duct_mesh or None); scoop faces go static."""
    src = hub_obj.data
    bm_spin = bmesh.new()
    bm_spin.from_mesh(src)
    bm_spin.faces.ensure_lookup_table()
    bmesh.ops.delete(
        bm_spin,
        geom=[f for i, f in enumerate(bm_spin.faces) if i in scoop_faces],
        context='FACES',
    )
    spin_me = bpy.data.meshes.new(hub_obj.data.name)
    bm_spin.to_mesh(spin_me)
    bm_spin.free()

    duct_me = None
    if scoop_faces:
        bm_duct = bmesh.new()
        bm_duct.from_mesh(src)
        bm_duct.faces.ensure_lookup_table()
        bmesh.ops.delete(
            bm_duct,
            geom=[f for i, f in enumerate(bm_duct.faces) if i not in scoop_faces],
            context='FACES',
        )
        duct_me = bpy.data.meshes.new('GEO_WHEEL_DUCT_STATIC_MESH')
        bm_duct.to_mesh(duct_me)
        bm_duct.free()
    return spin_me, duct_me


def export_selected(obs, path):
    bpy.ops.object.select_all(action='DESELECT')
    for o in obs:
        o.select_set(True)
    bpy.context.view_layer.objects.active = next(o for o in obs if o.type == 'MESH')
    tmp = REPORT / path.name
    bpy.ops.export_scene.gltf(
        filepath=str(tmp),
        export_format='GLB',
        use_selection=True,
        export_yup=True,
        export_texcoords=True,
        export_normals=True,
        export_materials='EXPORT',
        export_cameras=False,
        export_lights=False,
    )
    tmp.replace(path)


summary = {'axles': {}}
for axle in AXLES:
    src = ASSETS / f'f1_2026_2008_wheel_{axle}.glb'
    obs = load_fresh(src)
    hub = find(obs, 'GEO_WHEEL_HUB')
    tire = find(obs, 'GEO_WHEEL_TIRE')
    src_tris = len(hub.data.polygons)

    scoop_faces, scoop_info = isolate_scoop(hub)
    if axle == 'front':
        assert scoop_faces, 'front hub lost its brake scoop'
    else:
        assert not scoop_faces, scoop_info
    spin_me, duct_me = split_hub(hub, scoop_faces)
    spin_count = len(spin_me.polygons)
    duct_count = len(duct_me.polygons) if duct_me is not None else 0
    assert spin_count + duct_count == src_tris, (spin_count, duct_count, src_tris)

    hub.data = spin_me
    hub.data.update()

    mount = next((o for o in obs if o.name == 'JNT_WHEEL_MOUNT'), None)
    datums = [o for o in obs if o.name.startswith('DATUM_')]
    spin_path = ASSETS / f'f1_2026_2008_wheel_{axle}_spin.glb'
    export_selected([hub, tire] + datums + ([mount] if mount else []), spin_path)

    entry = {
        'source_tris': src_tris,
        'spin_tris': spin_count,
        'scoop': scoop_info,
        'spin_glb': spin_path.name,
        'sha256': {spin_path.name: sha(spin_path)},
    }
    if duct_me is not None:
        duct_obj = bpy.data.objects.new('GEO_WHEEL_DUCT_STATIC', duct_me)
        bpy.context.collection.objects.link(duct_obj)
        metal = hub.material_slots[0].material if len(hub.material_slots) else None
        if metal is not None:
            duct_obj.data.materials.append(metal)

        duct_r_path = ASSETS / f'f1_2026_2008_duct_{"FR" if axle == "front" else "RR"}.glb'
        export_selected([duct_obj] + datums + ([mount] if mount else []), duct_r_path)

        duct_l_path = ASSETS / f'f1_2026_2008_duct_{"FL" if axle == "front" else "RL"}.glb'
        mirror_duct_r_to_l(duct_r_path, duct_l_path)
        entry.update({
            'duct_tris': duct_count,
            'duct_r_glb': duct_r_path.name,
            'duct_l_glb': duct_l_path.name,
        })
        entry['sha256'].update({p.name: sha(p) for p in (duct_r_path, duct_l_path)})
    summary['axles'][axle] = entry

write_text(REPORT / 'split.json', json.dumps(summary, indent=2) + '\n')
print('DUCT_SPLIT ' + json.dumps(summary))
