import os
import sys
import numpy as np
import trimesh

chassis_path = "game/assets/models/vehicles/jordan_197/jordan_197_chassis.glb"
chassis_scene = trimesh.load(chassis_path)

print("=== SUSPENSION DETAILED ANALYSIS ===")
for name in ["SUSPENSION_RF", "SUSPENSION_LR", "SUSPENSION_RR"]:
    geom = chassis_scene.geometry[name]
    v = geom.vertices
    print(f"\n--- {name} ({len(v)} vertices) ---")
    print(f"Bounds: min={v.min(axis=0)}, max={v.max(axis=0)}")

# Let's inspect vertices of SUSPENSION_RF to separate Front Left and Front Right
v_front = chassis_scene.geometry["SUSPENSION_RF"].vertices
v_front_left = v_front[v_front[:, 0] < 0]
v_front_right = v_front[v_front[:, 0] > 0]

print(f"\nFront Left suspension mesh bounds: min={v_front_left.min(axis=0)}, max={v_front_left.max(axis=0)}")
print(f"Front Right suspension mesh bounds: min={v_front_right.min(axis=0)}, max={v_front_right.max(axis=0)}")

# Find outer-most hub points (the upright / brake duct / axle hub)
# The wheel hub connects at the outermost lateral position
fl_outer = v_front_left[v_front_left[:, 0] < v_front_left[:, 0].min() + 0.05]
fr_outer = v_front_right[v_front_right[:, 0] > v_front_right[:, 0].max() - 0.05]

print(f"Front Left hub center (outer tip mean): {np.mean(fl_outer, axis=0)}")
print(f"Front Right hub center (outer tip mean): {np.mean(fr_outer, axis=0)}")

# Rear suspension:
v_rear_l = chassis_scene.geometry["SUSPENSION_LR"].vertices
v_rear_r = chassis_scene.geometry["SUSPENSION_RR"].vertices

rl_outer = v_rear_l[v_rear_l[:, 0] < v_rear_l[:, 0].min() + 0.05]
rr_outer = v_rear_r[v_rear_r[:, 0] > v_rear_r[:, 0].max() - 0.05]

print(f"\nRear Left suspension mesh bounds: min={v_rear_l.min(axis=0)}, max={v_rear_l.max(axis=0)}")
print(f"Rear Right suspension mesh bounds: min={v_rear_r.min(axis=0)}, max={v_rear_r.max(axis=0)}")
print(f"Rear Left hub center (outer tip mean): {np.mean(rl_outer, axis=0)}")
print(f"Rear Right hub center (outer tip mean): {np.mean(rr_outer, axis=0)}")

# Let's check wheel meshes hub mounting planes
wheel_fl = trimesh.load("game/assets/models/vehicles/jordan_197/jordan_197_wheel_fl.glb")
wheel_rl = trimesh.load("game/assets/models/vehicles/jordan_197/jordan_197_wheel_rl.glb")
w_fl_v = trimesh.util.concatenate([g for g in wheel_fl.geometry.values()]).vertices
w_rl_v = trimesh.util.concatenate([g for g in wheel_rl.geometry.values()]).vertices

print(f"\nWheel FL bounds: {w_fl_v.min(axis=0)} to {w_fl_v.max(axis=0)}")
print(f"Wheel FL inner face X (rim inside): {w_fl_v[:, 0].max() if w_fl_v[:, 0].min() < 0 else w_fl_v[:, 0].min()}")
print(f"Wheel RL bounds: {w_rl_v.min(axis=0)} to {w_rl_v.max(axis=0)}")
