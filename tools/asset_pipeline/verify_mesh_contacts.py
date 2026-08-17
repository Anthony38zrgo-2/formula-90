import numpy as np
import trimesh
from scipy.spatial import cKDTree

chassis_scene = trimesh.load("game/assets/models/vehicles/jordan_197/jordan_197_chassis.glb")
wheel_fl = trimesh.load("game/assets/models/vehicles/jordan_197/jordan_197_wheel_fl.glb")
wheel_fr = trimesh.load("game/assets/models/vehicles/jordan_197/jordan_197_wheel_fr.glb")
wheel_rl = trimesh.load("game/assets/models/vehicles/jordan_197/jordan_197_wheel_rl.glb")
wheel_rr = trimesh.load("game/assets/models/vehicles/jordan_197/jordan_197_wheel_rr.glb")

def get_mesh(s):
    if isinstance(s, trimesh.Scene):
        return trimesh.util.concatenate([g for g in s.geometry.values() if isinstance(g, trimesh.Trimesh)])
    return s

m_chassis = get_mesh(chassis_scene)
m_fl = get_mesh(wheel_fl)
m_fr = get_mesh(wheel_fr)
m_rl = get_mesh(wheel_rl)
m_rr = get_mesh(wheel_rr)

# Symmetrical RayCast positions aligned with suspension geometry:
# Front suspension wishbones are at Z = -1.5337 m, Y = 0.02 m (in chassis local space).
# In GEVP, when chassis is at (0,0,0), RayCast origin at Y = 0.32:
# If chassis wishbone is at Y = 0.02 (relative to origin), and wheel hub is at Y = 0.32:
# If Chassis is at Vector3(0, 0.30, 0): then Chassis wishbones sit at Y = 0.30 + 0.02 = 0.32 m!
# OR if Chassis is at Vector3(0, 0, 0) and RayCasts are at Y = 0.02 + 0.32 m?
# In GEVP: ChassisVisual is a child of VehicleRigidBody.
# When VehicleRigidBody is at rest:
# Wheel RayCasts are at position = Vector3(x, y, z).
# Let's test placing ChassisVisual at Vector3(0, 0, 0):

print("=== CHECKING CONTACTS & DISTANCES ===")

# Test 1: Symmetrical Front Axle at Z = -1.5337, X = +/- 0.881
# Test 2: Symmetrical Rear Axle at Z = 1.5193, X = +/- 0.874 (or 0.885)

for name, wheel, x, y, z in [
    ("FL", m_fl, -0.881, 0.0, -1.5337),
    ("FR", m_fr,  0.881, 0.0, -1.5337),
    ("RL", m_rl, -0.874, 0.0,  1.5193),
    ("RR", m_rr,  0.874, 0.0,  1.5193),
]:
    # Translate wheel vertices to candidate position
    w_verts = wheel.vertices + np.array([x, y, z])
    tree = cKDTree(m_chassis.vertices)
    dists, _ = tree.query(w_verts)
    min_dist = np.min(dists)
    print(f"Wheel {name} at ({x}, {y}, {z}): Min distance to Chassis/Suspension mesh = {min_dist*1000:.2f} mm")

# Find the exact X where inner rim touches the suspension hub (min_dist == 0 or ~1-2mm clearance):
print("\n=== FINDING OPTIMAL LATERAL TRACK (X) FOR SUSPENSION MATING ===")
for name, wheel, sign, z in [
    ("FL", m_fl, -1.0, -1.5337),
    ("FR", m_fr,  1.0, -1.5337),
    ("RL", m_rl, -1.0,  1.5193),
    ("RR", m_rr,  1.0,  1.5193),
]:
    best_x = None
    best_dist = 999.0
    for test_x in np.linspace(0.80, 0.95, 151):
        actual_x = sign * test_x
        w_verts = wheel.vertices + np.array([actual_x, 0.0, z])
        tree = cKDTree(m_chassis.vertices)
        dists, _ = tree.query(w_verts)
        min_dist = np.min(dists)
        if min_dist < best_dist:
            best_dist = min_dist
            best_x = test_x
        if min_dist < 0.002: # 2mm clearance
            break
    print(f"Optimal {name}: Half-Track = {best_x:.4f} m (Track = {best_x*2:.4f} m), Min Distance = {best_dist*1000:.2f} mm")
