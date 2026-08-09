from __future__ import annotations

import argparse
from pathlib import Path
import json
import math
import sys

import bpy
from mathutils import Vector

SCRIPT_DIR=Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path: sys.path.insert(0,str(SCRIPT_DIR))

from blender_output import atomic_export_glb,atomic_publish,atomic_save_blend
from procedural_materials_blender import build_material_library
from terrain_grid import bank_degrees_at_fraction,build_heightfield,effective_far_ground_z


def args_after_double_dash(): return sys.argv[sys.argv.index("--")+1:] if "--" in sys.argv else []
def read_json(path): return json.loads(Path(path).read_text(encoding="utf-8"))
def clear_scene(): bpy.ops.object.select_all(action="SELECT"); bpy.ops.object.delete(use_global=False)
def godot_xz_to_blender(x,z,height=0.0): return Vector((float(x),-float(z),float(height)))
def cross_frame(points,i,config):
    n=len(points); prev=godot_xz_to_blender(*points[(i-1)%n]); nxt=godot_xz_to_blender(*points[(i+1)%n]); tangent=(nxt-prev).normalized(); normal=Vector((-tangent.y,tangent.x,0)); return tangent,normal,math.radians(bank_degrees_at_fraction(config,i/n))
def mesh_object(name,verts,faces,materials=None,planar_uv_scale_m=None):
    mesh=bpy.data.meshes.new(name+"Mesh"); mesh.from_pydata(verts,[],faces); mesh.update(); obj=bpy.data.objects.new(name,mesh); bpy.context.scene.collection.objects.link(obj)
    for mat in materials or []: obj.data.materials.append(mat)
    if planar_uv_scale_m:
        uv=mesh.uv_layers.new(name="UVMap"); scale=max(float(planar_uv_scale_m),.001)
        for poly in mesh.polygons:
            for loop in poly.loop_indices:
                vi=mesh.loops[loop].vertex_index; co=mesh.vertices[vi].co; uv.data[loop].uv=(co.x/scale,co.y/scale)
    return obj
def build_ribbon(points,half_width,top_z,name,material,collision_name,config):
    n=len(points); left=[]; right=[]
    for i,p in enumerate(points):
        _,normal,bank=cross_frame(points,i,config); center=godot_xz_to_blender(p[0],p[1],top_z); rise=math.tan(bank)*half_width; left.append(center-normal*half_width+Vector((0,0,-rise))); right.append(center+normal*half_width+Vector((0,0,rise)))
    verts=[]
    for a,b in zip(left,right): verts.extend([tuple(a),tuple(b)])
    faces=[(2*i,2*((i+1)%n),2*((i+1)%n)+1,2*i+1) for i in range(n)]
    road=mesh_object(name,verts,faces,[material],5.0); col=mesh_object(collision_name,verts,faces); col.hide_render=True; col.display_type="WIRE"; return road
def build_edge_line(points,side,road_half,line_width,config,material,top_z):
    n=len(points); sign=1 if side=="right" else -1; verts=[]; inner=road_half-line_width
    for i,p in enumerate(points):
        _,normal,bank=cross_frame(points,i,config); center=godot_xz_to_blender(p[0],p[1],top_z+.003)
        for offset in (inner,road_half):
            signed=sign*offset; verts.append(tuple(center+normal*signed+Vector((0,0,math.tan(bank)*signed))))
    faces=[(2*i,2*((i+1)%n),2*((i+1)%n)+1,2*i+1) for i in range(n)]; return mesh_object(("Right" if sign>0 else "Left")+"EdgeLine",verts,faces,[material])
def curb_indices(n,start,end):
    a=int(math.floor(start*n))%n; b=int(math.ceil(end*n))%n; return list(range(a,b+1)) if a<=b else list(range(a,n))+list(range(0,b+1))
def build_curb(points,segment,road_half,profile,materials,config,top_z):
    indices=curb_indices(len(points),float(segment["start_fraction"]),float(segment["end_fraction"])); side=1 if segment["side"]=="right" else -1; verts=[]; rows=len(profile)
    for idx in indices:
        _,normal,bank=cross_frame(points,idx,config); normal*=side; p=points[idx]; center=godot_xz_to_blender(p[0],p[1],top_z); edge=center+normal*road_half+Vector((0,0,math.tan(bank)*road_half*side))
        for off,h in profile: verts.append(tuple(edge+normal*float(off)+Vector((0,0,float(h)+math.tan(bank)*float(off)*side))))
    faces=[]
    for r in range(len(indices)-1):
        for c in range(rows-1):
            a=r*rows+c; b=(r+1)*rows+c; faces.append((a,b,b+1,a+1))
    obj=mesh_object("Curb_"+segment["name"],verts,faces,[materials["curb_red"],materials["curb_white"]],1.0); stripe=max(.5,float(config["curb"].get("stripe_length_m",2))); spacing=float(config.get("sample_spacing_m",2)); per=max(1,rows-1)
    for pi,poly in enumerate(obj.data.polygons): poly.material_index=int((pi//per*spacing)//stripe)%2
    col=mesh_object("CurbCollision_"+segment["name"]+"-colonly",verts,faces); col.hide_render=True; col.display_type="WIRE"
def build_terrain(points,config,material):
    cverts,faces,stats=build_heightfield(points,config,visual=False); vverts,vfaces,_=build_heightfield(points,config,visual=True)
    if vfaces!=faces: raise RuntimeError("Visual/collision terrain topology diverged unexpectedly")
    convert=lambda vv:[(x,-z,y) for x,z,y in vv]; visual=mesh_object("GrassTerrainVisual",convert(vverts),faces,[material],float(config.get("terrain",{}).get("texture_world_size_m",96))); col=mesh_object("GrassTerrainCollision-colonly",convert(cverts),faces); col.hide_render=True; col.display_type="WIRE"; visual["formula90s_terrain_grid_cell_m"]=stats["cell_m"]; col["formula90s_terrain_grid_cell_m"]=stats["cell_m"]; return stats
def build_start_finish(config,material):
    width=float(config["road"]["width_m"]); line_width=float(config["start_finish"]["line_width_m"]); z=float(config["road"]["surface_elevation_m"]); bpy.ops.mesh.primitive_cube_add(location=(0,0,z+.008)); line=bpy.context.object; line.name="StartFinish"; line.scale=(width*.5,line_width*.5,.006); bpy.ops.object.transform_apply(location=False,rotation=False,scale=True); line.data.materials.append(material)
def build_spawn_marker(config):
    spawn=float(config["start_finish"]["spawn_before_m"]); z=float(config["road"]["surface_elevation_m"]); marker=bpy.data.objects.new("PlayerSpawn",None); marker.location=godot_xz_to_blender(0,spawn,z); bpy.context.scene.collection.objects.link(marker)
def main():
    p=argparse.ArgumentParser(); p.add_argument("--config",required=True); ns=p.parse_args(args_after_double_dash()); cp=Path(ns.config).resolve(); repo=cp.parents[3]; config=read_json(cp); center=read_json(repo/config["generated_dir"]/"centerline.json"); points=center["points_xz"]; clear_scene(); generated=repo/config["generated_dir"]; runtime=repo/config["runtime_dir"]; materials=build_material_library(generated/"textures"); z=float(config["road"]["surface_elevation_m"]); half=float(config["road"]["width_m"])*.5
    stats=build_terrain(points,config,materials["ground"]); build_ribbon(points,half,z,"RoadVisual",materials["asphalt"],"RoadCollision-colonly",config); lw=float(config["road"]["edge_line_width_m"]); build_edge_line(points,"left",half,lw,config,materials["edge_line"],z); build_edge_line(points,"right",half,lw,config,materials["edge_line"],z)
    for segment in config["curb"]["segments"]: build_curb(points,segment,half,config["curb"]["profile"],materials,config,z)
    build_start_finish(config,materials["start_finish"]); build_spawn_marker(config); generated.mkdir(parents=True,exist_ok=True); runtime.mkdir(parents=True,exist_ok=True); blend=generated/"track_base.blend"; glb=runtime/f"{config['track_id']}_base.glb"; live=runtime/f"{config['track_id']}.glb"; atomic_save_blend(blend,generated/"backups"/"base"); atomic_export_glb(glb); atomic_publish(glb,live); print(f"[blender] terrain grid vertices={stats['vertices']} triangles={stats['triangles']} cell={stats['cell_m']:.2f}m"); print(f"[blender] terrain far z={effective_far_ground_z(config):.3f}m"); print(f"[blender] published runtime: {live}")

if __name__=="__main__": main()
