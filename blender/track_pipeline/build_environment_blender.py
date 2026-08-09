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


def import_asset(path: Path):
    before=set(bpy.data.objects)
    if path.suffix.lower()==".blend":
        with bpy.data.libraries.load(str(path),link=False) as (src,dst):
            dst.objects=[name for name in src.objects if name]
        for obj in dst.objects:
            if obj and obj.name not in bpy.context.scene.collection.objects:
                bpy.context.scene.collection.objects.link(obj)
    else:
        bpy.ops.import_scene.gltf(filepath=str(path))
    bpy.context.view_layer.update()
    objs=[o for o in bpy.data.objects if o not in before]
    for o in objs:
        o.hide_render=True
        o.hide_viewport=True
    return objs


def godot_xz_to_blender(x: float, z: float, height: float = 0.0):
    return Vector((float(x),-float(z),float(height)))


def instantiate_group(source_objects, name, x, z, yaw, scale, tint):
    root=bpy.data.objects.new(name,None)
    bpy.context.scene.collection.objects.link(root)
    root.location=godot_xz_to_blender(x,z,0.0)
    root.rotation_euler[2]=-yaw
    root.scale=(scale,scale,scale)
    root["formula90s_tint_rgb"]=list(tint)
    for src in source_objects:
        copy=src.copy()
        if src.data is not None:
            copy.data=src.data
        copy.hide_render=False
        copy.hide_viewport=False
        bpy.context.scene.collection.objects.link(copy)
        copy.parent=root
    return root


def sample_centerline(points, fraction):
    n=len(points)
    f=(fraction%1.0)*n
    i=int(math.floor(f))%n
    t=f-math.floor(f)
    ax,az=points[i]
    bx,bz=points[(i+1)%n]
    x=ax+(bx-ax)*t
    z=az+(bz-az)*t
    tx=bx-ax; tz=bz-az
    length=max(math.hypot(tx,tz),1e-9)
    tx/=length; tz/=length
    nx=tz; nz=-tx
    return (x,z),(tx,tz),(nx,nz)


def guardrail_asset(catalog):
    valid=[a for a in catalog["assets"] if a["category"]=="guardrails" and a.get("valid",False)]
    if not valid:
        raise RuntimeError("No valid guardrail asset found under blender/infrastructure/guardrails.")
    return valid[0]


def guardrail_blender_yaw(tangent, asset):
    tx,tz=tangent
    yaw=math.atan2(-tz,tx)
    if str(asset.get("long_axis","X")).upper()=="Y":
        yaw-=math.pi*0.5
    yaw+=math.radians(float(asset.get("rotation_correction_deg",0.0)))
    return yaw


def create_guardrail_collision(name,pos,tangent,length,height,thickness):
    x,z=pos
    bpy.ops.mesh.primitive_cube_add(location=godot_xz_to_blender(x,z,height*0.5))
    obj=bpy.context.object
    obj.name=name+"-colonly"
    tx,tz=tangent
    obj.rotation_euler[2]=math.atan2(-tz,tx)
    obj.scale=(length*0.5,thickness*0.5,height*0.5)
    bpy.ops.object.transform_apply(location=False,rotation=False,scale=True)
    obj.hide_render=True
    obj.display_type="WIRE"
    return obj


def build_guardrails(repo,config,catalog,points,cache):
    asset=guardrail_asset(catalog)
    asset_path=repo/asset["path"]
    sources=cache.setdefault(asset["id"],import_asset(asset_path))
    module_length=max(0.5,max(asset["dimensions_m"][0],asset["dimensions_m"][1]))
    road_half=float(config["road"]["width_m"])*0.5
    col_h=float(config["guardrails"]["collision_height_m"])
    col_t=float(config["guardrails"]["collision_thickness_m"])
    lap_length=float(config["_centerline_length_m"])

    for segment in config["guardrails"]["segments"]:
        start,end=float(segment["start_fraction"]),float(segment["end_fraction"])
        span=(end-start)%1.0
        if span <= 1e-9:
            continue
        approx_len=span*lap_length
        count=max(1,int(math.ceil(approx_len/module_length)))
        for i in range(count):
            frac=(start+(i+0.5)/count*span)%1.0
            pos,tangent,normal=sample_centerline(points,frac)
            side=1.0 if segment["side"]=="right" else -1.0
            offset=road_half+float(segment["offset_from_edge_m"])
            pos=(pos[0]+normal[0]*side*offset,pos[1]+normal[1]*side*offset)
            yaw=guardrail_blender_yaw(tangent,asset)
            root=bpy.data.objects.new(f"Guardrail_{segment['name']}_{i:03d}",None)
            bpy.context.scene.collection.objects.link(root)
            root.location=godot_xz_to_blender(pos[0],pos[1],0.0)
            root.rotation_euler[2]=yaw
            for src in sources:
                copy=src.copy()
                if src.data is not None:
                    copy.data=src.data
                copy.hide_render=False
                copy.hide_viewport=False
                bpy.context.scene.collection.objects.link(copy)
                copy.parent=root
            create_guardrail_collision(f"GuardrailCollision_{segment['name']}_{i:03d}",pos,tangent,module_length*1.02,col_h,col_t)


def main():
    p=argparse.ArgumentParser()
    p.add_argument("--config",required=True)
    p.add_argument("--catalog",required=True)
    ns=p.parse_args(args_after_double_dash())

    config_path=Path(ns.config).resolve()
    repo=config_path.parents[3]
    config=read_json(config_path)
    center=read_json(repo/config["generated_dir"]/ "centerline.json")
    config["_centerline_length_m"]=center["length_m"]
    placements=read_json(repo/config["generated_dir"]/ "placements.json")
    catalog=read_json(repo/ns.catalog)
    points=center["points_xz"]

    base_blend=repo/config["generated_dir"]/ "track_base.blend"
    if not base_blend.exists():
        raise RuntimeError(f"Base track missing: {base_blend}. Run Base mode first.")
    bpy.ops.wm.open_mainfile(filepath=str(base_blend))

    by_id={a["id"]:a for a in catalog["assets"]}
    cache={}
    for idx,item in enumerate(placements["placements"]):
        asset=by_id[item["asset_id"]]
        sources=cache.setdefault(asset["id"],import_asset(repo/asset["path"]))
        x,z=item["position_xz"]
        instantiate_group(sources,f"{item['category']}_{idx:04d}",x,z,float(item["yaw_rad"]),float(item["scale"]),item["tint_rgb"])

    build_guardrails(repo,config,catalog,points,cache)

    generated=repo/config["generated_dir"]
    runtime=repo/config["runtime_dir"]
    blend=generated/"track_environment.blend"
    glb=runtime/f"{config['track_id']}_environment.glb"
    bpy.ops.wm.save_as_mainfile(filepath=str(blend))
    bpy.ops.export_scene.gltf(filepath=str(glb), export_format="GLB", export_apply=True, export_extras=True, use_visible=True)
    print(f"[blender] environment blend: {blend}")
    print(f"[blender] environment glb: {glb}")


if __name__=="__main__":
    main()
