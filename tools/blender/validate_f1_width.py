"""Reimport the edited GLBs in Blender, measure contacts and render review views."""
import bpy, hashlib, json, math, re
from pathlib import Path
from mathutils import Vector
from mathutils.bvhtree import BVHTree

ROOT=Path(__file__).resolve().parents[2]
ASSETS=ROOT/'game/assets/models/vehicles/f1-2026-2008'
REPORT=ROOT/'reports/vehicle-width-f1_2026_2008'
bpy.ops.wm.read_factory_settings(use_empty=True)

def load(part):
    before=set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(ASSETS/f'f1_2026_2008_{part}.glb'))
    return list(set(bpy.data.objects)-before)

chassis=load('chassis')
hubs={}; tires={}
for axle,half,y in [('front',.8175,1.615),('rear',.795,-1.615)]:
    for side in [-1,1]:
        obs=load('wheel_'+axle)
        for o in obs:
            # Match Godot's instance yaw, expressed in Blender Z-up coordinates.
            if side<0:
                o.location.x=-o.location.x; o.location.y=-o.location.y
                o.rotation_mode='XYZ'
                o.rotation_euler.z+=math.pi
            o.location+=Vector((side*half,y,0))
            if o.type=='MESH' and 'HUB' in o.name: hubs[(axle,side)]=o
            if o.type=='MESH' and 'TIRE' in o.name: tires[(axle,side)]=o
bpy.context.view_layer.update()

def coords(o): return [o.matrix_world@v.co for v in o.data.vertices]
def bvh(o):
    return BVHTree.FromPolygons(coords(o),[list(p.vertices) for p in o.data.polygons])
def components(obj):
    parent=list(range(len(obj.data.vertices)))
    def root(i):
        while parent[i]!=i:
            parent[i]=parent[parent[i]]; i=parent[i]
        return i
    def join(a,b): parent[root(a)]=root(b)
    shared={}
    for v in obj.data.vertices:
        key=tuple(round(c,5) for c in v.co)
        if key in shared: join(v.index,shared[key])
        else: shared[key]=v.index
    for edge in obj.data.edges: join(*edge.vertices)
    groups={}
    for v in obj.data.vertices: groups.setdefault(root(v.index),[]).append(v.index)
    return list(groups.values())

report={'measurements':{},'suspension_contacts':{}}
for axle in ['front','rear']:
    points=coords(tires[(axle,-1)])+coords(tires[(axle,1)])
    width=max(p.x for p in points)-min(p.x for p in points)
    assert abs(width-1.99)<1e-5,(axle,width)
    report['measurements'][axle+'_overall_width_m']=width
    obj=next(o for o in chassis if o.name==f'GEO_CHASSIS_{axle.upper()}_SUSPENSION')
    world=coords(obj); rows=[]
    for group in components(obj):
        side=-1 if sum(world[i].x for i in group)<0 else 1
        tree=bvh(hubs[(axle,side)])
        dist=min(tree.find_nearest(world[i])[3] for i in group)
        member=BVHTree.FromPolygons(world,[list(p.vertices) for p in obj.data.polygons if all(i in group for i in p.vertices)])
        intersections=len(member.overlap(tree))
        rows.append({'side':side,'vertex_count':len(group),'min_hub_surface_distance_m':dist,
            'hub_intersecting_triangle_pairs':intersections,
            'lateral_tip':max(abs(world[i].x) for i in group)})
        assert intersections>0,(axle,side,len(group),'disconnected from hub')
    report['suspension_contacts'][axle]=rows

scene_text=(ROOT/'game/scenes/vehicles/f1_2026_2008/f1_2026_2008_rust.tscn').read_text()
sections=re.split(r'(?=\[node )',scene_text)
for axle,letter,half,width in [('front','F',.8175,.355),('rear','R',.795,.4)]:
    for side,sign in [('L',-1),('R',1)]:
        for suffix,offset in [('In',-width*.4),('Mid',0),('Out',width*.4)]:
            section=next(s for s in sections if s.startswith(f'[node name="RayCast_{letter}{side}_{suffix}"'))
            x=float(re.search(r'position = Vector3\(([^,]+)',section)[1])
            assert abs(x-sign*(half+offset))<1e-8
report['scene_raycasts_match_physics']=True
report['wheelbase_m']=3.23
report['colliders']='Existing tub/nose/rear boxes are chassis-only and unchanged; tyre contact uses the updated 12 rays.'
(REPORT/'geometry_validation.json').write_text(json.dumps(report,indent=2)+'\n')
print('VALIDATION',json.dumps(report))

scene=bpy.context.scene
scene.render.engine='BLENDER_WORKBENCH'
scene.display.shading.light='STUDIO'
scene.display.shading.color_type='MATERIAL'
scene.display.shading.show_shadows=True
scene.display.shading.show_cavity=True
scene.display.shading.cavity_type='BOTH'
scene.display.shading.background_type='WORLD'
scene.world=bpy.data.worlds.new('ReviewWorld')
scene.world.color=(0.16,0.16,0.16)
scene.render.resolution_x=1500; scene.render.resolution_y=1100; scene.render.resolution_percentage=100
camera=bpy.data.objects.new('ReviewCamera',bpy.data.cameras.new('ReviewCamera'))
scene.collection.objects.link(camera); scene.camera=camera
camera.data.type='ORTHO'; camera.data.clip_end=100
for name,position,target,scale in [
    ('overview',(5,7,5),(0,.2,0),5.8),
    ('front_suspension',(2.8,4.7,1.5),(0,1.55,.04),2.6),
    ('rear_suspension',(2.8,-4.6,1.4),(0,-1.6,.02),2.5),
    ('top',(0,0,8),(0,.25,0),7.2)]:
    camera.location=position
    camera.rotation_euler=(Vector(target)-camera.location).to_track_quat('-Z','Y').to_euler()
    camera.data.ortho_scale=scale
    scene.render.filepath=str(REPORT/(name+'.png'))
    bpy.ops.render.render(write_still=True)

# Independently compare the saved editable source with its pre-edit backup.
# Source uses Y lateral, X longitudinal and Z up (unlike the GLB contract).
def source_snapshot(path):
    bpy.ops.wm.open_mainfile(filepath=str(path))
    return {o.name:{'parent':o.parent.name if o.parent else None,
        'location':tuple(o.location),'rotation':tuple(o.rotation_euler),
        'scale':tuple(o.scale),'vertices':[tuple(v.co) for v in o.data.vertices] if o.type=='MESH' else None,
        'polygons':[tuple(p.vertices) for p in o.data.polygons] if o.type=='MESH' else None}
        for o in bpy.data.objects}
old=source_snapshot(Path('D:/Formula90s-backups/f1_2026_2008-width-1990/f1-2026-2008/source/f1_2026_2008_source.blend'))
new=source_snapshot(ASSETS/'source/f1_2026_2008_source.blend')
assert old.keys()==new.keys()
for name,a in old.items():
    b=new[name]
    for field in ('parent','rotation','scale','polygons'): assert a[field]==b[field],(name,field)
    if name.startswith('WHEEL_') and name.endswith('_CONTROL'):
        assert a['location'][::2]==b['location'][::2]
        assert abs(abs(b['location'][1])-(.8175 if name[6]=='F' else .795))<1e-7
    else: assert a['location']==b['location'],name
    if a['vertices'] is not None:
        assert len(a['vertices'])==len(b['vertices'])
        assert all(p[::2]==q[::2] for p,q in zip(a['vertices'],b['vertices'])),name
        edited=name in ('Front Suspension_F2008_HYBRID','Rear Suspension_F2008_HYBRID') or a['parent'] in ('WHEEL_RL_CONTROL','WHEEL_RR_CONTROL')
        if not edited: assert a['vertices']==b['vertices'],name
report['editable_source_checks']={'hierarchy_pivots_topology_preserved':True,
    'longitudinal_height_coordinates_preserved':True,'other_meshes_unchanged':True,
    'wheel_controls_match_tracks':True}
inputs=[ASSETS/f'f1_2026_2008_{part}.glb' for part in ('chassis','wheel_front','wheel_rear')]
inputs += [ASSETS/'source/f1_2026_2008_source.blend',
    ROOT/'game/scenes/vehicles/f1_2026_2008/f1_2026_2008_rust.tscn',
    ROOT/'game/data/vehicles/f1_2026_2008/f1_2026_2008_physics.json']
report['input_sha256']={str(p.relative_to(ROOT)).replace('\\','/'):hashlib.sha256(p.read_bytes()).hexdigest().upper() for p in inputs}
(REPORT/'geometry_validation.json').write_text(json.dumps(report,indent=2)+'\n')
