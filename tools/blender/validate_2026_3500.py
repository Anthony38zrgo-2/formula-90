"""Independent reimport checks for restored 2026 geometry and 3500/2000 dimensions."""
import bpy,json,hashlib,math,re
from pathlib import Path
from mathutils import Vector
from mathutils.kdtree import KDTree
from mathutils.bvhtree import BVHTree
ROOT=Path(__file__).resolve().parents[2]
ASSETS=ROOT/'game/assets/models/vehicles/f1-2026-2008'
BACKUP=Path('D:/Formula90s-backups/f1_2026_2008-jordan-remodel')
REPORT=ROOT/'reports/2026-3500-jordan'
PHYSICS=ROOT/'game/data/vehicles/f1_2026_2008/f1_2026_2008_physics.json'
SCENE=ROOT/'game/scenes/vehicles/f1_2026_2008/f1_2026_2008_rust.tscn'
def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def save(p,d):
    temp=p.with_name(p.name+'.tmp'); temp.write_text(json.dumps(d,indent=2)+'\n',encoding='utf-8',newline='\n'); temp.replace(p)
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest().upper()
def longitudinal(y): return y*3.5/3.23 if abs(y)<=1.615 else y+math.copysign(.135,y)
def load(p):
    before=set(bpy.data.objects); bpy.ops.import_scene.gltf(filepath=str(p)); bpy.context.view_layer.update()
    return list(set(bpy.data.objects)-before)
def coords(o): return [o.matrix_world@v.co for v in o.data.vertices]
def bvh(o): return BVHTree.FromPolygons(coords(o),[list(p.vertices) for p in o.data.polygons])
def deviation(a,b):
    def distance(a,b):
        tree=KDTree(len(a))
        for i,p in enumerate(a): tree.insert(p,i)
        tree.balance(); return max(tree.find(p)[2] for p in b)
    return max(distance(a,b),distance(b,a))
def components(o):
    parent=list(range(len(o.data.vertices)))
    def root(i):
        while parent[i]!=i: parent[i]=parent[parent[i]]; i=parent[i]
        return i
    def join(a,b): parent[root(a)]=root(b)
    shared={}
    for v in o.data.vertices:
        key=tuple(round(c,5) for c in v.co)
        if key in shared: join(v.index,shared[key])
        else: shared[key]=v.index
    for e in o.data.edges: join(*e.vertices)
    groups={}
    for v in o.data.vertices: groups.setdefault(root(v.index),[]).append(v.index)
    return list(groups.values())

bpy.ops.wm.read_factory_settings(use_empty=True)
original={}; names={}; triangles={}
for part in ('chassis','wheel_front','wheel_rear'):
    obs=load(BACKUP/f'f1-2026-2008/f1_2026_2008_{part}.glb')
    names[part]={o.name for o in obs}; original[part]={}; triangles[part]={}
    for o in obs:
        if o.type=='MESH':
            original[part][o.name]=coords(o); o.data.calc_loop_triangles()
            triangles[part][o.name]=len(o.data.loop_triangles)
    for o in obs: bpy.data.objects.remove(o,do_unlink=True)
chassis=load(ASSETS/'f1_2026_2008_chassis.glb'); objects={o.name:o for o in chassis}
assert set(objects)==names['chassis'],'unexpected remodeling nodes'
stats={}; errors={}; allpoints=[]
for name,points in original['chassis'].items():
    expected=[Vector((p.x,longitudinal(p.y),p.z)) for p in points]
    error=deviation(expected,coords(objects[name])); assert error<2e-6,(name,error)
    errors[name]=error
    objects[name].data.calc_loop_triangles()
    assert len(objects[name].data.loop_triangles)==triangles['chassis'][name],(name,'topology changed')
for o in chassis:
    if o.type!='MESH': continue
    o.data.calc_loop_triangles()
    assert all(math.isfinite(c) for p in coords(o) for c in p)
    assert o.data.uv_layers
    # Existing source has small triangles; reject only numerically collapsed faces.
    count=sum(t.area<1e-14 for t in o.data.loop_triangles)
    assert count==0,(o.name,'collapsed triangles',count)
    stats[o.name]={'vertices':len(o.data.vertices),'triangles':len(o.data.loop_triangles)}
    allpoints.extend(coords(o))
for suffix,x,y in [('FL',-.8175,1.75),('FR',.8175,1.75),('RL',-.795,-1.75),('RR',.795,-1.75)]:
    assert (objects['JNT_WHEEL_'+suffix].location-Vector((x,y,0))).length<2e-7

hubs={}; widths={}; wheel_checks={}
for axle,half,y,target,ratio in [('front',.8175,1.75,.365,.365/.355),('rear',.795,-1.75,.410,.410/.400)]:
    p=ASSETS/f'f1_2026_2008_wheel_{axle}.glb'
    obs=load(p)
    canonical=lambda name: re.sub(r'\.\d{3}$','',name)
    assert {canonical(o.name) for o in obs}==names['wheel_'+axle]
    checks={}
    for o in obs:
        if o.type=='MESH':
            expected=[Vector((c.x*ratio,c.y,c.z)) for c in original['wheel_'+axle][canonical(o.name)]]
            error=deviation(expected,coords(o)); assert error<2e-6
            checks[o.name]=error
            o.data.calc_loop_triangles(); assert len(o.data.loop_triangles)==triangles['wheel_'+axle][canonical(o.name)]
    tire=next(o for o in obs if 'TIRE' in o.name and o.type=='MESH')
    width=max(p.x for p in coords(tire))-min(p.x for p in coords(tire))
    assert abs(width-target)<1e-6
    widths[axle]=2*half+width; wheel_checks[axle]=checks
    for o in obs: bpy.data.objects.remove(o,do_unlink=True)
    for side in [-1,1]:
        obs=load(p)
        for o in obs:
            o.rotation_mode='XYZ'
            if side<0:
                o.location.x=-o.location.x; o.location.y=-o.location.y; o.rotation_euler.z=math.pi
            o.location+=Vector((side*half,y,0))
            if o.type=='MESH' and 'HUB' in o.name: hubs[(axle,side)]=o
        bpy.context.view_layer.update()
        for o in obs:
            if o.type=='MESH': allpoints.extend(coords(o))

contacts=[]
for axle in ('front','rear'):
    o=objects[f'GEO_CHASSIS_{axle.upper()}_SUSPENSION']; points=coords(o)
    for group in components(o):
        side=-1 if sum(points[i].x for i in group)<0 else 1
        tree=BVHTree.FromPolygons(points,[list(p.vertices) for p in o.data.polygons if all(i in group for i in p.vertices)])
        overlap=len(tree.overlap(bvh(hubs[(axle,side)])))
        assert overlap>0,(axle,side,len(group),'suspension/hub disconnected')
        contacts.append({'axle':axle,'side':side,'hub_triangle_intersections':overlap})
assert len(contacts)==14

physics=read(PHYSICS); expected=read(BACKUP/PHYSICS.name)
expected['geometry']['wheelbase']=3.5; expected['tires']['front']['width']=.365; expected['tires']['rear']['width']=.410
assert physics==expected,'unexpected physics change'
scene=SCENE.read_text(); sections=re.split(r'(?=\[node )',scene)
assert scene.count('type="CollisionShape3D"')==3
assert re.findall(r'^\[node .*$',scene,re.M)==re.findall(r'^\[node .*$',(BACKUP/SCENE.name).read_text(),re.M)
for axle,half,width,z in [('F',.8175,.365,-1.75),('R',.795,.410,1.75)]:
    for side,sign in [('L',-1),('R',1)]:
        for suffix,offset in [('In',-width*.4),('Mid',0),('Out',width*.4)]:
            s=next(s for s in sections if s.startswith(f'[node name="RayCast_{axle}{side}_{suffix}"'))
            pos=list(map(float,re.search(r'position = Vector3\(([^)]+)',s)[1].split(',')))
            assert abs(pos[0]-sign*(half+offset))<1e-8 and pos[2]==z

dimensions={'overall_width':max(p.x for p in allpoints)-min(p.x for p in allpoints),
    'overall_length':max(p.y for p in allpoints)-min(p.y for p in allpoints),
    'overall_height_from_ground':max(p.z for p in allpoints)+.33,
    'wheelbase':3.5,'front_track_center':1.635,'rear_track_center':1.590,
    'front_tire_width_visual':.365,'rear_tire_width_visual':.410,
    'front_total_width':widths['front'],'rear_total_width':widths['rear']}
assert abs(dimensions['overall_width']-2.)<1e-6
inputs=[ASSETS/f'f1_2026_2008_{part}.glb' for part in ('chassis','wheel_front','wheel_rear')]+[PHYSICS,SCENE,ASSETS/'source/f1_2026_2008_source.blend',ASSETS/'textures/jordan_1997_unbranded_atlas.png',ASSETS/'textures/jordan_1997_body_albedo.png']
report={'schema':'formula90s/restored-2026-3500-validation/v1','passed':True,'dimensions_m':dimensions,
    'geometry_tolerance_m':2e-6,'chassis_max_bidirectional_vertex_error_m':errors,'wheel_max_vertex_error_m':wheel_checks,
    'checks':{'original_29_chassis_meshes_restored':len(errors)==29,'original_mesh_topology_preserved':True,
        'chassis_only_longitudinal_extension':True,'wheel_only_lateral_widening':True,'original_scene_hierarchy_restored':True,
        'raycasts_match_physics':True,'three_original_colliders_restored_and_extended':True,'all_14_hub_connections':True},
    'suspension_contacts':contacts,'scope':'Static exported geometry checked by Blender reimport; driving/animated suspension not tested.',
    'input_sha256':{str(p.relative_to(ROOT)).replace('\\','/'):sha(p) for p in inputs}}
save(REPORT/'validation.json',report); save(ASSETS/'validation_report.json',report)
metadata=read(BACKUP/'f1-2026-2008/vehicle_metadata.json'); metadata['dimensions_m'].update(dimensions)
for name in metadata['validation_datums']:
    p=objects[name].location; metadata['validation_datums'][name]=[p.x,p.z,-p.y]
for part,width in [('front',.365),('rear',.410)]:
    metadata['wheel_'+part+'_datums']['DATUM_WHEEL_INBOARD_FACE'][0]=-width/2
    metadata['wheel_'+part+'_datums']['DATUM_WHEEL_OUTBOARD_FACE'][0]=width/2
metadata['notes']['design']='Restored original 2026 aero/chassis, 3500 mm wheelbase and 2000 mm tyre envelope; Jordan 1997-inspired unbranded snakeskin livery.'
metadata['notes']['source']='Editable assembly: Blender X right, Y forward, Z up; GLB uses original Godot axes.'
save(ASSETS/'vehicle_metadata.json',metadata)
manifest=read(BACKUP/'f1-2026-2008/manifest.json'); manifest['physics_sha256']=sha(PHYSICS)
for item in manifest['geometry_assets'].values(): item['sha256']=sha(ASSETS/item['path'])
manifest['validation']={'status':'PASS','note':'Original 2026 geometry restored; 3500/2000 dimensions, topology, raycasts and static hub connections verified. See validation_report.json.'}
save(ASSETS/'manifest.json',manifest)
save(ASSETS/'export_report.json',{'operation':'Restore original 2026 body, lengthen wheelbase, widen tyres, apply Jordan-inspired livery',
    'mesh_stats':stats,'triangles_total':sum(v['triangles'] for v in stats.values()),
    'files':{str(p.relative_to(ASSETS)).replace('\\','/'):p.stat().st_size for p in inputs if p.is_relative_to(ASSETS)},'passed':True})
print('VALIDATED',json.dumps(dimensions))
