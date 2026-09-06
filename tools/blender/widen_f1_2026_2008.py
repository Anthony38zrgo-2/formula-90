"""Blender-only, reproducible lateral edit; preserves original GLB containers.

Run: blender --background --factory-startup --python tools/blender/widen_f1_2026_2008.py
Reads the pre-edit backup, never compounds a previous deformation. Blender edits
the actual imported vertices. Write those positions and transformed normals into
the original GLB buffers: node hierarchy, pivots, UVs and embedded images stay exact.
"""
import bpy
import hashlib
import json
import math
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / 'game/assets/models/vehicles/f1-2026-2008'
BACKUP = Path('D:/Formula90s-backups/f1_2026_2008-width-1990/f1-2026-2008')
REPORT = ROOT / 'reports/vehicle-width-f1_2026_2008'
REPORT.mkdir(parents=True, exist_ok=True)

def lateral(value, front, tip):
    # Preserve chassis-side pickups; extend each member into the inboard hub
    # disk. Targets lie INSIDE the disk thickness, not at the tyre or rim.
    inner, target = (0.17, 0.655) if front else (0.31, 0.608)
    derivative = (target-inner)/(tip-inner) if abs(value)>inner else 1.0
    result = inner+(abs(value)-inner)*derivative if abs(value)>inner else abs(value)
    return math.copysign(result,value), derivative

def member_tips(mesh, axis):
    # GLB duplicates vertices at UV/normal seams. Weld only for connectivity
    # analysis; do not change actual vertices, indices or material boundaries.
    parent=list(range(len(mesh.vertices)))
    def root(i):
        while parent[i]!=i:
            parent[i]=parent[parent[i]]; i=parent[i]
        return i
    def join(a,b): parent[root(a)]=root(b)
    shared={}
    for v in mesh.vertices:
        key=tuple(round(c,5) for c in v.co)
        if key in shared: join(v.index,shared[key])
        else: shared[key]=v.index
    for edge in mesh.edges: join(*edge.vertices)
    tips={}
    for v in mesh.vertices: tips[root(v.index)]=max(tips.get(root(v.index),0),abs(v.co[axis]))
    return {v.index:tips[root(v.index)] for v in mesh.vertices}

def read_glb(path):
    raw = path.read_bytes()
    assert raw[:4] == b'glTF'
    length, kind = struct.unpack_from('<II', raw, 12)
    doc = json.loads(raw[20:20+length])
    off = 20+length
    size, kind = struct.unpack_from('<II', raw, off)
    return doc, bytearray(raw[off+8:off+8+size])

def values(doc, blob, index):
    a=doc['accessors'][index]; view=doc['bufferViews'][a['bufferView']]
    assert a['componentType']==5126 and a['type']=='VEC3'
    off=view.get('byteOffset',0)+a.get('byteOffset',0)
    stride=view.get('byteStride',12)
    return [struct.unpack_from('<3f',blob,off+i*stride) for i in range(a['count'])]

def put_values(doc, blob, index, rows):
    a=doc['accessors'][index]; view=doc['bufferViews'][a['bufferView']]
    off=view.get('byteOffset',0)+a.get('byteOffset',0); stride=view.get('byteStride',12)
    for i,row in enumerate(rows): struct.pack_into('<3f',blob,off+i*stride,*row)
    if 'min' in a: a['min']=[min(r[k] for r in rows) for k in range(3)]
    if 'max' in a: a['max']=[max(r[k] for r in rows) for k in range(3)]

def write_glb(path,doc,blob):
    js=json.dumps(doc,separators=(',',':')).encode()
    js+=b' '*((-len(js))%4); blob+=b'\0'*((-len(blob))%4)
    path.write_bytes(struct.pack('<4sII',b'glTF',2,28+len(js)+len(blob))+
        struct.pack('<II',len(js),0x4e4f534a)+js+struct.pack('<II',len(blob),0x004e4942)+blob)

report={'dimensions_m':{'front_track':1.635,'rear_track':1.590,'front_tire_width':0.355,
    'rear_tire_width':0.400,'front_total':1.990,'rear_total':1.990,'wheelbase':3.23},'assets':{}}
for part in ['chassis','wheel_rear']:
    file='f1_2026_2008_'+part+'.glb'
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(BACKUP/file))
    doc,blob=read_glb(BACKUP/file)
    changes={}
    for obj in bpy.data.objects:
        if obj.type!='MESH': continue
        front='FRONT_SUSPENSION' in obj.name
        if part=='chassis' and not (front or 'REAR_SUSPENSION' in obj.name): continue
        mapping={}
        tips=member_tips(obj.data,0) if part=='chassis' else {}
        for vertex in obj.data.vertices:
            before=vertex.co.copy()
            x,scale=lateral(before.x,front,tips[vertex.index]) if part=='chassis' else (before.x*0.400/0.380,0.400/0.380)
            vertex.co.x=x
            key=tuple(round(c,6) for c in (before.x,before.z,-before.y))
            mapping[key]=(tuple((vertex.co.x,vertex.co.z,-vertex.co.y)),scale)
        obj.data.update()
        node=next(n for n in doc['nodes'] if n['name']==obj.name)
        changed=0
        for primitive in doc['meshes'][node['mesh']]['primitives']:
            attr=primitive['attributes']; points=values(doc,blob,attr['POSITION'])
            normals=values(doc,blob,attr['NORMAL'])
            newpoints=[]; newnormals=[]
            for point,normal in zip(points,normals):
                transformed,scale=mapping[tuple(round(c,6) for c in point)]
                newpoints.append(transformed)
                n=(normal[0]/scale,normal[1],normal[2]); length=math.sqrt(sum(c*c for c in n))
                newnormals.append(tuple(c/length for c in n))
                assert abs(point[1]-transformed[1])<1e-7 and abs(point[2]-transformed[2])<1e-7
                changed+=abs(point[0]-transformed[0])>1e-7
            put_values(doc,blob,attr['POSITION'],newpoints)
            put_values(doc,blob,attr['NORMAL'],newnormals)
        changes[obj.name]={'changed_vertices':changed,'topology_preserved':True,'yz_preserved':True}
    for node in doc['nodes']:
        name=node.get('name','')
        if part=='chassis' and name.startswith('JNT_WHEEL_'):
            node['translation'][0]=math.copysign(0.8175 if name[-2]=='F' else 0.795,node['translation'][0])
        elif part=='wheel_rear' and name in ('DATUM_WHEEL_INBOARD_FACE','DATUM_WHEEL_OUTBOARD_FACE'):
            node['translation'][0]*=0.400/0.380
    write_glb(ASSETS/file,doc,blob)
    original,_=read_glb(BACKUP/file)
    assert [(n['name'],n.get('children')) for n in original['nodes']]==[(n['name'],n.get('children')) for n in doc['nodes']]
    assert original.get('materials')==doc.get('materials')
    assert original.get('images')==doc.get('images')
    report['assets'][part]={'changes':changes,'sha256':hashlib.sha256((ASSETS/file).read_bytes()).hexdigest(),
        'hierarchy_preserved':True,'materials_preserved':True}

# Keep the editable source in sync. Its longitudinal/lateral axes are X/Y.
bpy.ops.wm.open_mainfile(filepath=str(BACKUP/'source/f1_2026_2008_source.blend'))
bpy.context.view_layer.update()
for obj in bpy.data.objects:
    if obj.type=='MESH' and obj.name in ('Front Suspension_F2008_HYBRID','Rear Suspension_F2008_HYBRID'):
        tips=member_tips(obj.data,1)
        for vertex in obj.data.vertices:
            vertex.co.y=lateral(vertex.co.y,obj.name.startswith('Front'),tips[vertex.index])[0]
        obj.data.update()
    if obj.name.startswith('WHEEL_') and obj.name.endswith('_CONTROL'):
        front=obj.name[6]=='F'
        obj.location.y=math.copysign(0.8175 if front else 0.795,obj.location.y)
        if not front:
            for child in obj.children:
                if child.type=='MESH':
                    for vertex in child.data.vertices: vertex.co.y*=0.400/0.380
                    child.data.update()
bpy.context.preferences.filepaths.save_version=0
bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS/'source/f1_2026_2008_source.blend'))
(REPORT/'blender_edit.json').write_text(json.dumps(report,indent=2)+'\n')
print('WIDTH_EDIT',json.dumps(report))
