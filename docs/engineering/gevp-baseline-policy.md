# GEVP baseline policy

`res://scenes/tests/vehicle_track_combinations/gevp_baseline.tscn` is the known-good physics regression scene.

Rules:

- `res://scenes/vehicles/baseline_2026/baseline_2026.tscn` is a frozen copy of the vehicle that was manually validated with correct GEVP behavior.
- Do not tune Formula90s handling in this baseline scene.
- Do not attach DrivingAids, custom aero, telemetry forces, camera-force helpers, or other runtime systems that can mutate vehicle physics.
- Changes to `addons/gevp/` must be validated against this baseline before they are accepted.
- New vehicles must be tested in separate scenes using the same GEVP controller/track structure.
- The legacy internal node/resource names inside the frozen vehicle are intentionally retained to avoid changing the validated baseline.

Jordan development belongs in `res://scenes/tests/vehicle_track_combinations/jordan_handling_test.tscn` and must not modify this reference vehicle.
