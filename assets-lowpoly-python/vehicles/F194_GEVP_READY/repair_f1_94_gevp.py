from __future__ import annotations

from pathlib import Path
from collections import defaultdict, deque
import json, math, shutil, struct, zipfile
import numpy as np
from PIL import Image
import trimesh
from trimesh.visual.texture import TextureVisuals
from trimesh.visual.material import PBRMaterial

SRC_OBJ = Path('/mnt/data/F1-94(1).obj')
SRC_MTL = Path('/mnt/data/F1-94(1).mtl')
SRC_TEX = Path('/mnt/data/texture(1).png')
OUT = Path('/mnt/data/F1_94_GEVP_READY')
OUT.mkdir(exist_ok=True)

OBJ_OUT = OUT/'F1_94_GEVP_READY.obj'
MTL_OUT = OUT/'F1_94_GEVP_READY.mtl'
TEX_OUT = OUT/'F1_94_texture.png'
FULL_GLB = OUT/'F1_94_GEVP_READY.glb'
CHASSIS_GLB = OUT/'F1_94_chassis.glb'
WHEEL_FRONT_GLB = OUT/'F1_94_wheel_front.glb'
WHEEL_REAR_GLB = OUT/'F1_94_wheel_rear.glb'
META_OUT = OUT/'F1_94_gevp_metadata.json'
REPORT_OUT = OUT/'REPAIR_REPORT.txt'
NOTES_OUT = OUT/'GEVP_IMPORT_NOTES.txt'
SCRIPT_OUT = OUT/'repair_f1_94_gevp.py'
ZIP_OUT = Path('/mnt/data/F1_94_GEVP_READY.zip')

# ---------- Parse OBJ ----------
vertices=[]; texcoords=[]; normals=[]; faces=[]; objects=[]; object_faces=defaultdict(list)
current_obj='UNNAMED'; current_mtl='F1_94_PIXEL_MATERIAL'
with SRC_OBJ.open('r', errors='ignore') as f:
    for line in f:
        s=line.strip()
        if not s or s.startswith('#') or s.startswith('mtllib '):
            continue
        if s.startswith('o '):
            current_obj=s[2:].strip()
            if current_obj not in objects: objects.append(current_obj)
        elif s.startswith('usemtl '):
            current_mtl=s.split(None,1)[1].strip()
        elif s.startswith('v '):
            _,x,y,z,*_=s.split(); vertices.append([float(x),float(y),float(z)])
        elif s.startswith('vt '):
            _,u,v,*_=s.split(); texcoords.append([float(u),float(v)])
        elif s.startswith('vn '):
            _,x,y,z,*_=s.split(); normals.append([float(x),float(y),float(z)])
        elif s.startswith('f '):
            corners=[]
            for tok in s.split()[1:]:
                parts=tok.split('/')
                vi=int(parts[0]); ti=int(parts[1]) if len(parts)>1 and parts[1] else None; ni=int(parts[2]) if len(parts)>2 and parts[2] else None
                vi=vi-1 if vi>0 else len(vertices)+vi
                ti=(ti-1 if ti and ti>0 else len(texcoords)+ti) if ti is not None else None
                ni=(ni-1 if ni and ni>0 else len(normals)+ni) if ni is not None else None
                corners.append((vi,ti,ni))
            if len(corners)!=3:
                # deterministic fan triangulation, though input is already triangles
                base=corners[0]
                tris=[[base,corners[i],corners[i+1]] for i in range(1,len(corners)-1)]
            else:
                tris=[corners]
            for tri in tris:
                idx=len(faces)
                faces.append({'obj':current_obj,'mtl':current_mtl,'corners':tri})
                object_faces[current_obj].append(idx)

V=np.asarray(vertices,dtype=np.float64)
VT=np.asarray(texcoords,dtype=np.float64)

# ---------- Coordinate normalization to Formula-90's current GEVP scene convention ----------
# Existing project scene uses FL x<0,z<0; FR x>0,z<0; RL x<0,z>0; RR x>0,z>0.
# Source Blockbench mesh is opposite on X and Z, so rotate 180 degrees around Y.
Vr=V.copy(); Vr[:,0]*=-1.0; Vr[:,2]*=-1.0

# Derive wheel centers from complete tire bounds, not hub meshes (hubs are biased toward outer face).
wheel_obj_sets={
    'FL':['LP_TIRE_LF_SIDE_OUTER','LP_TIRE_LF_TREAD','LP_TIRE_LF_SIDE_INNER'],
    'FR':['LP_TIRE_RF_SIDE_OUTER','LP_TIRE_RF_TREAD','LP_TIRE_RF_SIDE_INNER'],
    'RL':['LP_TIRE_LR_SIDE_OUTER','LP_TIRE_LR_TREAD','LP_TIRE_LR_SIDE_INNER'],
    'RR':['LP_TIRE_RR_SIDE_OUTER','LP_TIRE_RR_TREAD','LP_TIRE_RR_SIDE_INNER'],
}

def obj_vertex_indices(names):
    out=[]
    for n in names:
        for fi in object_faces.get(n,[]):
            out.extend(c[0] for c in faces[fi]['corners'])
    return sorted(set(out))

wheel_centers_pre={}; wheel_bounds_pre={}
for key,names in wheel_obj_sets.items():
    inds=obj_vertex_indices(names)
    pts=Vr[inds]
    bmin=pts.min(0); bmax=pts.max(0)
    wheel_bounds_pre[key]=(bmin,bmax)
    wheel_centers_pre[key]=(bmin+bmax)/2.0

# Origin = axle midpoint longitudinally + mean wheel center height.
front_c=(wheel_centers_pre['FL']+wheel_centers_pre['FR'])/2
rear_c=(wheel_centers_pre['RL']+wheel_centers_pre['RR'])/2
origin_shift=np.array([0.0, (front_c[1]+rear_c[1])/2.0, (front_c[2]+rear_c[2])/2.0])
Vn=Vr-origin_shift
wheel_centers={k:v-origin_shift for k,v in wheel_centers_pre.items()}
wheel_bounds={k:(a-origin_shift,b-origin_shift) for k,(a,b) in wheel_bounds_pre.items()}

# ---------- Naming ----------
rename={
 'LP_CHASSIS':'GEO_CHASSIS',
 'LP_BODY_INTERIOR':'GEO_BODY_INTERIOR',
 'LP_FRONT_WING':'GEO_AERO_FRONT_WING',
 'LP_REAR_WING':'GEO_AERO_REAR_WING',
 'LP_NOSE':'GEO_NOSE',
 'LP_SUSP_LF':'GEO_SUSPENSION_FL',
 'LP_SUSP_RF':'GEO_SUSPENSION_FR',
 'LP_SUSP_LR':'GEO_SUSPENSION_RL',
 'LP_SUSP_RR':'GEO_SUSPENSION_RR',
 'LP_TIRE_LF_SIDE_OUTER':'GEO_WHEEL_FL_TIRE_OUTER',
 'LP_TIRE_LF_TREAD':'GEO_WHEEL_FL_TREAD',
 'LP_TIRE_LF_SIDE_INNER':'GEO_WHEEL_FL_TIRE_INNER',
 'LP_HUB_LF':'GEO_WHEEL_FL_HUB',
 'LP_TIRE_RF_SIDE_OUTER':'GEO_WHEEL_FR_TIRE_OUTER',
 'LP_TIRE_RF_TREAD':'GEO_WHEEL_FR_TREAD',
 'LP_TIRE_RF_SIDE_INNER':'GEO_WHEEL_FR_TIRE_INNER',
 'LP_HUB_RF':'GEO_WHEEL_FR_HUB',
 'LP_TIRE_LR_SIDE_OUTER':'GEO_WHEEL_RL_TIRE_OUTER',
 'LP_TIRE_LR_TREAD':'GEO_WHEEL_RL_TREAD',
 'LP_TIRE_LR_SIDE_INNER':'GEO_WHEEL_RL_TIRE_INNER',
 'LP_HUB_LR':'GEO_WHEEL_RL_HUB',
 'LP_TIRE_RR_SIDE_OUTER':'GEO_WHEEL_RR_TIRE_OUTER',
 'LP_TIRE_RR_TREAD':'GEO_WHEEL_RR_TREAD',
 'LP_TIRE_RR_SIDE_INNER':'GEO_WHEEL_RR_TIRE_INNER',
 'LP_HUB_RR':'GEO_WHEEL_RR_HUB',
 'LP_HELMET':'GEO_DRIVER_HELMET',
}
for n in objects:
    if n.startswith('LP_LCD_PART_'):
        rename[n]='GEO_COCKPIT_LCD_'+n.split('_')[-1].zfill(2)
    elif n=='LP_LCD_INDICATOR':
        rename[n]='GEO_COCKPIT_LCD_INDICATOR'
    elif n not in rename:
        rename[n]='GEO_'+n.removeprefix('LP_')

# ---------- Topology repair ----------
# Faces use duplicated per-corner vertices. Build adjacency by quantized position, propagate a consistent
# winding, then orient each shell/surface outward using signed volume or semantic expectations.
face_flip={i:False for i in range(len(faces))}
repair_stats={}
object_components={}

# semantic expected normal helper for wheel surface components
def wheel_key_from_obj(name):
    for k in ('FL','FR','RL','RR'):
        if f'_{k}_' in name or name.endswith('_'+k): return k
    return None

def area_normal(tri):
    cr=np.cross(tri[1]-tri[0],tri[2]-tri[0]); ln=np.linalg.norm(cr)
    if ln<1e-14: return np.zeros(3),0.0
    return cr/ln, ln*0.5

for obj in objects:
    fis=object_faces[obj]
    # geometric IDs within this object
    key_to_gid={}; gtris=[]
    for fi in fis:
        ids=[]
        for vi,ti,ni in faces[fi]['corners']:
            key=tuple(np.round(Vn[vi],6))
            if key not in key_to_gid: key_to_gid[key]=len(key_to_gid)
            ids.append(key_to_gid[key])
        gtris.append(ids)
    edge_faces=defaultdict(list)
    for li,t in enumerate(gtris):
        for a,b in ((t[0],t[1]),(t[1],t[2]),(t[2],t[0])):
            edge_faces[tuple(sorted((a,b)))].append((li,a,b))
    boundary=sum(1 for v in edge_faces.values() if len(v)==1)
    nonman=sum(1 for v in edge_faces.values() if len(v)>2)
    adj=defaultdict(list)
    for lst in edge_faces.values():
        # connect all pairs deterministically; input has no non-manifold edges but preserve behavior
        for i in range(len(lst)):
            for j in range(i+1,len(lst)):
                f1,a1,b1=lst[i]; f2,a2,b2=lst[j]
                same=(a1==a2 and b1==b2)
                adj[f1].append((f2,same)); adj[f2].append((f1,same))
    state={}; comps=[]; conflicts=0
    for start in range(len(gtris)):
        if start in state: continue
        state[start]=False; comp=[]; q=deque([start])
        while q:
            a=q.popleft(); comp.append(a)
            for b,need_diff in adj[a]:
                desired=state[a]^need_diff
                if b in state:
                    if state[b]!=desired: conflicts+=1
                else:
                    state[b]=desired; q.append(b)
        comps.append(comp)
    # apply local consistency first
    for li,st in state.items():
        if st: face_flip[fis[li]] = not face_flip[fis[li]]

    # whole-object bbox center gives a stable radial outward reference for open surfaces
    pts_all=np.vstack([Vn[[c[0] for c in faces[fi]['corners']]] for fi in fis])
    obj_center=(pts_all.min(0)+pts_all.max(0))/2
    comp_records=[]
    for ci,comp in enumerate(comps,1):
        # gather component edge counts to know if closed
        comp_set=set(comp); comp_edge_counts=defaultdict(int)
        for li in comp:
            t=gtris[li]
            for a,b in ((t[0],t[1]),(t[1],t[2]),(t[2],t[0])):
                comp_edge_counts[tuple(sorted((a,b)))] += 1
        closed=all(v==2 for v in comp_edge_counts.values()) if comp_edge_counts else False
        vol=0.; radial_score=0.; semantic_score=0.; area_sum=0.
        wk=wheel_key_from_obj(obj)
        for li in comp:
            fi=fis[li]
            inds=[c[0] for c in faces[fi]['corners']]
            tri=Vn[inds].copy()
            if face_flip[fi]: tri=tri[[0,2,1]]
            n,ar=area_normal(tri); fc=tri.mean(0)
            area_sum+=ar
            radial_score += float(np.dot(n,fc-obj_center))*ar
            vol += float(np.dot(tri[0],np.cross(tri[1],tri[2])))/6.0
            if wk:
                wc=wheel_centers[wk]
                if '_TREAD' in obj:
                    exp=np.array([0.0,fc[1]-wc[1],fc[2]-wc[2]])
                elif '_SIDE_OUTER' in obj or '_HUB_' in obj:
                    exp=np.array([-1.0 if wk[1]=='L' else 1.0,0.0,0.0])
                elif '_SIDE_INNER' in obj:
                    exp=np.array([1.0 if wk[1]=='L' else -1.0,0.0,0.0])
                else:
                    exp=fc-wc
                le=np.linalg.norm(exp)
                if le>1e-12: semantic_score += float(np.dot(n,exp/le))*ar
        flip_component=False; rule='radial'
        if closed and abs(vol)>1e-12:
            flip_component=vol<0; rule='signed_volume'
        elif wk and abs(semantic_score)>1e-10:
            flip_component=semantic_score<0; rule='wheel_semantic'
        elif obj.startswith('LP_LCD_'):
            # Dashboard/LCD sheets should face rearward (+Z in project convention), toward driver/cockpit.
            # Use the average normal of the component.
            avg=np.zeros(3)
            for li in comp:
                fi=fis[li]; tri=Vn[[c[0] for c in faces[fi]['corners']]].copy()
                if face_flip[fi]: tri=tri[[0,2,1]]
                n,ar=area_normal(tri); avg+=n*ar
            flip_component=avg[2]<0; rule='cockpit_faces_driver_+Z'
        elif obj=='LP_BODY_INTERIOR':
            avg=np.zeros(3)
            for li in comp:
                fi=fis[li]; tri=Vn[[c[0] for c in faces[fi]['corners']]].copy()
                if face_flip[fi]: tri=tri[[0,2,1]]
                n,ar=area_normal(tri); avg+=n*ar
            flip_component=avg[2]<0; rule='interior_faces_rear_+Z'
        else:
            # radial score from object center handles open shells/endplates. Avoid flipping when numerically ambiguous.
            if abs(radial_score) > max(1e-10, area_sum*1e-5):
                flip_component=radial_score<0; rule='object_center_outward'
            else:
                flip_component=False; rule='ambiguous_preserved'
        if flip_component:
            for li in comp:
                fi=fis[li]; face_flip[fi]=not face_flip[fi]
        comp_records.append({'component':ci,'faces':len(comp),'closed':closed,'signed_volume_before_global':vol,
                             'radial_score':radial_score/(area_sum+1e-30),'semantic_score':semantic_score/(area_sum+1e-30),
                             'global_flip':flip_component,'rule':rule})
    object_components[obj]=comps
    repair_stats[obj]={'faces':len(fis),'geometric_vertices':len(key_to_gid),'boundary_edges':boundary,
                       'nonmanifold_edges':nonman,'orientation_conflicts':conflicts,'components':comp_records}

# ---------- Recalculate flat face normals ----------
face_normals=[]
for fi,f in enumerate(faces):
    inds=[c[0] for c in f['corners']]
    tri=Vn[inds].copy()
    if face_flip[fi]: tri=tri[[0,2,1]]
    n,ar=area_normal(tri)
    if ar==0: n=np.array([0.,1.,0.])
    face_normals.append(n)
face_normals=np.asarray(face_normals)

# ---------- Joint/datums ----------
front_axle=(wheel_centers['FL']+wheel_centers['FR'])/2
rear_axle=(wheel_centers['RL']+wheel_centers['RR'])/2

joints={
 'DATUM_VEHICLE_ORIGIN':np.array([0.,0.,0.]),
 'DATUM_FRONT_AXLE_CENTER':front_axle,
 'DATUM_REAR_AXLE_CENTER':rear_axle,
 'JNT_WHEEL_FL':wheel_centers['FL'],
 'JNT_WHEEL_FR':wheel_centers['FR'],
 'JNT_WHEEL_RL':wheel_centers['RL'],
 'JNT_WHEEL_RR':wheel_centers['RR'],
}

# Suspension arm endpoints: each connected 12-face bar/component becomes ARM_XX inner/outer markers.
def component_unique_points(obj, comp_local):
    fis=object_faces[obj]
    inds=[]
    for li in comp_local:
        fi=fis[li]; inds.extend(c[0] for c in faces[fi]['corners'])
    pts=Vn[sorted(set(inds))]
    # unique by rounded geometry
    uniq={tuple(np.round(p,6)):p for p in pts}
    return np.asarray(list(uniq.values()))

susp_joint_summary={}
for obj,side in [('LP_SUSP_LF','FL'),('LP_SUSP_RF','FR'),('LP_SUSP_LR','RL'),('LP_SUSP_RR','RR')]:
    arm_pairs=[]
    comps=object_components[obj]
    for ai,comp in enumerate(comps,1):
        pts=component_unique_points(obj,comp)
        absx=np.abs(pts[:,0])
        # End clusters based on quartiles of lateral extent.
        amin=absx.min(); amax=absx.max(); span=max(amax-amin,1e-9)
        inner_pts=pts[absx <= amin+0.18*span]
        outer_pts=pts[absx >= amax-0.18*span]
        if len(inner_pts)==0: inner_pts=pts[[absx.argmin()]]
        if len(outer_pts)==0: outer_pts=pts[[absx.argmax()]]
        inner=inner_pts.mean(0); outer=outer_pts.mean(0)
        joints[f'JNT_SUSP_{side}_ARM_{ai:02d}_INNER']=inner
        joints[f'JNT_SUSP_{side}_ARM_{ai:02d}_OUTER']=outer
        arm_pairs.append((inner,outer))
    joints[f'JNT_SUSP_{side}_CHASSIS']=np.mean([p[0] for p in arm_pairs],axis=0)
    joints[f'JNT_SUSP_{side}_HUB']=np.mean([p[1] for p in arm_pairs],axis=0)
    susp_joint_summary[side]=len(arm_pairs)

# Generic connection markers inferred from interface extents.
def object_points(obj):
    inds=[]
    for fi in object_faces[obj]: inds.extend(c[0] for c in faces[fi]['corners'])
    return Vn[sorted(set(inds))]

# Nose/chassis seam: rear-most Z of nose (larger Z; front is -Z) near centerline.
nose=object_points('LP_NOSE'); z_if=nose[:,2].max(); sel=nose[(nose[:,2]>z_if-0.03)&(np.abs(nose[:,0])<0.20)]
if len(sel)==0: sel=nose[np.argsort(nose[:,2])[-max(1,len(nose)//20):]]
joints['JNT_NOSE_CHASSIS']=sel.mean(0)

fw=object_points('LP_FRONT_WING'); zrear=fw[:,2].max(); sel=fw[(fw[:,2]>zrear-0.06)&(np.abs(fw[:,0])<0.22)]
if len(sel)==0: sel=fw[np.argsort(fw[:,2])[-max(1,len(fw)//20):]]
joints['JNT_FRONT_WING_MOUNT']=sel.mean(0)

rw=object_points('LP_REAR_WING'); zfront=rw[:,2].min(); # closest to chassis/front side of rear wing
sel=rw[(rw[:,2]<zfront+0.08)&(np.abs(rw[:,0])<0.25)]
if len(sel)==0: sel=rw[np.argsort(rw[:,2])[:max(1,len(rw)//20)]]
joints['JNT_REAR_WING_MOUNT']=sel.mean(0)

helmet=object_points('LP_HELMET'); joints['JNT_DRIVER_HEAD']=helmet.mean(0)

# ---------- OBJ + MTL ----------
shutil.copy2(SRC_TEX,TEX_OUT)
with MTL_OUT.open('w') as f:
    f.write('# F1-94 repaired for Godot / Formula-90 GEVP\n')
    f.write('newmtl F1_94_PIXEL_MATERIAL\n')
    f.write('Ka 1.000000 1.000000 1.000000\n')
    f.write('Kd 1.000000 1.000000 1.000000\n')
    f.write('Ks 0.000000 0.000000 0.000000\n')
    f.write('Ns 1.000000\n')
    f.write('d 1.000000\n')
    f.write('illum 1\n')
    f.write('map_Kd F1_94_texture.png\n')

with OBJ_OUT.open('w') as f:
    f.write('# Repaired deterministically from Blockbench OBJ\n')
    f.write('# Formula-90 GEVP convention: +X right, +Y up, front axle at -Z, rear axle at +Z\n')
    f.write('# Origin rebased to axle midpoint / mean wheel-center height\n')
    for name,pos in sorted(joints.items()):
        f.write(f'# locator {name} {pos[0]:.8f} {pos[1]:.8f} {pos[2]:.8f}\n')
    f.write('mtllib F1_94_GEVP_READY.mtl\n\n')
    for p in Vn: f.write(f'v {p[0]:.8f} {p[1]:.8f} {p[2]:.8f}\n')
    for uv in VT: f.write(f'vt {uv[0]:.8f} {uv[1]:.8f}\n')
    for n in face_normals: f.write(f'vn {n[0]:.10f} {n[1]:.10f} {n[2]:.10f}\n')
    f.write('\n')
    last_obj=None
    for fi,face in enumerate(faces):
        obj=rename[face['obj']]
        if obj!=last_obj:
            f.write(f'\no {obj}\nusemtl F1_94_PIXEL_MATERIAL\ns off\n')
            last_obj=obj
        corners=list(face['corners'])
        if face_flip[fi]: corners=[corners[0],corners[2],corners[1]]
        toks=[]
        for vi,ti,ni in corners:
            toks.append(f'{vi+1}/{ti+1}/{fi+1}')
        f.write('f '+' '.join(toks)+'\n')

# ---------- GLB helpers ----------
image=Image.open(TEX_OUT).convert('RGBA')
material=PBRMaterial(name='F1_94_PIXEL_MATERIAL', baseColorTexture=image, baseColorFactor=[255,255,255,255],
                     metallicFactor=0.0, roughnessFactor=1.0, doubleSided=False, alphaMode='MASK', alphaCutoff=0.5)

wheel_geometry_objs={
 'FL':['LP_TIRE_LF_SIDE_OUTER','LP_TIRE_LF_TREAD','LP_TIRE_LF_SIDE_INNER','LP_HUB_LF'],
 'FR':['LP_TIRE_RF_SIDE_OUTER','LP_TIRE_RF_TREAD','LP_TIRE_RF_SIDE_INNER','LP_HUB_RF'],
 'RL':['LP_TIRE_LR_SIDE_OUTER','LP_TIRE_LR_TREAD','LP_TIRE_LR_SIDE_INNER','LP_HUB_LR'],
 'RR':['LP_TIRE_RR_SIDE_OUTER','LP_TIRE_RR_TREAD','LP_TIRE_RR_SIDE_INNER','LP_HUB_RR'],
}
all_wheel_obj_names={x for v in wheel_geometry_objs.values() for x in v}

def build_mesh_for_object(obj, local_origin=None):
    vs=[]; uvs=[]; fs=[]
    for fi in object_faces[obj]:
        face=faces[fi]; corners=list(face['corners'])
        if face_flip[fi]: corners=[corners[0],corners[2],corners[1]]
        base=len(vs)
        for vi,ti,ni in corners:
            p=Vn[vi].copy()
            if local_origin is not None: p=p-local_origin
            vs.append(p); uvs.append(VT[ti])
        fs.append([base,base+1,base+2])
    mesh=trimesh.Trimesh(vertices=np.asarray(vs),faces=np.asarray(fs),process=False,validate=False)
    mesh.visual=TextureVisuals(uv=np.asarray(uvs),material=material)
    # normals are flat because vertices are per-face unique
    mesh.vertex_normals = np.repeat(face_normals[object_faces[obj]],3,axis=0)
    return mesh

def add_locator(scene,name,pos,parent=None,kind='joint',extra=None):
    md={'kind':kind,'position_m':[float(x) for x in pos]}
    if extra: md.update(extra)
    scene.graph.update(frame_to=name, frame_from=parent or scene.graph.base_frame, translation=np.asarray(pos,float), metadata=md)

def patch_glb_nearest(blob:bytes)->bytes:
    # Rewrite JSON chunk to force nearest-neighbor texture sampling; preserve BIN chunk.
    magic,ver,total=struct.unpack_from('<4sII',blob,0)
    assert magic==b'glTF' and ver==2
    off=12; chunks=[]
    while off<len(blob):
        ln,typ=struct.unpack_from('<I4s',blob,off); data=blob[off+8:off+8+ln]; chunks.append((typ,data)); off+=8+ln
    jidx=next(i for i,(t,d) in enumerate(chunks) if t==b'JSON')
    doc=json.loads(chunks[jidx][1].decode('utf-8').rstrip(' \t\r\n\x00'))
    if doc.get('textures'):
        doc['samplers']=[{'magFilter':9728,'minFilter':9728,'wrapS':10497,'wrapT':10497}]
        for t in doc['textures']: t['sampler']=0
    raw=json.dumps(doc,separators=(',',':')).encode('utf-8')
    raw+=b' ' *((4-len(raw)%4)%4)
    chunks[jidx]=(b'JSON',raw)
    body=b''
    for typ,data in chunks:
        if len(data)%4:
            pad=b' ' if typ==b'JSON' else b'\x00'
            data=data+pad*((4-len(data)%4)%4)
        body+=struct.pack('<I4s',len(data),typ)+data
    return struct.pack('<4sII',b'glTF',2,12+len(body))+body

def export_scene(scene,path):
    blob=trimesh.exchange.gltf.export_glb(scene,include_normals=True)
    blob=patch_glb_nearest(blob)
    path.write_bytes(blob)

# Full preview GLB with wheel pivot hierarchy.
scene=trimesh.Scene(base_frame='F1_94_GEVP_READY', metadata={'units':'meters','front_axis':'-Z','right_axis':'+X','up_axis':'+Y'})
for obj in objects:
    if obj in all_wheel_obj_names: continue
    mesh=build_mesh_for_object(obj)
    scene.add_geometry(mesh,node_name=rename[obj],geom_name=rename[obj],parent_node_name=scene.graph.base_frame,
                       metadata={'source_object':obj,'role':'geometry'})
for wk,names in wheel_geometry_objs.items():
    pivot=f'JNT_WHEEL_{wk}'
    add_locator(scene,pivot,wheel_centers[wk],kind='wheel_pivot',extra={'gevp_mapping':{'FL':'WheelFrontLeft','FR':'WheelFrontRight','RL':'WheelRearLeft','RR':'WheelRearRight'}[wk]})
    for obj in names:
        mesh=build_mesh_for_object(obj,local_origin=wheel_centers[wk])
        scene.add_geometry(mesh,node_name=rename[obj],geom_name=rename[obj],parent_node_name=pivot,metadata={'source_object':obj,'role':'wheel_geometry'})
# Add all non-wheel markers (wheel pivots already created)
for name,pos in joints.items():
    if name.startswith('JNT_WHEEL_'): continue
    kind='datum' if name.startswith('DATUM_') else 'joint'
    add_locator(scene,name,pos,kind=kind)
export_scene(scene,FULL_GLB)

# Chassis GLB: no wheel geometry, all markers retained.
ch=trimesh.Scene(base_frame='F1_94_CHASSIS',metadata={'units':'meters','front_axis':'-Z','right_axis':'+X','up_axis':'+Y'})
for obj in objects:
    if obj in all_wheel_obj_names: continue
    mesh=build_mesh_for_object(obj)
    ch.add_geometry(mesh,node_name=rename[obj],geom_name=rename[obj],parent_node_name=ch.graph.base_frame,metadata={'source_object':obj})
for name,pos in joints.items():
    kind='datum' if name.startswith('DATUM_') else ('wheel_pivot' if name.startswith('JNT_WHEEL_') else 'joint')
    add_locator(ch,name,pos,kind=kind)
export_scene(ch,CHASSIS_GLB)

# Canonical left wheel assets for project policy: one front + one rear, local origin at wheel center.
def export_canonical_wheel(which,src_key,path):
    s=trimesh.Scene(base_frame=f'F1_94_WHEEL_{which}',metadata={'units':'meters','axis':'wheel axle = X','canonical_side':'left'})
    center=wheel_centers[src_key]
    for obj in wheel_geometry_objs[src_key]:
        mesh=build_mesh_for_object(obj,local_origin=center)
        # Generic names strip side for reusable asset
        nm=rename[obj].replace(f'_{src_key}_','_')
        s.add_geometry(mesh,node_name=nm,geom_name=nm,parent_node_name=s.graph.base_frame,metadata={'source_object':obj})
    s.graph.update(frame_to='DATUM_WHEEL_CENTER',frame_from=s.graph.base_frame,translation=[0,0,0],metadata={'kind':'datum'})
    export_scene(s,path)
export_canonical_wheel('FRONT','FL',WHEEL_FRONT_GLB)
export_canonical_wheel('REAR','RL',WHEEL_REAR_GLB)

# ---------- Metadata ----------
def f3(v): return [round(float(x),6) for x in v]
front_radius=max((wheel_bounds['FL'][1][1]-wheel_bounds['FL'][0][1])/2,(wheel_bounds['FL'][1][2]-wheel_bounds['FL'][0][2])/2)
rear_radius=max((wheel_bounds['RL'][1][1]-wheel_bounds['RL'][0][1])/2,(wheel_bounds['RL'][1][2]-wheel_bounds['RL'][0][2])/2)
front_width=wheel_bounds['FL'][1][0]-wheel_bounds['FL'][0][0]
rear_width=wheel_bounds['RL'][1][0]-wheel_bounds['RL'][0][0]
wheelbase=abs(rear_axle[2]-front_axle[2])
front_track=abs(wheel_centers['FR'][0]-wheel_centers['FL'][0])
rear_track=abs(wheel_centers['RR'][0]-wheel_centers['RL'][0])

# Recommended raycast anchors at nominal rest: wheel center + spring current length (50% of configured travel).
# Uses current Formula-90 baseline ratios only as placement guidance, not a handling retune.
recommended_raycast={
 'WheelFrontLeft':wheel_centers['FL']+np.array([0,0.05,0]),
 'WheelFrontRight':wheel_centers['FR']+np.array([0,0.05,0]),
 'WheelRearLeft':wheel_centers['RL']+np.array([0,0.06,0]),
 'WheelRearRight':wheel_centers['RR']+np.array([0,0.06,0]),
}
meta={
 'asset':'F1_94_GEVP_READY',
 'source':'Blockbench OBJ repaired deterministically',
 'units':'meters',
 'coordinate_convention':{
     'X':'+right / -left', 'Y':'+up', 'Z':'front axle at negative Z; rear axle at positive Z',
     'origin':'midpoint between front/rear axle centers, at mean wheel-center height',
     'transform_from_source':'rotate 180 degrees around Y, then translate to canonical origin',
 },
 'dimensions_m':{
     'wheelbase':round(float(wheelbase),6),'front_track_center':round(float(front_track),6),'rear_track_center':round(float(rear_track),6),
     'front_tire_radius_visual':round(float(front_radius),6),'rear_tire_radius_visual':round(float(rear_radius),6),
     'front_tire_width_visual':round(float(front_width),6),'rear_tire_width_visual':round(float(rear_width),6),
 },
 'wheel_centers_m':{k:f3(v) for k,v in wheel_centers.items()},
 'gevp_mapping':{
     'WheelFrontLeft':'JNT_WHEEL_FL','WheelFrontRight':'JNT_WHEEL_FR','WheelRearLeft':'JNT_WHEEL_RL','WheelRearRight':'JNT_WHEEL_RR',
     'wheel_node_rule':'Keep GEVP RayCast3D -> wheel_node Pivot -> Visual hierarchy. Do not bake persistent visual offsets into the GEVP-controlled pivot.',
     'recommended_raycast_anchor_m_at_50pct_rest':{k:f3(v) for k,v in recommended_raycast.items()},
 },
 'joints_and_datums_m':{k:f3(v) for k,v in sorted(joints.items())},
 'suspension_arms_detected':susp_joint_summary,
 'geometry_names':{k:rename[k] for k in objects},
 'rendering':{'material':'F1_94_PIXEL_MATERIAL','culling':'back-face enabled','alpha_mode':'MASK','alpha_cutoff':0.5,'texture_filter':'NEAREST embedded in GLB'},
 'repair':{
     'faces':len(faces),'source_vertices':len(vertices),'source_uvs':len(texcoords),'degenerate_faces':int(sum(np.linalg.norm(np.cross(Vn[[c[0] for c in f['corners']]][1]-Vn[[c[0] for c in f['corners']]][0],Vn[[c[0] for c in f['corners']]][2]-Vn[[c[0] for c in f['corners']]][0]))<1e-12 for f in faces)),
     'faces_reversed':int(sum(face_flip.values())),'objects':len(objects),'topology_by_object':repair_stats,
 }
}
META_OUT.write_text(json.dumps(meta,indent=2,default=lambda o: o.item() if isinstance(o, np.generic) else (_ for _ in ()).throw(TypeError(type(o)))),encoding='utf-8')

# ---------- Validate exported files ----------
validation=[]
for p in [FULL_GLB,CHASSIS_GLB,WHEEL_FRONT_GLB,WHEEL_REAR_GLB]:
    try:
        sc=trimesh.load(p,force='scene',process=False)
        geos=len(sc.geometry); bounds=np.asarray(sc.bounds)
        validation.append(f'{p.name}: OK, geometries={geos}, bounds={np.round(bounds,6).tolist()}')
    except Exception as e:
        validation.append(f'{p.name}: FAILED: {e}')

# validate OBJ with trimesh too
try:
    sc=trimesh.load(OBJ_OUT,force='scene',process=False)
    validation.append(f'{OBJ_OUT.name}: OK, geometries={len(sc.geometry)}, bounds={np.round(sc.bounds,6).tolist()}')
except Exception as e:
    validation.append(f'{OBJ_OUT.name}: FAILED: {e}')

REPORT_OUT.write_text(
    'F1-94 deterministic repair report\n'
    '================================\n\n'
    f'Source faces: {len(faces)}\nObjects: {len(objects)}\nFaces reversed after topology/orientation analysis: {sum(face_flip.values())}\n'
    f'Wheelbase: {wheelbase:.6f} m\nFront track (visual tire center): {front_track:.6f} m\nRear track (visual tire center): {rear_track:.6f} m\n'
    f'Front visual tire radius: {front_radius:.6f} m\nRear visual tire radius: {rear_radius:.6f} m\n'
    f'Origin shift from source-after-180Y: {origin_shift.tolist()}\n\n'
    'Repairs performed:\n'
    '- Rotated source 180 deg around Y to match Formula-90 current GEVP scene semantics (FL x<0,z<0).\n'
    '- Rebased origin to axle midpoint and mean wheel-center height.\n'
    '- Reconstructed topological adjacency from duplicated Blockbench vertices.\n'
    '- Corrected inconsistent/inward winding per connected shell/surface.\n'
    '- Recomputed flat face normals from corrected winding.\n'
    '- Preserved original UV coordinates and 64x64 texture atlas.\n'
    '- Embedded texture in GLB with nearest-neighbor sampler and alpha mask.\n'
    '- Added explicit geometry names, wheel pivots, suspension-arm endpoints, axle datums and wing/nose mount markers.\n'
    '- Exported canonical reusable front-left and rear-left wheel assets with wheel center at local origin.\n\n'
    'Validation:\n'+'\n'.join('- '+x for x in validation)+'\n',encoding='utf-8')

NOTES_OUT.write_text(
'''F1-94 / Formula-90 / GEVP import notes
=======================================

Primary file: F1_94_GEVP_READY.glb
For the current GEVP architecture, use F1_94_chassis.glb as the chassis visual and the canonical wheel files as wheel visuals.

Coordinate convention used here
- +X = vehicle right, -X = vehicle left
- +Y = up
- front axle = negative Z
- rear axle = positive Z
- origin = midpoint of the two axle centers, at mean wheel-center height
- units = meters

GEVP hierarchy
VehicleRigidBody
  WheelFrontLeft   (RayCast3D / GEVP Wheel)
    FrontLeftWheel (Node3D assigned to wheel_node)
      Visual       (instance F1_94_wheel_front.glb)
  WheelFrontRight
    FrontRightWheel
      Visual       (reuse F1_94_wheel_front.glb; rotate the visual 180 deg around Y if required by sidewall orientation)
  WheelRearLeft
    RearLeftWheel
      Visual       (instance F1_94_wheel_rear.glb)
  WheelRearRight
    RearRightWheel
      Visual       (reuse F1_94_wheel_rear.glb; rotate the visual 180 deg around Y if required)

Important: GEVP changes wheel_node.position.y and wheel_node.rotation.x at runtime. Keep calibration offsets below that pivot, on the Visual node, not on the GEVP-controlled wheel_node itself.

The exact marker coordinates are in F1_94_gevp_metadata.json. The GLB also contains empty nodes named JNT_* and DATUM_*.

Rendering
- Correct winding + recomputed normals; material is not double-sided.
- GLB texture sampler is nearest-neighbor for pixel-art preservation.
- Alpha is MASK at 0.5. The source atlas has transparent unused pixels, but UV triangles do not cover them.
- Do not enable smooth shading if the PS1/low-poly faceted appearance is desired.
''',encoding='utf-8')

# Copy script itself for reproducibility
shutil.copy2(Path(__file__),SCRIPT_OUT)

# Zip package
if ZIP_OUT.exists(): ZIP_OUT.unlink()
with zipfile.ZipFile(ZIP_OUT,'w',zipfile.ZIP_DEFLATED) as z:
    for p in sorted(OUT.iterdir()): z.write(p,arcname=p.name)

print('OUTPUT',ZIP_OUT)
print(REPORT_OUT.read_text())
