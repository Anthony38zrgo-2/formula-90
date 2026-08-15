import os
import sys
import numpy as np
import trimesh

chassis_path = "game/assets/models/vehicles/jordan_197/jordan_197_chassis.glb"
wheel_fl_path = "game/assets/models/vehicles/jordan_197/jordan_197_wheel_fl.glb"
wheel_fr_path = "game/assets/models/vehicles/jordan_197/jordan_197_wheel_fr.glb"
wheel_rl_path = "game/assets/models/vehicles/jordan_197/jordan_197_wheel_rl.glb"
wheel_rr_path = "game/assets/models/vehicles/jordan_197/jordan_197_wheel_rr.glb"

print("=== LOADING GLB MESHES ===")
chassis_scene = trimesh.load(chassis_path)
wheel_fl_scene = trimesh.load(wheel_fl_path)
wheel_fr_scene = trimesh.load(wheel_fr_path)
wheel_rl_scene = trimesh.load(wheel_rl_path)
wheel_rr_scene = trimesh.load(wheel_rr_path)

def get_mesh(scene_or_mesh):
    if isinstance(scene_or_mesh, trimesh.Scene):
        return trimesh.util.concatenate([g for g in scene_or_mesh.geometry.values() if isinstance(g, trimesh.Trimesh)])
    return scene_or_mesh

chassis_mesh = get_mesh(chassis_scene)
wheel_fl = get_mesh(wheel_fl_scene)
wheel_fr = get_mesh(wheel_fr_scene)
wheel_rl = get_mesh(wheel_rl_scene)
wheel_rr = get_mesh(wheel_rr_scene)

print(f"Chassis: {len(chassis_mesh.vertices)} verts, bounds: {chassis_mesh.bounds}")
print(f"Wheel FL: {len(wheel_fl.vertices)} verts, bounds: {wheel_fl.bounds}, centroid: {wheel_fl.centroid}")
print(f"Wheel FR: {len(wheel_fr.vertices)} verts, bounds: {wheel_fr.bounds}, centroid: {wheel_fr.centroid}")
print(f"Wheel RL: {len(wheel_rl.vertices)} verts, bounds: {wheel_rl.bounds}, centroid: {wheel_rl.centroid}")
print(f"Wheel RR: {len(wheel_rr.vertices)} verts, bounds: {wheel_rr.bounds}, centroid: {wheel_rr.centroid}")

# Find suspension arm endpoints on the chassis mesh
# Front suspension is around Z < -0.8 and abs(X) > 0.4
# Rear suspension is around Z > 0.8 and abs(X) > 0.4
verts = chassis_mesh.vertices

# Let's inspect sub-meshes in chassis_scene to find named parts (suspension, body, etc.)
if isinstance(chassis_scene, trimesh.Scene):
    print("\nChassis sub-nodes / geometries:")
    for name, geom in chassis_scene.geometry.items():
        print(f"  Geom: {name}, verts: {len(geom.vertices)}, bounds: {geom.bounds}")

# Let's find the outermost vertices on left and right for front and rear suspension
fl_susp_mask = (verts[:, 2] < -1.0) & (verts[:, 2] > -1.8) & (verts[:, 0] < -0.6)
fr_susp_mask = (verts[:, 2] < -1.0) & (verts[:, 2] > -1.8) & (verts[:, 0] > 0.6)
rl_susp_mask = (verts[:, 2] > 1.2) & (verts[:, 2] < 2.0) & (verts[:, 0] < -0.6)
rr_susp_mask = (verts[:, 2] > 1.2) & (verts[:, 2] < 2.0) & (verts[:, 0] > 0.6)

print("\n=== SUSPENSION ARM ATTACHMENT HUBS ON CHASSIS ===")
if np.any(fl_susp_mask):
    fl_pts = verts[fl_susp_mask]
    # The hub center is at the extreme lateral tip (min X)
    tip_idx = np.argmin(fl_pts[:, 0])
    print(f"Front Left Susp tip: {fl_pts[tip_idx]}, centroid of outer cluster: {np.mean(fl_pts, axis=0)}")
if np.any(fr_susp_mask):
    fr_pts = verts[fr_susp_mask]
    tip_idx = np.argmax(fr_pts[:, 0])
    print(f"Front Right Susp tip: {fr_pts[tip_idx]}, centroid of outer cluster: {np.mean(fr_pts, axis=0)}")
if np.any(rl_susp_mask):
    rl_pts = verts[rl_susp_mask]
    tip_idx = np.argmin(rl_pts[:, 0])
    print(f"Rear Left Susp tip: {rl_pts[tip_idx]}, centroid of outer cluster: {np.mean(rl_pts, axis=0)}")
if np.any(rr_susp_mask):
    rr_pts = verts[rr_susp_mask]
    tip_idx = np.argmax(rr_pts[:, 0])
    print(f"Rear Right Susp tip: {rr_pts[tip_idx]}, centroid of outer cluster: {np.mean(rr_pts, axis=0)}")

