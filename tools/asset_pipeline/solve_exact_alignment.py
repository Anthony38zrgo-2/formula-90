import os
import sys
import numpy as np
import trimesh
from scipy.spatial import cKDTree

# Load meshes
chassis_scene = trimesh.load("game/assets/models/vehicles/jordan_197/jordan_197_chassis.glb")
wheel_fl_scene = trimesh.load("game/assets/models/vehicles/jordan_197/jordan_197_wheel_fl.glb")
wheel_fr_scene = trimesh.load("game/assets/models/vehicles/jordan_197/jordan_197_wheel_fr.glb")
wheel_rl_scene = trimesh.load("game/assets/models/vehicles/jordan_197/jordan_197_wheel_rl.glb")
wheel_rr_scene = trimesh.load("game/assets/models/vehicles/jordan_197/jordan_197_wheel_rr.glb")

def get_mesh(s):
    if isinstance(s, trimesh.Scene):
        return trimesh.util.concatenate([g for g in s.geometry.values() if isinstance(g, trimesh.Trimesh)])
    return s

wheel_fl = get_mesh(wheel_fl_scene)
wheel_fr = get_mesh(wheel_fr_scene)
wheel_rl = get_mesh(wheel_rl_scene)
wheel_rr = get_mesh(wheel_rr_scene)

# Suspension geometry on chassis
v_front = chassis_scene.geometry["SUSPENSION_RF"].vertices
v_fl = v_front[v_front[:, 0] < 0]
v_fr = v_front[v_front[:, 0] > 0]
v_rl = chassis_scene.geometry["SUSPENSION_LR"].vertices
v_rr = chassis_scene.geometry["SUSPENSION_RR"].vertices

# Extract outer hub rings / wishbone endpoints
fl_outer = v_fl[v_fl[:, 0] < v_fl[:, 0].min() + 0.03]
fr_outer = v_fr[v_fr[:, 0] > v_fr[:, 0].max() - 0.03]
rl_outer = v_rl[v_rl[:, 0] < v_rl[:, 0].min() + 0.03]
rr_outer = v_rr[v_rr[:, 0] > v_rr[:, 0].max() - 0.03]

hub_fl = np.mean(fl_outer, axis=0)
hub_fr = np.mean(fr_outer, axis=0)
hub_rl = np.mean(rl_outer, axis=0)
hub_rr = np.mean(rr_outer, axis=0)

print("Raw Hub Centers in Chassis Space (Chassis origin at 0,0,0):")
print(f"  Front Left  Hub: {hub_fl}")
print(f"  Front Right Hub: {hub_fr}")
print(f"  Rear Left   Hub: {hub_rl}")
print(f"  Rear Right  Hub: {hub_rr}")

# Symmetrize in chassis space:
# Front axle datum:
front_z_chassis = (hub_fl[2] + hub_fr[2]) * 0.5
front_y_chassis = (hub_fl[1] + hub_fr[1]) * 0.5
front_track_half = (abs(hub_fl[0]) + abs(hub_fr[0])) * 0.5

# Rear axle datum:
rear_z_chassis = (hub_rl[2] + hub_rr[2]) * 0.5
rear_y_chassis = (hub_rl[1] + hub_rr[1]) * 0.5
rear_track_half = (abs(hub_rl[0]) + abs(hub_rr[0])) * 0.5

print("\nSymmetrical Suspension Datums in Chassis Space:")
print(f"  Front Axle: Z = {front_z_chassis:.6f}, Y = {front_y_chassis:.6f}, Half-Track = {front_track_half:.6f}")
print(f"  Rear Axle:  Z = {rear_z_chassis:.6f}, Y = {rear_y_chassis:.6f}, Half-Track = {rear_track_half:.6f}")
print(f"  Wheelbase on Chassis: {abs(rear_z_chassis - front_z_chassis):.6f} m")

# Now let's calculate the Wheel Hub centers on the wheel meshes:
# Wheel origin is at (0,0,0) (the center of rotation of the wheel).
# The rim inner mounting flange is at X:
fl_rim_inner = wheel_fl.bounds[1, 0] # max X (facing chassis)
rl_rim_inner = wheel_rl.bounds[1, 0]

print(f"\nWheel Hub Mounting Centers:")
print(f"  Front Wheel: Center = (0,0,0), Inner Rim Edge = {fl_rim_inner:.6f}, Half Width = {wheel_fl.bounds[1,0]:.6f}")
print(f"  Rear Wheel:  Center = (0,0,0), Inner Rim Edge = {rl_rim_inner:.6f}, Half Width = {wheel_rl.bounds[1,0]:.6f}")

# The Wheel RayCast X position should place the inner rim right against the suspension hub
# X_raycast_front = front_track_half + offset to wheel center (0.1278 m approx)
# Standard F1 1997 track: Front track = 1.762m (half = 0.881m), Rear track = 1.748m (half = 0.874m)
x_fl = -0.881
x_fr = 0.881
x_rl = -0.874
x_rr = 0.874

# Wheel RayCast Y position is the suspension anchor height
# Standard hub height at resting ratio is Y = 0.320 m
# Let's align ChassisVisual translation (Tx, Ty, Tz) such that:
# 1. Front suspension hub matches Front Wheel Z
# 2. Rear suspension hub matches Rear Wheel Z
# 3. Left/Right symmetry is strictly 0.0 on Tx
# 4. Wheelbase matches (rear_Z - front_Z)

# If we set RayCasts at Z_front = front_z_chassis + Tz, and Z_rear = rear_z_chassis + Tz:
# Or if ChassisVisual has translation (0, Ty, Tz):
# Let's see what happens if ChassisVisual is placed at (0, 0, 0) vs shifted:
# In standard vehicle manifest:
# front_axle_center_runtime_m = [0, 0.32, -1.433]
# rear_axle_center_runtime_m = [0, 0.32, 1.640]
# But on chassis mesh, front_z is -1.5337 and rear_z is +1.5200!
# Note the difference:
# -1.433 - (-1.5337) = +0.1007 m
# +1.640 - (+1.5200) = +0.1200 m
# The suspension wishbones on the chassis were modeled with front at -1.5337 and rear at +1.5200!
# Wheelbase on chassis = 1.5200 - (-1.5337) = 3.0537 m (Wheelbase in manifest = 3.073 m, difference only 19 mm).

# If we place RayCasts exactly at the chassis suspension wishbone hubs:
# Front RayCast: Z = -1.5337 m (or symmetrically)
# Rear RayCast:  Z = +1.5200 m
# ChassisVisual: Position = Vector3(0, 0, 0) (perfect zero-offset alignment!)
# OR if RayCasts are placed at the exact wishbone Z, then Wheel Visual transform is strictly Vector3(0, 0, 0)!

print("\n=== EXACT ALIGNMENT CONFIGURATION ===")
print("Option A: Zero visual offsets (ChassisVisual at (0,0,0), Wheel Visuals at (0,0,0)), RayCasts at exact mesh hubs:")
print(f"  RayCast Front Left : ({-front_track_half - 0.1278:.4f}, {0.32:.4f}, {front_z_chassis:.4f})")
print(f"  RayCast Front Right: ({ front_track_half + 0.1278:.4f}, {0.32:.4f}, {front_z_chassis:.4f})")
print(f"  RayCast Rear Left  : ({-rear_track_half  - 0.1190:.4f}, {0.32:.4f}, {rear_z_chassis:.4f})")
print(f"  RayCast Rear Right : ({ rear_track_half  + 0.1190:.4f}, {0.32:.4f}, {rear_z_chassis:.4f})")
