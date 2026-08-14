from pathlib import Path
import shutil, json, struct, hashlib, zipfile
import trimesh
from PIL import Image
from trimesh.visual.material import PBRMaterial

SRC_ROOT = Path('/mnt/data/F1_94_GEVP_READY')
SRC_ASSEMBLY = SRC_ROOT / 'F1_94_GEVP_READY.glb'
SRC_CHASSIS = SRC_ROOT / 'F1_94_chassis.glb'
SRC_WHEEL_FRONT = SRC_ROOT / 'F1_94_wheel_front.glb'
SRC_WHEEL_REAR = SRC_ROOT / 'F1_94_wheel_rear.glb'
SRC_ALBEDO = Path('/mnt/data/texture(1).png')
OUT = Path('/mnt/data/F1_94_DECOUPLED')
ZIP = Path('/mnt/data/F1_94_DECOUPLED.zip')

if OUT.exists():
    shutil.rmtree(OUT)
OUT.mkdir(parents=True)
(OUT/'geometry').mkdir()
(OUT/'textures'/'albedo').mkdir(parents=True)
(OUT/'textures'/'source').mkdir(parents=True)
(OUT/'godot').mkdir()
(OUT/'docs').mkdir()


def sha256(path: Path):
    h = hashlib.sha256()
    with path.open('rb') as f:
        for chunk in iter(lambda: f.read(1<<20), b''):
            h.update(chunk)
    return h.hexdigest()


def glb_json(path: Path):
    with path.open('rb') as f:
        magic, version, length = struct.unpack('<4sII', f.read(12))
        if magic != b'glTF':
            raise ValueError(f'{path} is not GLB')
        while f.tell() < length:
            clen, ctype = struct.unpack('<II', f.read(8))
            data = f.read(clen)
            if ctype == 0x4E4F534A:
                return json.loads(data.rstrip(b'\x00 ').decode('utf-8'))
    raise ValueError('No JSON chunk')


def strip_textures(src: Path, dst: Path, root_name=None):
    scene = trimesh.load(src, force='scene', process=False)
    for name, geom in scene.geometry.items():
        # Keep geometry/UV/normals, replace only material payload with a texture-free placeholder.
        geom.visual.material = PBRMaterial(
            name=f'MAT_{name}',
            baseColorFactor=[255,255,255,255],
            metallicFactor=0.0,
            roughnessFactor=1.0,
            doubleSided=False,
        )
    scene.export(dst)
    return scene

assembly = strip_textures(SRC_ASSEMBLY, OUT/'geometry'/'F1_94_geometry.glb')
chassis = strip_textures(SRC_CHASSIS, OUT/'geometry'/'F1_94_chassis_geometry.glb')
wheel_front = strip_textures(SRC_WHEEL_FRONT, OUT/'geometry'/'F1_94_wheel_front_geometry.glb')
wheel_rear = strip_textures(SRC_WHEEL_REAR, OUT/'geometry'/'F1_94_wheel_rear_geometry.glb')

# Copy source atlas once for auditing/reference.
shutil.copy2(SRC_ALBEDO, OUT/'textures'/'source'/'F1_94_source_albedo.png')

# One external albedo file per mesh object. Deliberately preserve the exact 64x64 atlas layout
# and the original UV coordinates. This avoids any UV repack/remap risk.
all_mesh_names = sorted(set(assembly.geometry) | set(chassis.geometry) | set(wheel_front.geometry) | set(wheel_rear.geometry))
for name in all_mesh_names:
    shutil.copy2(SRC_ALBEDO, OUT/'textures'/'albedo'/f'{name}.png')

# Manifest for deterministic Godot assignment.
def classify(name: str):
    if name.startswith('GEO_WHEEL_'): return 'wheel'
    if name.startswith('GEO_SUSPENSION_'): return 'suspension'
    if name.startswith('GEO_AERO_FRONT_WING'): return 'front_wing'
    if name.startswith('GEO_AERO_REAR_WING'): return 'rear_wing'
    if name.startswith('GEO_COCKPIT_'): return 'cockpit'
    if name.startswith('GEO_DRIVER_'): return 'driver'
    if name == 'GEO_NOSE': return 'nose'
    if name in ('GEO_CHASSIS','GEO_BODY_INTERIOR'): return 'chassis'
    return 'other'

manifest = {
    'asset': 'F1_94',
    'version': 1,
    'standard': 'Formula-90 GEVP decoupled visual asset',
    'coordinate_convention': {
        '+X': 'vehicle right', '-X': 'vehicle left', '+Y': 'up', '-Z': 'front', '+Z': 'rear', 'unit': 'meter'
    },
    'decoupling_strategy': {
        'geometry_contains': ['positions','triangles','normals','UV0','hierarchy','JNT_* pivots','DATUM_* references','texture-free placeholder materials'],
        'geometry_does_not_contain': ['embedded images','embedded albedo textures','custom Godot shaders'],
        'albedo': 'one external PNG per mesh; original 64x64 atlas layout preserved; UV0 unchanged',
        'reason': 'avoid material/shader/texture embedding issues without introducing UV repack risk'
    },
    'geometry_assets': {
        'assembly': 'geometry/F1_94_geometry.glb',
        'chassis_gevp': 'geometry/F1_94_chassis_geometry.glb',
        'wheel_front_canonical': 'geometry/F1_94_wheel_front_geometry.glb',
        'wheel_rear_canonical': 'geometry/F1_94_wheel_rear_geometry.glb'
    },
    'meshes': [],
}

for name in sorted(assembly.geometry.keys()):
    g = assembly.geometry[name]
    uv = getattr(g.visual, 'uv', None)
    manifest['meshes'].append({
        'name': name,
        'role': classify(name),
        'material_slot': f'MAT_{name}',
        'albedo': f'textures/albedo/{name}.png',
        'vertices': int(len(g.vertices)),
        'triangles': int(len(g.faces)),
        'has_uv0': bool(uv is not None),
    })

(OUT/'manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')

# Godot helper script: attach to a parent of the imported GLB, or call apply_to(imported_root).
gd = r'''extends Node
class_name F194ExternalAlbedoBinder

@export_dir var albedo_root: String = "res://assets/vehicles/F1_94/textures/albedo"

func _ready() -> void:
    apply_to(self)

func apply_to(root: Node) -> void:
    for node in root.find_children("GEO_*", "MeshInstance3D", true, false):
        _bind_mesh(node as MeshInstance3D)

func _bind_mesh(mesh_instance: MeshInstance3D) -> void:
    var texture_path := albedo_root.path_join(mesh_instance.name + ".png")
    if not ResourceLoader.exists(texture_path):
        push_warning("F1_94: missing external albedo for " + mesh_instance.name + ": " + texture_path)
        return

    var material := StandardMaterial3D.new()
    material.resource_name = "MAT_" + mesh_instance.name
    material.albedo_texture = load(texture_path)
    material.texture_filter = BaseMaterial3D.TEXTURE_FILTER_NEAREST
    material.metallic = 0.0
    material.roughness = 1.0
    material.cull_mode = BaseMaterial3D.CULL_BACK
    material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA_SCISSOR
    material.alpha_scissor_threshold = 0.5
    mesh_instance.material_override = material
'''
(OUT/'godot'/'f1_94_external_albedo_binder.gd').write_text(gd, encoding='utf-8')

readme = '''# F1_94 — paquete desacoplado para Godot / GEVP

Este paquete separa la geometría del color visible.

## Regla

- Los `.glb` contienen geometría, normales, UV0, jerarquía, pivotes `JNT_*` y referencias `DATUM_*`.
- Los `.glb` NO contienen imágenes ni texturas albedo embebidas.
- Cada `GEO_*` tiene un PNG externo en `textures/albedo/<GEO_NAME>.png`.
- Los UV NO fueron repaquetados ni modificados. Cada PNG conserva exactamente el layout 64×64 del atlas original. Esto prioriza seguridad y evita introducir nuevos errores UV.
- Los materiales placeholder del GLB son blancos, simples, sin textura y se llaman `MAT_<GEO_NAME>`.

## Asset principal

`geometry/F1_94_geometry.glb`

## Assets GEVP

- `geometry/F1_94_chassis_geometry.glb`
- `geometry/F1_94_wheel_front_geometry.glb`
- `geometry/F1_94_wheel_rear_geometry.glb`

Las ruedas canónicas mantienen su origen en el centro de rotación.

## Godot

`godot/f1_94_external_albedo_binder.gd` asigna automáticamente el PNG externo cuyo nombre coincide con cada nodo `GEO_*` y fuerza `Nearest`, roughness 1.0, metallic 0 y alpha scissor.

Ajusta `albedo_root` a la carpeta `textures/albedo` dentro de tu proyecto.

## Importante

No se hizo UV painting nuevo. No se horneó color en vértices. No se repaquetaron UVs. La asociación es estrictamente:

`GEO_* -> UV0 del GLB -> PNG externo del mismo nombre`.
'''
(OUT/'README.md').write_text(readme, encoding='utf-8')

# Validation report
validation = {'glbs': {}, 'albedo_files': 0, 'errors': []}
for p in sorted((OUT/'geometry').glob('*.glb')):
    j = glb_json(p)
    s = trimesh.load(p, force='scene', process=False)
    entry = {
        'images': len(j.get('images', [])),
        'textures': len(j.get('textures', [])),
        'materials': len(j.get('materials', [])),
        'meshes': len(j.get('meshes', [])),
        'nodes': len(j.get('nodes', [])),
        'all_meshes_have_uv0': all(getattr(g.visual, 'uv', None) is not None for g in s.geometry.values()),
        'sha256': sha256(p),
    }
    validation['glbs'][p.name] = entry
    if entry['images'] != 0 or entry['textures'] != 0:
        validation['errors'].append(f'{p.name}: embedded images/textures found')
    if not entry['all_meshes_have_uv0']:
        validation['errors'].append(f'{p.name}: mesh without UV0')

albedos = list((OUT/'textures'/'albedo').glob('*.png'))
validation['albedo_files'] = len(albedos)
validation['source_albedo'] = {
    'size': list(Image.open(SRC_ALBEDO).size),
    'mode': Image.open(SRC_ALBEDO).mode,
    'sha256': sha256(SRC_ALBEDO),
}
(OUT/'docs'/'validation.json').write_text(json.dumps(validation, indent=2), encoding='utf-8')

# Copy builder for reproducibility.
shutil.copy2('/mnt/data/build_f1_94_decoupled.py', OUT/'docs'/'build_f1_94_decoupled.py')

# Zip package.
if ZIP.exists(): ZIP.unlink()
with zipfile.ZipFile(ZIP, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as z:
    for p in sorted(OUT.rglob('*')):
        if p.is_file():
            z.write(p, p.relative_to(OUT.parent))

print(json.dumps({
    'package': str(ZIP),
    'folder': str(OUT),
    'assembly_meshes': len(assembly.geometry),
    'external_albedos': len(albedos),
    'validation_errors': validation['errors'],
    'geometry_validation': validation['glbs'],
}, indent=2))
