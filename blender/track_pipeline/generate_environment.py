from __future__ import annotations

import argparse
from pathlib import Path
import math

import numpy as np

from pipeline_common import read_json,write_json,closed_polyline_length,interpolate_at_fraction,min_distance_to_closed_polyline,SpatialHash,Occupant,stable_rng
from procedural_catalog import biome_from_config,specs_for_biome,weighted_choice

DENSITIES=("none","very_low","low","medium","high")


def place_category(category,density,config,points,occupancy,seed):
    env=config["procedural_environment"]; profile=env["density_profiles"][density]; target=int(round(float(profile[f"{category}_per_km"])*closed_polyline_length(points)/1000.0)); zone=env["zones"][category]; min_d=float(zone["min_track_distance_m"]); max_d=float(zone["max_track_distance_m"]); biome=biome_from_config(config); specs=specs_for_biome(biome,category); rng=stable_rng(seed,f"{biome.id}:{category}")
    output=[]; attempts=0; max_attempts=max(500,target*100)
    while len(output)<target and attempts<max_attempts:
        attempts+=1; spec=weighted_choice(rng,specs); fraction=rng.random(); pos,tangent,normal=interpolate_at_fraction(points,fraction); side=-1.0 if rng.random()<.5 else 1.0; distance=rng.uniform(min_d,max_d); candidate=pos+normal*(side*distance); global_distance=min_distance_to_closed_polyline(candidate,points)
        if global_distance<min_d-.05 or global_distance>max_d+.75: continue
        scale=rng.uniform(spec.scale_min,spec.scale_max); radius=max(.05,spec.radius_m*scale); padding={"grass":.08,"bushes":.35,"trees":1.10,"fake_buildings":3.0}[category]
        if not occupancy.can_place(float(candidate[0]),float(candidate[1]),radius,padding): continue
        yaw=math.atan2(float(tangent[1]),float(tangent[0]))+rng.uniform(-.10,.10) if category=="fake_buildings" else rng.uniform(0,math.tau); strength={"grass":.035,"bushes":.045,"trees":.040,"fake_buildings":.020}[category]; tint=[round(1+rng.uniform(-strength,strength),4) for _ in range(3)]
        output.append({"category":category,"variant_id":spec.id,"position_xz":[round(float(candidate[0]),4),round(float(candidate[1]),4)],"track_fraction":round(float(fraction),7),"side":int(side),"distance_from_center_m":round(float(distance),4),"yaw_rad":round(float(yaw),6),"scale":round(float(scale),5),"radius_m":round(float(radius),4),"tint_rgb":tint}); occupancy.add(Occupant(float(candidate[0]),float(candidate[1]),radius,category,spec.id))
    if len(output)<target: raise RuntimeError(f"Could only place {len(output)}/{target} {category} after {attempts} attempts.")
    return output,attempts

def main()->int:
    p=argparse.ArgumentParser(description="Generate reproducible procedural environment placement."); p.add_argument("--config",required=True); p.add_argument("--trees-density",choices=DENSITIES,default="low"); p.add_argument("--bushes-density",choices=DENSITIES,default="low"); p.add_argument("--grass-density",choices=DENSITIES,default="medium"); p.add_argument("--buildings-density",choices=DENSITIES,default="very_low"); p.add_argument("--seed",type=int,default=1995); ns=p.parse_args(); cp=Path(ns.config).resolve(); repo=cp.parents[3]; config=read_json(cp); center=read_json(repo/config["generated_dir"]/"centerline.json"); points=np.asarray(center["points_xz"],dtype=float); biome=biome_from_config(config); occupancy=SpatialHash(cell_size=10); placements=[]; stats={}
    for category,density in (("fake_buildings",ns.buildings_density),("trees",ns.trees_density),("bushes",ns.bushes_density),("grass",ns.grass_density)):
        placed,attempts=place_category(category,density,config,points,occupancy,ns.seed); placements.extend(placed); stats[category]={"density":density,"placed":len(placed),"attempts":attempts}
    output=repo/config["generated_dir"]/"placements.json"; write_json(output,{"track_id":config["track_id"],"biome":biome.id,"seed":ns.seed,"stats":stats,"placements":placements}); print(f"[environment] biome={biome.id} wrote {output}")
    for category,stat in stats.items(): print(f"[environment] {category}: {stat['placed']} ({stat['density']}) attempts={stat['attempts']}")
    return 0

if __name__=="__main__": raise SystemExit(main())
