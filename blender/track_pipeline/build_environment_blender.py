from __future__ import annotations

import argparse
from pathlib import Path
import json
import math
import sys
import bpy

SCRIPT_DIR=Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path: sys.path.insert(0,str(SCRIPT_DIR))

from blender_output import atomic_export_glb,atomic_publish,atomic_save_blend
from procedural_assets_blender import create_guardrail_collision,create_guardrail_prototype,create_prototypes,instantiate_prototype,sample_centerline,terrain_height
from procedural_catalog import biome_from_config
from procedural_materials_blender import build_material_library

def args_after_double_dash(): return sys.argv[sys.argv.index("--")+1:] if "--" in sys.argv else []
def read_json(path): return json.loads(Path(path).read_text(encoding="utf-8"))
def build_guardrails(config,points,materials):
    sources,module_length=create_guardrail_prototype(config,materials); road_half=float(config["road"]["width_m"])*.5; lap=float(config["_centerline_length_m"]); created=0
    for segment in config["guardrails"]["segments"]:
        start=float(segment["start_fraction"]); end=float(segment["end_fraction"]); span=(end-start)%1.0
        if span<=1e-9: continue
        count=max(1,int(math.ceil(span*lap/module_length)))
        for i in range(count):
            fraction=(start+(i+.5)/count*span)%1.0; pos,tangent,normal=sample_centerline(points,fraction); side=1 if segment["side"]=="right" else -1; distance=road_half+float(segment["offset_from_edge_m"]); pos=(pos[0]+normal[0]*side*distance,pos[1]+normal[1]*side*distance); ground=terrain_height(config,fraction,side,distance); yaw=math.atan2(float(tangent[1]),float(tangent[0])); instantiate_prototype(sources,f"Guardrail_{segment['name']}_{i:03d}",pos[0],pos[1],ground,yaw,1.0); create_guardrail_collision(f"GuardrailCollision_{segment['name']}_{i:03d}",pos,tangent,module_length*1.02,ground,config); created+=1
    return created
def main():
    p=argparse.ArgumentParser(); p.add_argument("--config",required=True); ns=p.parse_args(args_after_double_dash()); cp=Path(ns.config).resolve(); repo=cp.parents[3]; config=read_json(cp); generated=repo/config["generated_dir"]; runtime=repo/config["runtime_dir"]; base=generated/"track_base.blend"
    if not base.exists(): raise RuntimeError(f"Base track missing: {base}. Run Base mode and validate it first.")
    center=read_json(generated/"centerline.json"); placements=read_json(generated/"placements.json"); config["_centerline_length_m"]=center["length_m"]; points=center["points_xz"]; bpy.ops.wm.open_mainfile(filepath=str(base)); materials=build_material_library(generated/"textures"); biome=biome_from_config(config); prototypes=create_prototypes(materials,config); placed=0
    for idx,item in enumerate(placements["placements"]):
        vid=item["variant_id"]
        if vid not in prototypes: raise RuntimeError(f"No Blender procedural prototype for {vid!r} in biome {biome.id!r}")
        x,z=item["position_xz"]; h=terrain_height(config,float(item["track_fraction"]),float(item["side"]),float(item["distance_from_center_m"])); instantiate_prototype(prototypes[vid],f"{item['category']}_{idx:04d}_{vid}",float(x),float(z),h,float(item["yaw_rad"]),float(item["scale"]),item.get("tint_rgb")); placed+=1
    guards=build_guardrails(config,points,materials); blend=generated/"track_environment.blend"; glb=runtime/f"{config['track_id']}_environment.glb"; live=runtime/f"{config['track_id']}.glb"; atomic_save_blend(blend,generated/"backups"/"environment"); atomic_export_glb(glb); atomic_publish(glb,live); print(f"[blender] biome={biome.id} procedural placements={placed}"); print(f"[blender] guardrail modules={guards}"); print(f"[blender] published runtime: {live}")
if __name__=="__main__": main()
