extends SceneTree

## Deterministic invariant tests for the F1 2030 visual suspension geometry
## (UPPER_WISHBONE / LOWER_WISHBONE / PUSHROD / TRACKROD / UPRIGHT / ROCKER /
## DRIVESHAFT). Runs without the native DLLs: only the physics JSON + the pure
## RefCounted solver are exercised.

const PHYSICS_JSON := "res://data/vehicles/f1_2030/f1_2030_v10_physics.json"
const GEOMETRY_SCRIPT := "res://scripts/vehicle/suspension_geometry.gd"

const TOL_RADIUS := 0.01
const TOL_TRIANGLE := 0.01
const TOL_HUB := 0.025
const TOL_PUSHROD := 0.03

var _failures: Array[String] = []

func _init() -> void:
	call_deferred("_run")


func _fail(msg: String) -> void:
	_failures.append(msg)
	printerr("[FAIL] " + msg)


func _check(condition: bool, msg: String) -> void:
	if not condition:
		_fail(msg)


func _run() -> void:
	print("=== F1 2030 Suspension Geometry Invariants ===")
	var file := FileAccess.open(PHYSICS_JSON, FileAccess.READ)
	if file == null:
		_fail("Cannot open physics JSON: " + PHYSICS_JSON)
		quit(1)
		return
	var json_data: Variant = JSON.parse_string(file.get_as_text())
	if not json_data is Dictionary:
		_fail("Physics JSON is not a Dictionary")
		quit(1)
		return

	var geometry_script := load(GEOMETRY_SCRIPT) as Script
	if geometry_script == null or not geometry_script.can_instantiate():
		_fail("Cannot load " + GEOMETRY_SCRIPT)
		quit(1)
		return

	var geometry = geometry_script.from_json_dict(json_data)
	if geometry == null:
		_fail("SuspensionGeometry failed to build from the physics JSON")
		quit(1)
		return

	var suspension: Dictionary = json_data["suspension"]
	var front: Dictionary = suspension["front"]
	var rear: Dictionary = suspension["rear"]
	var spring: PackedFloat64Array = [float(front["spring_length"]), float(front["spring_length"]), float(rear["spring_length"]), float(rear["spring_length"])]
	var rest_ratio: PackedFloat64Array = [float(front["resting_ratio"]), float(front["resting_ratio"]), float(rear["resting_ratio"]), float(rear["resting_ratio"])]

	# Corner validity + element presence.
	for wheel_index in range(4):
		_check(geometry.is_valid(wheel_index), "wheel %d geometry must be valid" % wheel_index)
		var corner: Dictionary = geometry.get_corner(wheel_index)
		_check(corner.get("has_driveshaft", false) == (wheel_index >= 2),
			"wheel %d driveshaft presence must match rear-only" % wheel_index)
		_check(corner.get("l_radius", 0.0) > 0.05, "wheel %d lower arm radius plausible" % wheel_index)
		_check(corner.get("u_radius", 0.0) > 0.05, "wheel %d upper arm radius plausible" % wheel_index)
		_check(corner.get("L_pushrod", 0.0) > 0.10, "wheel %d pushrod length plausible" % wheel_index)

	# Rest state: solve at rest compression must land on the rest hub.
	for wheel_index in range(4):
		var rest_c: float = spring[wheel_index] * rest_ratio[wheel_index]
		var data: Dictionary = geometry.solve(wheel_index, rest_c, 0.0, 0.0, 0.0)
		_check(not data.is_empty(), "wheel %d rest solve must produce data" % wheel_index)
		if data.is_empty():
			continue
		var corner: Dictionary = geometry.get_corner(wheel_index)
		var hub: Vector3 = data["hub"]
		_check(hub.distance_to(corner["hub_center"]) < 0.02,
			"wheel %d rest hub must sit on hub_center, got %s" % [wheel_index, hub])

	# Travel sweep: keep the mechanism rigid inside the operative range.
	for wheel_index in range(4):
		var rest_c: float = spring[wheel_index] * rest_ratio[wheel_index]
		var lo := maxf(0.0, rest_c - 0.020)
		var hi := minf(spring[wheel_index], rest_c + 0.100)
		var steps := 24
		var prev_hub_y := INF
		for step in range(steps + 1):
			var c := lo + (hi - lo) * (float(step) / float(steps))
			var data: Dictionary = geometry.solve(wheel_index, c, 0.0, 0.0, 0.0)
			if data.is_empty():
				_fail("wheel %d solve empty at compression %.3f" % [wheel_index, c])
				continue
			var residuals: Dictionary = data["residuals"]
			_check(float(residuals["lower_radius"]) < TOL_RADIUS,
				"wheel %d lower radius err %.4f > tol at c=%.3f" % [wheel_index, residuals["lower_radius"], c])
			_check(float(residuals["upper_radius"]) < TOL_RADIUS,
				"wheel %d upper radius err %.4f > tol at c=%.3f" % [wheel_index, residuals["upper_radius"], c])
			_check(float(residuals["tri_lu"]) < TOL_TRIANGLE and float(residuals["tri_lh"]) < TOL_TRIANGLE and float(residuals["tri_uh"]) < TOL_TRIANGLE,
				"wheel %d triangle errs at c=%.3f" % [wheel_index, c])
			_check(float(residuals["pushrod"]) < TOL_PUSHROD,
				"wheel %d pushrod err %.4f > tol at c=%.3f" % [wheel_index, residuals["pushrod"], c])
			_check(float(residuals["hub"]) < TOL_HUB,
				"wheel %d hub residual %.4f > tol at c=%.3f" % [wheel_index, residuals["hub"], c])
			var hub_y: float = data["hub"].y
			if prev_hub_y != INF:
				_check(absf(hub_y - prev_hub_y) < 0.03, "wheel %d hub must move smoothly, dY=%.4f" % [wheel_index, hub_y - prev_hub_y])
			prev_hub_y = hub_y
			_check(data["lower"][2].is_finite() and data["upper"][2].is_finite(), "wheel %d ball joints finite" % wheel_index)
			_check(data["pushrod"][1].is_finite() and data["damper"][1].is_finite(), "wheel %d rocker chain finite" % wheel_index)

	# Steering sweep (front wheels) must keep the linkage connected.
	for wheel_index in range(2):
		var rest_c: float = spring[wheel_index] * rest_ratio[wheel_index]
		for steer_deg in [-20.0, -10.0, 0.0, 10.0, 20.0]:
			var data: Dictionary = geometry.solve(wheel_index, rest_c, deg_to_rad(steer_deg), -0.048, 0.0)
			_check(not data.is_empty(), "wheel %d steer solve must produce data at %d deg" % [wheel_index, int(steer_deg)])
			if data.is_empty():
				continue
			var trackrod: Array = data["trackrod"]
			_check(trackrod[0].is_finite() and trackrod[1].is_finite(), "wheel %d trackrod finite at %d deg" % [wheel_index, int(steer_deg)])
			_check(float(data["residuals"]["hub"]) < TOL_HUB, "wheel %d steer keeps hub bound at %d deg" % [wheel_index, int(steer_deg)])

	print("[RESULT] Suspension geometry invariants: %d failure(s)" % _failures.size())
	quit(0 if _failures.is_empty() else 1)
