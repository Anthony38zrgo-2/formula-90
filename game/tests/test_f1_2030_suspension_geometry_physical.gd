extends SceneTree

## Single-source regression: the F1 2030 geometric profile must drive the visual
## linkage from `suspension.geometry_physical.corners` (the mechanism the Rust
## solver actually moves). The legacy profile must keep its authored
## `suspension.geometry` fallback. This guards the SUS-GEO-11 follow-up that
## fixed the visually misaligned rocker/damper.

const GEOMETRIC_JSON := "res://data/vehicles/f1_2030/f1_2030_v10_geometric.json"
const LEGACY_JSON := "res://data/vehicles/f1_2030/f1_2030_v10_physics.json"

var failures: Array[String] = []

func _init() -> void:
	call_deferred("run")

func check(ok: bool, message: String) -> void:
	if not ok:
		failures.append(message)
		printerr("[FAIL] " + message)

func _load_json(path: String) -> Dictionary:
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	return parsed if parsed is Dictionary else {}

func _mirror(v: Variant) -> Variant:
	if v is Array and v.size() == 3 and v[0] is float and v[1] is float and v[2] is float:
		return [-float(v[0]), float(v[1]), float(v[2])]
	if v is Dictionary:
		var out := {}
		for k in v:
			out[k] = _mirror(v[k])
		return out
	if v is Array:
		var items := []
		for item in v:
			items.append(_mirror(item))
		return items
	return v

func _resolve(corners: Dictionary, key: String) -> Dictionary:
	var c: Dictionary = corners.get(key, {})
	if c.has("mirror_of"):
		var base: Dictionary = _resolve(corners, String(c["mirror_of"]))
		var out := {}
		for k in base:
			out[k] = _mirror(base[k])
		return out
	return c

func _vec3(raw: Variant) -> Vector3:
	if raw is Array and raw.size() >= 3:
		return Vector3(float(raw[0]), float(raw[1]), float(raw[2]))
	return Vector3.ZERO

func _same_vec(a: Vector3, b: Vector3) -> bool:
	return a.distance_to(b) < 1e-9

func run() -> void:
	var geo_json := _load_json(GEOMETRIC_JSON)
	if geo_json.is_empty():
		check(false, "Could not parse the geometric profile")
		quit(1)
		return
	var geometry := SuspensionGeometry.from_json_path(GEOMETRIC_JSON)
	check(geometry != null, "Geometric profile failed to build the visual geometry")
	if geometry == null:
		quit(1)
		return
	check(not geometry.visual_meshes_path.is_empty(), "Visual mesh metadata was dropped in geometry_physical mode")

	var suspension: Dictionary = geo_json["suspension"]
	var corners: Dictionary = suspension["geometry_physical"]["corners"]
	var axle_keys := ["front", "front", "rear", "rear"]
	for wheel in range(4):
		var key: String = SuspensionGeometry.WHEEL_KEYS[wheel]
		var c := geometry.get_corner(wheel)
		var phys := _resolve(corners, key)
		var axle: Dictionary = suspension[axle_keys[wheel]]
		var tag := "wheel %s" % key

		check(_same_vec(c["hub_center"], _vec3(phys["hub_center"])), "%s hub_center is not the physical corner" % tag)
		check(_same_vec(c["lbj_rest"], _vec3(phys["lower_wishbone"]["outer"])), "%s lower ball joint differs from physics" % tag)
		check(_same_vec(c["ubj_rest"], _vec3(phys["upper_wishbone"]["outer"])), "%s upper ball joint differs from physics" % tag)
		check(_same_vec(c["lw_if"], _vec3(phys["lower_wishbone"]["inner_front"])), "%s lower front pivot differs from physics" % tag)
		check(_same_vec(c["lw_ir"], _vec3(phys["lower_wishbone"]["inner_rear"])), "%s lower rear pivot differs from physics" % tag)
		check(_same_vec(c["uw_if"], _vec3(phys["upper_wishbone"]["inner_front"])), "%s upper front pivot differs from physics" % tag)
		check(_same_vec(c["uw_ir"], _vec3(phys["upper_wishbone"]["inner_rear"])), "%s upper rear pivot differs from physics" % tag)
		check(_same_vec(c["trackrod_inner"], _vec3(phys["trackrod"]["inner"])), "%s trackrod inner differs from physics" % tag)
		check(_same_vec(c["trackrod_outer"], _vec3(phys["trackrod"]["outer"])), "%s trackrod outer differs from physics" % tag)
		check(_same_vec(c["rocker_pivot"], _vec3(phys["rocker"]["pivot"])), "%s rocker pivot differs from physics" % tag)
		check(_same_vec(c["rocker_arm_rest"], _vec3(phys["rocker"]["pushrod_arm"])), "%s rocker pushrod arm differs from physics" % tag)
		check(_same_vec(c["damper_arm_rest"], _vec3(phys["rocker"]["damper_arm"])), "%s rocker damper arm differs from physics" % tag)
		check(_same_vec(c["damper_chassis"], _vec3(phys["damper"]["chassis"])), "%s damper chassis differs from physics" % tag)
		var phys_rod: Dictionary = phys.get("rod", {})
		check(_same_vec(c["pushrod_outer_rest"], _vec3(phys_rod.get("outer", []))), "%s rod outer differs from physics" % tag)
		check(String(c["pushrod_mount"]) == String(phys_rod.get("attachment", "lower")), "%s rod attachment differs from physics" % tag)

		# The old bug: the visual corner rotated the rocker about the vertical
		# axis while the physical rocker is transverse.
		var axis: Vector3 = c["rocker_axis"]
		check(absf(axis.y) < 1e-6 and absf(axis.x) > 0.99, "%s rocker axis is not the physical transverse axis (%s)" % [tag, axis])

		# Rest pose must close on the physical rest attachments.
		var rest: float = float(c["spring_len"]) * float(c["resting_ratio"])
		var rest_pose := geometry.solve(wheel, rest, 0.0, 0.0, 0.0)
		check(not rest_pose.is_empty(), "%s rest pose is empty" % tag)
		if not rest_pose.is_empty():
			check(rest_pose["damper"][1].distance_to(c["damper_arm_rest"]) < 1e-6, "%s rest damper arm is detached" % tag)
			check(rest_pose["pushrod"][1].distance_to(c["rocker_arm_rest"]) < 1e-6, "%s rest rocker arm is detached" % tag)

		# The visual window must contain the physical travel limits; otherwise the
		# rendered wheel clamps before the simulated one reaches its stops.
		var droop := float(axle.get("wheel_droop_m", 0.0))
		var bump := float(axle.get("wheel_bump_m", 0.0))
		check(float(c["travel_min"]) <= rest - droop + 1e-4, "%s visual droop window is tighter than physics" % tag)
		check(float(c["travel_max"]) >= rest + bump - 1e-4, "%s visual bump window is tighter than physics" % tag)
		for limit in [rest - droop, rest + bump]:
			var pose := geometry.solve(wheel, limit, 0.0, 0.0, 0.0)
			check(not pose.is_empty(), "%s physical limit pose is empty" % tag)
			if not pose.is_empty():
				check(not pose["rocker_clamped"] and not pose["steering_clamped"], "%s physical limit pose is clamped" % tag)
				check(float(pose["residuals"]["hub"]) < 1e-3, "%s physical limit hub residual is high" % tag)

	# Fallback guard: profiles without a physical block keep the authored linkage.
	var legacy_json := _load_json(LEGACY_JSON)
	check(not legacy_json.is_empty(), "Could not parse the legacy profile")
	check(not legacy_json["suspension"].has("geometry_physical"), "Legacy profile unexpectedly carries geometry_physical")
	var legacy_geometry := SuspensionGeometry.from_json_path(LEGACY_JSON)
	check(legacy_geometry != null, "Legacy profile failed to build the visual geometry")
	if legacy_geometry != null:
		var legacy_axis: Vector3 = legacy_geometry.get_corner(0)["rocker_axis"]
		check(absf(legacy_axis.y) > 0.99, "Legacy fallback did not keep its authored rocker axis (%s)" % legacy_axis)

	print("[RESULT] Suspension geometry single-source: %d failure(s)" % failures.size())
	quit(0 if failures.is_empty() else 1)
