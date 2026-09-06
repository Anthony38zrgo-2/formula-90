"""Restore pre-remodel aero/chassis; 3500 mm wheelbase, 2000 mm tyre envelope.
Run in Blender. Reads the immutable pre-remodel backup, never its own output.
"""
import bpy,json,math,hashlib,re
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[2]
ASSETS=ROOT/'game/assets/models/vehicles/f1-2026-2008'
BACKUP=Path('D:/Formula90s-backups/f1_2026_2008-jordan-remodel')
REPORT=ROOT/'reports/2026-3500-jordan'
REPORT.mkdir(parents=True,exist_ok=True)
ATLAS=ASSETS/'textures/jordan_1997_unbranded_atlas.png'
SCALE=3.5/3.23
def longitudinal(y): return y*SCALE if abs(y)<=1.615 else y+math.copysign(.135,y)
def write_text(p,text):
    temp=p.with_name(p.name+'.tmp'); temp.write_text(text,encoding='utf-8',newline='\n'); temp.replace(p)
def save(p,o): write_text(p,json.dumps(o,indent=2)+'\n')
def load(p):
    before=set(bpy.data.objects); bpy.ops.import_scene.gltf(filepath=str(p)); bpy.context.view_layer.update()
    return list(set(bpy.data.objects)-before)
def export(obs,path):
    bpy.ops.object.select_all(action='DESELECT')
    for o in obs: o.select_set(True)
    temp=REPORT/path.name
    bpy.ops.export_scene.gltf(filepath=str(temp),export_format='GLB',use_selection=True,
        export_yup=True,export_texcoords=True,export_normals=True,export_materials='EXPORT',export_cameras=False,export_lights=False)
    temp.replace(path)

bpy.ops.wm.read_factory_settings(use_empty=True)
chassis=load(BACKUP/'f1-2026-2008/f1_2026_2008_chassis.glb')
original_points={o.name:[tuple(v.co) for v in o.data.vertices] for o in chassis if o.type=='MESH'}
img=bpy.data.images.load(str(ATLAS)); img.pack()
yellow=bpy.data.materials['MAT_LIVERY_YELLOW']
red=bpy.data.materials['MAT_LIVERY_RED']
black=bpy.data.materials['MAT_LIVERY_BLACK']
for mat,rgba,rough in [(yellow,(1,.8,.001,1),.22),(red,(.82,.004,.001,1),.24),(black,(.006,.008,.009,1),.32)]:
    shader=next(n for n in mat.node_tree.nodes if n.type=='BSDF_PRINCIPLED')
    shader.inputs['Base Color'].default_value=rgba; shader.inputs['Roughness'].default_value=rough
    mat.diffuse_color=rgba
import sys
sys.path.insert(0,str(Path(__file__).resolve().parent))
from bake_jordan_livery import apply as bake_livery
body=next(o for o in chassis if o.name=='GEO_CHASSIS_BODY')
bake_livery(body,img,ASSETS/'textures/jordan_1997_body_albedo.png')

# Stretch only between axle planes; translate the original overhang geometry.
for o in chassis:
    if o.type=='MESH':
        for vertex in o.data.vertices: vertex.co.y=longitudinal(vertex.co.y)
        o.data.update()
    else: o.location.y=longitudinal(o.location.y)
for name,points in original_points.items():
    o=bpy.data.objects[name]
    assert len(points)==len(o.data.vertices)
    assert all(abs(a[0]-b.co.x)<1e-7 and abs(a[2]-b.co.z)<1e-7 and abs(longitudinal(a[1])-b.co.y)<3e-7 for a,b in zip(points,o.data.vertices))
export(chassis,ASSETS/'f1_2026_2008_chassis.glb')

wheel_sets={}
for part,ratio in [('front',.365/.355),('rear',.410/.400)]:
    obs=load(BACKUP/f'f1-2026-2008/f1_2026_2008_wheel_{part}.glb')
    # Prevent Blender suffixing contract names against the other wheel asset.
    for o in obs:
        o.name=re.sub(r'\.\d{3}$','',o.name)
        if o.type=='MESH':
            for v in o.data.vertices: v.co.x*=ratio
            o.data.update()
        else: o.location.x*=ratio
    export(obs,ASSETS/f'f1_2026_2008_wheel_{part}.glb')
    # Reload for the editable assembly after exporting clean per-wheel contracts.
    for o in obs: bpy.data.objects.remove(o,do_unlink=True)

for axle,half,y in [('front',.8175,1.75),('rear',.795,-1.75)]:
    for side,label in [(-1,'L'),(1,'R')]:
        obs=load(ASSETS/f'f1_2026_2008_wheel_{axle}.glb')
        parent=bpy.data.objects.new(f'WHEEL_{axle.upper()}_{label}_CONTROL',None); bpy.context.collection.objects.link(parent)
        parent.location=(side*half,y,0); parent.rotation_euler.z=math.pi if side<0 else 0
        for o in obs: o.parent=parent
bpy.context.view_layer.update()
bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/'source/f1_2026_2008_source.blend'))

physics=json.loads((BACKUP/'f1_2026_2008_physics.json').read_text())
physics['geometry']['wheelbase']=3.5
physics['tires']['front']['width']=.365; physics['tires']['rear']['width']=.410
save(ROOT/'game/data/vehicles/f1_2026_2008/f1_2026_2008_physics.json',physics)

scene=(BACKUP/'f1_2026_2008_rust.tscn').read_text()
sections=re.split(r'(?=\[node )',scene)
# Restore the three original chassis boxes, extending their longitudinal bounds.
boxes={'CollisionTub':('Box_tub',2.9),'CollisionNose':('Box_nose',1.6),'CollisionRear':('Box_rear',1.1)}
for i,s in enumerate(sections):
    m=re.search(r'position = Vector3\(([^)]+)\)',s)
    if not m: continue
    xyz=list(map(float,m[1].split(',')))
    node=re.search(r'\[node name="([^"]+)"',s)[1]
    if node in boxes:
        box,span=boxes[node]; low=longitudinal(xyz[2]-span/2); high=longitudinal(xyz[2]+span/2)
        xyz[2]=(low+high)/2
        sections[0]=re.sub(r'(\[sub_resource type="BoxShape3D" id="'+box+r'"\]\nsize = Vector3\([^,]+, [^,]+, )[^)]+',lambda m:m[1]+format(high-low,'.9f'),sections[0])
    else: xyz[2]=longitudinal(xyz[2])
    if node.startswith('RayCast_'):
        axle=node[8]; side=node[9]; suffix=node.split('_')[-1]
        half,width=(.8175,.365) if axle=='F' else (.795,.410)
        offset={'In':-width*.4,'Mid':0,'Out':width*.4}[suffix]
        xyz[0]=(-1 if side=='L' else 1)*(half+offset)
    sections[i]=s[:m.start(1)]+', '.join(format(c,'.9g') for c in xyz)+s[m.end(1):]
write_text(ROOT/'game/scenes/vehicles/f1_2026_2008/f1_2026_2008_rust.tscn',''.join(sections))
save(REPORT/'build.json',{'original_backup':str(BACKUP),'wheelbase_m':3.5,'front_track_m':1.635,
    'rear_track_m':1.590,'front_tire_width_m':.365,'rear_tire_width_m':.410,'front_total_m':2.,'rear_total_m':2.,
    'geometry':'Original pre-remodel chassis and wings; longitudinal extension only; wider wheels; baked opaque livery.',
    'original_chassis_mesh_count':len(original_points),'livery_atlas':str(ATLAS)})
print('RESTORE_3500_COMPLETE')
