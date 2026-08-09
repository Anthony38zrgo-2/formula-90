from __future__ import annotations

import argparse
from pathlib import Path
import json
import math
import sys

import bpy
from mathutils import Vector


def args_after_double_dash():
    return sys.argv[sys.argv.index("--")+1:] if "--" in sys.argv else []


def read_json(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def clear_scene():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)


def godot_xz_to_blender(x: float, z: float, height: float = 0.0) -> Vector:
    return Vector((float(x), -float(z), float(height)))


def material(name, color, metallic=0.0, roughness=0.8):
    mat = bpy.data.materials.new(name)
    mat.diffuse_color = (*color,1.0)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = (*color,1.0)
    bsdf.inputs["Metallic"].default_value = metallic
    bsdf.inputs["Roughness"].default_value = roughness
    return mat


def mesh_object(name, verts, faces, mat=None):
    mesh = bpy.data.meshes.new(name+"Mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(obj)
    if mat:
        obj.data.materials.append(mat)
    return obj


def bank_degrees_at_fraction(config, fraction: float) -> float:
    result = 0.0
    f = fraction % 1.0
    for zone in config.get("banking", []):
        center = float(zone["center_fraction"]) % 1.0
        half = max(float(zone["half_width_fraction"]), 1e-6)
        d = abs((f-center+0.5)%1.0-0.5)
        if d <= half:
            weight = 0.5 * (1.0 + math.cos(math.pi * d / half))
            result += float(zone["degrees"]) * weight
    return result


def cross_frame(points, i, config):
    n = len(points)
    prev = godot_xz_to_blender(*points[(i-1)%n])
    nxt = godot_xz_to_blender(*points[(i+1)%n])
    tangent = (nxt-prev).normalized()
    normal = Vector((-tangent.y, tangent.x, 0.0))
    bank = math.radians(bank_degrees_at_fraction(config, i/n))
    return tangent, normal, bank


def build_ribbon(points, half_width, top_z, name, mat, collision_name, config):
    n=len(points)
    left=[]; right=[]
    for i,p in enumerate(points):
        _,normal,bank=cross_frame(points,i,config)
        center=godot_xz_to_blender(p[0],p[1],top_z)
        rise=math.tan(bank)*half_width
        left.append(center-normal*half_width+Vector((0,0,-rise)))
        right.append(center+normal*half_width+Vector((0,0,rise)))
    verts=[]
    for a,b in zip(left,right):
        verts.extend([tuple(a),tuple(b)])
    faces=[]
    for i in range(n):
        j=(i+1)%n
        faces.append((2*i,2*j,2*j+1,2*i+1))
    road=mesh_object(name,verts,faces,mat)
    collision=mesh_object(collision_name,verts,faces,None)
    collision.hide_render=True
    collision.display_type="WIRE"
    return road


def build_edge_line(points, side, road_half, line_width, config, mat):
    n=len(points)
    side_sign=1.0 if side=="right" else -1.0
    verts=[]; faces=[]
    inner=road_half-line_width
    outer=road_half
    for i,p in enumerate(points):
        _,normal,bank=cross_frame(points,i,config)
        center=godot_xz_to_blender(p[0],p[1],0.008)
        for offset in (inner,outer):
            signed=side_sign*offset
            rise=math.tan(bank)*signed
            v=center+normal*signed+Vector((0,0,rise))
            verts.append(tuple(v))
    for i in range(n):
        j=(i+1)%n
        faces.append((2*i,2*j,2*j+1,2*i+1))
    return mesh_object(("Right" if side_sign>0 else "Left")+"EdgeLine",verts,faces,mat)


def curb_indices(n, start_fraction, end_fraction):
    start=int(math.floor(start_fraction*n))%n
    end=int(math.ceil(end_fraction*n))%n
    if start <= end:
        return list(range(start,end+1))
    return list(range(start,n))+list(range(0,end+1))


def build_curb(points, segment, road_half, profile, mat, config):
    indices=curb_indices(len(points),float(segment["start_fraction"]),float(segment["end_fraction"]))
    side=1.0 if segment["side"]=="right" else -1.0
    verts=[]; faces=[]
    rows=len(profile)
    for idx in indices:
        _,normal,bank=cross_frame(points,idx,config)
        normal*=side
        p=points[idx]
        center=godot_xz_to_blender(p[0],p[1],0.006)
        road_edge_rise=math.tan(bank)*road_half*side
        edge=center+normal*road_half+Vector((0,0,road_edge_rise))
        for offset,height in profile:
            cross_rise=math.tan(bank)*float(offset)*side
            v=edge+normal*float(offset)+Vector((0,0,float(height)+cross_rise))
            verts.append(tuple(v))
    for r in range(len(indices)-1):
        for c in range(rows-1):
            a=r*rows+c; b=(r+1)*rows+c
            faces.append((a,b,b+1,a+1))
    obj=mesh_object("Curb_"+segment["name"],verts,faces,mat)
    col=mesh_object("CurbCollision_"+segment["name"]+"-colonly",verts,faces,None)
    col.hide_render=True
    col.display_type="WIRE"
    return obj


def build_start_finish(config):
    width=float(config["road"]["width_m"])
    line_width=float(config["start_finish"]["line_width_m"])
    bpy.ops.mesh.primitive_cube_add(location=(0,0,0.013))
    line=bpy.context.object
    line.name="StartFinish"
    line.scale=(width*0.5,line_width*0.5,0.006)
    bpy.ops.object.transform_apply(location=False,rotation=False,scale=True)

    mat=bpy.data.materials.new("StartFinishChecker")
    mat.use_nodes=True
    nodes=mat.node_tree.nodes
    links=mat.node_tree.links
    for node in list(nodes):
        nodes.remove(node)
    out=nodes.new("ShaderNodeOutputMaterial")
    bsdf=nodes.new("ShaderNodeBsdfPrincipled")
    checker=nodes.new("ShaderNodeTexChecker")
    tex=nodes.new("ShaderNodeTexCoord")
    mapping=nodes.new("ShaderNodeMapping")
    checker.inputs["Color1"].default_value=(0.02,0.02,0.02,1)
    checker.inputs["Color2"].default_value=(0.95,0.95,0.95,1)
    checker.inputs["Scale"].default_value=8.0
    bsdf.inputs["Roughness"].default_value=0.72
    links.new(tex.outputs["Generated"],mapping.inputs["Vector"])
    links.new(mapping.outputs["Vector"],checker.inputs["Vector"])
    links.new(checker.outputs["Color"],bsdf.inputs["Base Color"])
    links.new(bsdf.outputs["BSDF"],out.inputs["Surface"])
    line.data.materials.append(mat)


def build_spawn_marker(config):
    spawn=float(config["start_finish"]["spawn_before_m"])
    marker=bpy.data.objects.new("PlayerSpawn",None)
    marker.location=(0.0,spawn,0.0)
    bpy.context.scene.collection.objects.link(marker)


def main():
    p=argparse.ArgumentParser()
    p.add_argument("--config",required=True)
    ns=p.parse_args(args_after_double_dash())
    config_path=Path(ns.config).resolve()
    repo=config_path.parents[3]
    config=read_json(config_path)
    center=read_json(repo/config["generated_dir"]/ "centerline.json")
    points=center["points_xz"]

    clear_scene()
    road_mat=material("Road",(0.17,0.17,0.18),roughness=0.96)
    curb_mat=material("CurbBase",(0.82,0.08,0.06),roughness=0.78)
    line_mat=material("EdgeLine",(0.94,0.94,0.92),roughness=0.86)
    ground_mat=material("DryGrassGround",(0.22,0.29,0.16),roughness=1.0)

    road_half=float(config["road"]["width_m"])*0.5
    build_ribbon(points,road_half,0.006,"RoadVisual",road_mat,"RoadCollision-colonly",config)
    line_width=float(config["road"]["edge_line_width_m"])
    build_edge_line(points,"left",road_half,line_width,config,line_mat)
    build_edge_line(points,"right",road_half,line_width,config,line_mat)

    for segment in config["curb"]["segments"]:
        build_curb(points,segment,road_half,config["curb"]["profile"],curb_mat,config)

    xs=[p[0] for p in points]; zs=[p[1] for p in points]
    sx=(max(xs)-min(xs))+180.0; sy=(max(zs)-min(zs))+180.0
    cx=(max(xs)+min(xs))*0.5; godot_cz=(max(zs)+min(zs))*0.5
    cy=-godot_cz

    bpy.ops.mesh.primitive_plane_add(size=2.0,location=(cx,cy,-0.01))
    ground=bpy.context.object
    ground.name="GrassVisual"
    ground.scale=(sx*0.5,sy*0.5,1.0)
    bpy.ops.object.transform_apply(location=False,rotation=False,scale=True)
    ground.data.materials.append(ground_mat)

    bpy.ops.mesh.primitive_plane_add(size=2.0,location=(cx,cy,-0.012))
    ground_col=bpy.context.object
    ground_col.name="GrassCollision-colonly"
    ground_col.scale=(sx*0.5,sy*0.5,1.0)
    bpy.ops.object.transform_apply(location=False,rotation=False,scale=True)
    ground_col.hide_render=True
    ground_col.display_type="WIRE"

    build_start_finish(config)
    build_spawn_marker(config)

    generated=repo/config["generated_dir"]
    runtime=repo/config["runtime_dir"]
    generated.mkdir(parents=True,exist_ok=True)
    runtime.mkdir(parents=True,exist_ok=True)

    blend=generated/"track_base.blend"
    glb=runtime/f"{config['track_id']}_base.glb"
    bpy.ops.wm.save_as_mainfile(filepath=str(blend))
    bpy.ops.export_scene.gltf(filepath=str(glb), export_format="GLB", export_apply=True, export_extras=True, use_visible=True)
    print(f"[blender] base blend: {blend}")
    print(f"[blender] base glb: {glb}")


if __name__=="__main__":
    main()
