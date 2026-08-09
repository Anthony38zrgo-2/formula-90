from __future__ import annotations

import argparse
from pathlib import Path
import numpy as np

from pipeline_common import read_json,closed_polyline_length,count_self_intersections,signed_curvature
from terrain_grid import validate_heightfield,effective_far_ground_z


def main()->int:
    parser=argparse.ArgumentParser(description="Validate deterministic track and terrain geometry before Blender generation."); parser.add_argument("--config",required=True); parser.add_argument("--max-length-error-percent",type=float,default=.5); args=parser.parse_args()
    cp=Path(args.config).resolve(); repo=cp.parents[3]; config=read_json(cp); center=read_json(repo/config["generated_dir"]/"centerline.json"); points=np.asarray(center["points_xz"],dtype=float)
    length=closed_polyline_length(points); target=float(center["target_length_m"]); error=abs(length-target)/target*100; intersections=count_self_intersections(points); curvature=np.abs(signed_curvature(points)); nz=curvature[curvature>1e-7]; min_radius=float(1/nz.max()) if len(nz) else float("inf")
    curb_profile=config["curb"]["profile"]; curb_max=max(float(p[1]) for p in curb_profile); curb_width=float(config["curb"]["width_m"]); road_width=float(config["road"]["width_m"]); surface=float(config["road"].get("surface_elevation_m",0)); terrain=validate_heightfield(points.tolist(),config); grid=float(config.get("terrain",{}).get("grid_cell_m",8)); sink=float(config.get("terrain",{}).get("visual_sink_m",0))
    checks={"length_error_percent":error<=args.max_length_error_percent,"centerline_self_intersections":intersections==0,"road_width_positive":road_width>=8,"surface_elevation_small_positive":.005<=surface<=.08,"curb_width_reasonable":.20<=curb_width<=1.20,"curb_height_reasonable":0<=curb_max<=.050,"terrain_grid_cell_reasonable":3<=grid<=12,"terrain_visual_sink_visual_only":0<=sink<=.010,"terrain_vertices_finite":bool(terrain["finite_vertices"]),"terrain_triangles_nondegenerate":bool(terrain["nondegenerate_triangles"]),"terrain_collision_seam_continuous":float(terrain["max_collision_seam_error_m"])<=1e-6,"terrain_triangle_budget":int(terrain["triangles"])<=100000}
    print("TRACK VALIDATION"); print(f" length: {length:.3f} m / target {target:.3f} m ({error:.4f}% error)"); print(f" centerline self intersections: {intersections}"); print(f" minimum sampled radius: {min_radius:.2f} m"); print(f" road width: {road_width:.2f} m surface elevation={surface*1000:.1f} mm"); print(f" curb: width={curb_width:.3f} m max_height={curb_max*1000:.1f} mm"); print(f" terrain grid: {terrain['vertices']} vertices / {terrain['triangles']} triangles / cell={terrain['cell_m']:.2f} m"); print(f" terrain collision seam max error: {float(terrain['max_collision_seam_error_m'])*1000:.4f} mm"); print(f" terrain far z: {effective_far_ground_z(config):.3f} m")
    for name,ok in checks.items(): print(f" {'PASS' if ok else 'FAIL'} {name}")
    return 0 if all(checks.values()) else 2

if __name__=="__main__": raise SystemExit(main())
