extends SceneTree

## Single-source regression: the F1 2030 geometric profile must drive the visual
## linkage from `suspension.geometry_physical.corners` (the mechanism the Rust
## solver actually moves). The legacy profile must keep its authored
## `suspension.geometry` fallback. This guards the SUS-GEO-11 follow-up that
## fixed the visually misaligned rocker/damper.

const GEOMETRIC_JSON := "res://data/vehicles/f1_2030/f1_2030_v10_geometric.json"
const LEGACY_JSON := "res://data/vehicles/f1_2030/f1_2030_v10_physics.json"
const MESHES_JSON := "res://data/vehicles/f1_2030/f1_2030_suspension_meshes.json"

## Measured f1_2030.blend monocoque/nose top envelope in chassis-local metres
## (5 cm z bins, GEO_CHASSIS_BODY/INTERIOR/FLOOR/STEERCOLUM vertices, read-only
## Blender dump). Piecewise-linear top height and half width per z anchor.
const NOSE_ENVELOPE_Z := [-1.70, -1.50, -1.30, -1.15, -1.00, -0.85]
const NOSE_ENVELOPE_TOP := [0.218, 0.251, 0.277, 0.296, 0.308, 0.312]
const NOSE_ENVELOPE_HALF_WIDTH := [0.176, 0.188, 0.202, 0.216, 0.228, 0.238]

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

func _nose_top(z: float) -> float:
	var zz := clampf(z, NOSE_ENVELOPE_Z[0], NOSE_ENVELOPE_Z[NOSE_ENVELOPE_Z.size() - 1])
	for i in range(NOSE_ENVELOPE_Z.size() - 1):
		if zz <= NOSE_ENVELOPE_Z[i + 1]:
			var t: float = (zz - NOSE_ENVELOPE_Z[i]) / (NOSE_ENVELOPE_Z[i + 1] - NOSE_ENVELOPE_Z[i])
			return lerpf(NOSE_ENVELOPE_TOP[i], NOSE_ENVELOPE_TOP[i + 1], t)
	return NOSE_ENVELOPE_TOP[NOSE_ENVELOPE_TOP.size() - 1]

func _nose_half_width(z: float) -> float:
	var zz := clampf(z, NOSE_ENVELOPE_Z[0], NOSE_ENVELOPE_Z[NOSE_ENVELOPE_Z.size() - 1])
	for i in range(NOSE_ENVELOPE_Z.size() - 1):
		if zz <= NOSE_ENVELOPE_Z[i + 1]:
			var t: float = (zz - NOSE_ENVELOPE_Z[i]) / (NOSE_ENVELOPE_Z[i + 1] - NOSE_ENVELOPE_Z[i])
			return lerpf(NOSE_ENVELOPE_HALF_WIDTH[i], NOSE_ENVELOPE_HALF_WIDTH[i + 1], t)
	return NOSE_ENVELOPE_HALF_WIDTH[NOSE_ENVELOPE_HALF_WIDTH.size() - 1]

## Chassis-side suspension hardpoints must stay inside the measured nose
## envelope (packaging regression for the SUS-GEO-12 protrusion fix).
func check_packaging(corners: Dictionary, tag_prefix: String) -> void:
	for wheel in ["FL", "FR"]:
		var phys: Dictionary = corners.get(wheel, {})
		if phys.is_empty() or phys.has("mirror_of"):
			continue
		var rocker: Dictionary = phys.get("rocker", {})
		var damper: Dictionary = phys.get("damper", {})
		for point_name in ["rocker.pivot", "rocker.pushrod_arm", "rocker.damper_arm", "damper.chassis"]:
			var parts: PackedStringArray = point_name.split(".")
			var point: Variant = phys.get(parts[0], {}).get(parts[1], null)
			if point is Array and point.size() >= 3:
				var v := _vec3(point)
				var top := _nose_top(v.z)
				var width := _nose_half_width(v.z)
				check(v.y <= top - 0.01, "%s%s lies above the measured nose top (%.3f > %.3f at z=%.3f)" % [tag_prefix, point_name, v.y, top, v.z])
				check(absf(v.x) <= width - 0.01, "%s%s lies outside the measured nose width (|%.3f| > %.3f at z=%.3f)" % [tag_prefix, point_name, v.x, width, v.z])

## The authored meshes must land on the physical hardpoints: for the FL/FR
## pushrod, the blade's end-ring centroids must coincide with the rod outer
## hardpoint and the rocker arm rest (rendered-attachment regression).
func check_rendered_pushrod_endpoints(meshes_root: Dictionary, geometry: SuspensionGeometry) -> void:
	var meshes: Dictionary = meshes_root.get("corners", {})
	for wheel in range(2):
		var key: String = SuspensionGeometry.WHEEL_KEYS[wheel]
		var role: Dictionary = meshes.get(key, {}).get("pushrod", {})
		if role.is_empty():
			check(false, "%s pushrod authored mesh missing" % key)
			continue
		var c := geometry.get_corner(wheel)
		var axis: Vector3 = (c["rocker_arm_rest"] - c["pushrod_outer_rest"]).normalized()
		var length: float = c["pushrod_outer_rest"].distance_to(c["rocker_arm_rest"])
		var tip: Array = []
		var base: Array = []
		for vertex in role["vertices"]:
			var v := _vec3(vertex)
			var axial: float = (v - c["pushrod_outer_rest"]).dot(axis)
			if axial > length - 0.02:
				tip.append(v)
			elif axial < 0.02:
				base.append(v)
		check(tip.size() >= 4, "%s pushrod tip ring not found" % key)
		check(base.size() >= 4, "%s pushrod base ring not found" % key)
		if tip.is_empty() or base.is_empty():
			continue
		var tip_centroid := Vector3.ZERO
		for v in tip:
			tip_centroid += v
		tip_centroid /= float(tip.size())
		var base_centroid := Vector3.ZERO
		for v in base:
			base_centroid += v
		base_centroid /= float(base.size())
		check(tip_centroid.distance_to(c["rocker_arm_rest"]) < 0.002, "%s pushrod blade tip is %.1f mm from the physical rocker arm" % [key, tip_centroid.distance_to(c["rocker_arm_rest"]) * 1000.0])
		check(base_centroid.distance_to(c["pushrod_outer_rest"]) < 0.002, "%s pushrod blade base is %.1f mm from the rod outer hardpoint" % [key, base_centroid.distance_to(c["pushrod_outer_rest"]) * 1000.0])
		# Across the whole validated travel the posed blade tip must ride on
		# the solved rocker end (the rest-to-current mapping is rigid; a mesh
		# built for different endpoints would drift).
		var rest: float = float(c["spring_len"]) * float(c["resting_ratio"])
		for travel in [c["travel_min"], rest, c["travel_max"]]:
			var pose := geometry.solve(wheel, travel, 0.0, 0.0, 0.0)
			if pose.is_empty():
				check(false, "%s travel pose empty at %.4f" % [key, travel])
				continue
			var posed_tip: Vector3 = pose["pushrod_pose"] * tip_centroid
			check(posed_tip.distance_to(pose["pushrod"][1]) < 0.004, "%s posed pushrod tip is %.1f mm from the solved rocker end at travel %.4f" % [key, posed_tip.distance_to(pose["pushrod"][1]) * 1000.0, travel])

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
	var meshes_parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(MESHES_JSON))
	check(meshes_parsed is Dictionary, "Could not parse the authored suspension meshes")
	check_packaging(corners, "physical ")
	if geometry != null and meshes_parsed is Dictionary:
		check_rendered_pushrod_endpoints(meshes_parsed, geometry)

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
