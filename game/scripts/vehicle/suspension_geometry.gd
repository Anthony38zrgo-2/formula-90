class_name SuspensionGeometry
extends RefCounted

## Visual kinematic suspension geometry for the Formula-90 open-wheel vehicles.
##
## Authority model: the Rust 1-DOF solver remains the only suspension physics.
## This class solves the *linkage* (UPPER_WISHBONE / LOWER_WISHBONE / PUSHROD /
## TRACKROD / UPRIGHT / ROCKER / DRIVESHAFT) from a per-corner hardpoint table
## declared in the vehicle physics JSON under `suspension.geometry`. The wheel
## centre (telemetry compression) drives the mechanism as a soft target; a small
## position-based solver keeps the arms rigid and the upright triangle closed.
##
## Pure RefCounted: no scene dependency, fully deterministic and unit-testable.

const WHEEL_KEYS := ["FL", "FR", "RL", "RR"]

const PBD_ITERATIONS := 48
const PBD_HUB_WEIGHT := 0.25
const PBD_FINAL_PASSES := 80

var visual_meshes_path: String = ""
var _corners: Array = []
var _axles: Array = []


static func from_json_dict(json: Dictionary) -> SuspensionGeometry:
	var suspension: Variant = json.get("suspension", {})
	if not suspension is Dictionary:
		return null
	var geometry: Variant = suspension.get("geometry", {})
	if not geometry is Dictionary or geometry.is_empty():
		return null
	var axles := {
		"front": _axle_params(suspension.get("front", {})),
		"rear": _axle_params(suspension.get("rear", {})),
	}
	return SuspensionGeometry.new(geometry, axles)


static func from_json_path(path: String) -> SuspensionGeometry:
	if path.is_empty() or not FileAccess.file_exists(path):
		return null
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		return null
	var json_data: Variant = JSON.parse_string(file.get_as_text())
	if not json_data is Dictionary:
		return null
	return from_json_dict(json_data)


static func _axle_params(axle: Variant) -> Dictionary:
	var result := {
		"spring_length": 0.3,
		"resting_ratio": 0.15,
		"camber": 0.0,
		"toe": 0.0,
	}
	if axle is Dictionary:
		result["spring_length"] = float(axle.get("spring_length", result["spring_length"]))
		result["resting_ratio"] = float(axle.get("resting_ratio", result["resting_ratio"]))
		result["camber"] = float(axle.get("camber", 0.0))
		result["toe"] = float(axle.get("toe", 0.0))
	return result


func _init(geometry: Dictionary, axles: Dictionary) -> void:
	visual_meshes_path = String(geometry.get("_visual_meshes", ""))
	_axles = [
		axles.get("front", {}),
		axles.get("front", {}),
		axles.get("rear", {}),
		axles.get("rear", {}),
	]
	for wheel_index in range(4):
		var raw: Variant = geometry.get(WHEEL_KEYS[wheel_index], {})
		if not raw is Dictionary or raw.is_empty():
			_corners.append({})
			continue
		if raw.has("mirror_of"):
			var base: Variant = geometry.get(raw["mirror_of"], {})
			if not base is Dictionary or base.is_empty():
				_corners.append({})
				continue
			raw = _mirror_dict(base)
		_corners.append(_build_corner(raw, _axles[wheel_index]))
	# Determine the connected, reachable travel interval once, never stretch a
	# rigid member to force incompatible telemetry through the authored linkage.
	for i in range(4):
		if is_valid(i):
			_corners[i]["travel_min"] = _travel_limit(i, 0.0)
			_corners[i]["travel_max"] = _travel_limit(i, _corners[i]["spring_len"])



func is_valid(wheel_index: int) -> bool:
	if wheel_index < 0 or wheel_index >= _corners.size():
		return false
	var corner: Dictionary = _corners[wheel_index]
	return corner.get("valid", false)


func get_corner(wheel_index: int) -> Dictionary:
	if wheel_index < 0 or wheel_index >= _corners.size():
		return {}
	return _corners[wheel_index]


## Re-centre a corner's mechanism on the controller's actual rest hub (native
## anchor minus spring travel). All hardpoints translate rigidly, so arm radii,
## triangle edges and the pushrod attachment are preserved.
func align_hub(wheel_index: int, hub_rest: Vector3) -> void:
	var corner: Dictionary = _corners[wheel_index]
	if corner.is_empty() or not hub_rest.is_finite():
		return
	var delta: Vector3 = hub_rest - corner["hub_center"]
	if delta.length() < 1e-6:
		return
	for key in [
		"hub_center", "lw_if", "lw_ir", "lbj_rest", "uw_if", "uw_ir", "ubj_rest",
		"trackrod_inner", "trackrod_outer", "damper_chassis", "ds_inner",
		"ds_outer_rest", "rocker_pivot", "rocker_arm_rest", "damper_arm_rest",
		"pushrod_outer_rest",
	]:
		if corner.has(key):
			corner[key] = corner[key] + delta
	corner["l_center"] = _closest_point_on_axis(corner["lbj_rest"], corner["lw_if"], corner["l_dir"])
	corner["u_center"] = _closest_point_on_axis(corner["ubj_rest"], corner["uw_if"], corner["u_dir"])
	corner["l_radius"] = (corner["lbj_rest"] - corner["l_center"]).length()
	corner["u_radius"] = (corner["ubj_rest"] - corner["u_center"]).length()



## Solve the linkage for one wheel. Returns a Dictionary with element endpoints,
## the solved upright frame, the driveshaft spin and diagnostic residuals.
## Returns an empty Dictionary when the corner has no valid geometry.
func solve(wheel_index: int, compression_m: float, steer_rad: float, _camber_rad: float, spin_rad: float) -> Dictionary:
	if not is_valid(wheel_index):
		return {}
	var c: Dictionary = _corners[wheel_index]
	var requested := compression_m if is_finite(compression_m) else float(c["spring_len"]) * float(c["resting_ratio"])
	var travel := clampf(requested, c["travel_min"], c["travel_max"])
	var pose := _solve_pose(wheel_index, travel, steer_rad, spin_rad)
	pose["travel_limited"] = absf(travel - requested) > 0.000001
	pose["requested_compression"] = requested
	pose["solved_compression"] = travel
	return pose


func _travel_limit(wheel: int, endpoint: float) -> float:
	var c: Dictionary = _corners[wheel]
	var rest: float = c["spring_len"] * c["resting_ratio"]
	var good := rest
	for step in range(1, 33):
		var candidate := lerpf(rest, endpoint, float(step) / 32.0)
		if _reachable(_solve_pose(wheel, candidate, 0.0, 0.0)):
			good = candidate
			continue
		var bad := candidate
		for iteration in range(16):
			var middle := (good + bad) * 0.5
			if _reachable(_solve_pose(wheel, middle, 0.0, 0.0)):
				good = middle
			else:
				bad = middle
		return good
	return good


func _reachable(pose: Dictionary) -> bool:
	if pose.is_empty() or pose["rocker_clamped"] or pose["steering_clamped"]:
		return false
	for key in ["lower_radius", "upper_radius", "tri_lu", "tri_lh", "tri_uh", "pushrod", "trackrod"]:
		if pose["residuals"][key] > 0.0001:
			return false
	return pose["residuals"]["hub"] < 0.001


func _solve_pose(wheel_index: int, compression_m: float, steer_rad: float, spin_rad: float) -> Dictionary:
	var corner: Dictionary = _corners[wheel_index]
	if not corner.get("valid", false):
		return {}

	var spring_len: float = corner["spring_len"]
	var resting_ratio: float = corner["resting_ratio"]
	var rest_compression := spring_len * resting_ratio
	var hub_target: Vector3 = corner["hub_center"] + Vector3.UP * (compression_m - rest_compression)
	if not hub_target.is_finite():
		hub_target = corner["hub_center"]

	var lbj: Vector3 = hub_target + (corner["lbj_rest"] - corner["hub_center"])
	var ubj: Vector3 = hub_target + (corner["ubj_rest"] - corner["hub_center"])
	var hub: Vector3 = hub_target

	for i in range(PBD_ITERATIONS):
		lbj = _project_circle(lbj, corner["l_center"], corner["l_dir"], corner["l_radius"])
		ubj = _project_circle(ubj, corner["u_center"], corner["u_dir"], corner["u_radius"])
		var tri := _constrain_triangle(lbj, ubj, hub, corner)
		lbj = tri[0]
		ubj = tri[1]
		hub = tri[2]

		# Pull only the vertical component: the mechanism owns the lateral track
		# change (real scrub/track variation), so the arms can stay perfectly rigid.
		hub.y += (hub_target.y - hub.y) * PBD_HUB_WEIGHT

	# Constraint-only refinement: removes the
	# residual left by the last hub pull so the linkage closes exactly.
	for i in range(PBD_FINAL_PASSES):
		lbj = _project_circle(lbj, corner["l_center"], corner["l_dir"], corner["l_radius"])
		ubj = _project_circle(ubj, corner["u_center"], corner["u_dir"], corner["u_radius"])
		var tri := _constrain_triangle(lbj, ubj, hub, corner)
		lbj = tri[0]
		ubj = tri[1]
		hub = tri[2]

	if not (lbj.is_finite() and ubj.is_finite() and hub.is_finite()):
		return {}

	# Transport the authored rigid upright using its triangle, then rotate the
	# whole body (including the offset hub) about the ball-joint kingpin.
	var transport := _triangle_frame(lbj, ubj, hub) * _triangle_frame(
		corner["lbj_rest"], corner["ubj_rest"], corner["hub_center"]).transposed()
	var kingpin := (ubj - lbj).normalized()
	var outer_unsteered: Vector3 = lbj + transport * (corner["trackrod_outer"] - corner["lbj_rest"])
	var rack: Vector3 = corner["trackrod_inner"]
	if wheel_index < 2:
		rack.x += _rack_displacement(steer_rad)
	var steering := _circle_link(lbj, kingpin, outer_unsteered, rack, corner["L_trackrod"], steer_rad)
	var steer_rotation := Basis(kingpin, float(steering["angle"]))
	var upright_basis := (steer_rotation * transport).orthonormalized()
	hub = lbj + steer_rotation * (hub - lbj)
	var steer_arm: Vector3 = lbj + upright_basis * (corner["trackrod_outer"] - corner["lbj_rest"])
	# Camber/toe are fixed bearing alignment. Travel camber and bump steer come
	# from the linkage, never a second animated rotation of the wheel bearing.
	var side := 1.0 if wheel_index % 2 == 0 else -1.0
	var wheel_basis := upright_basis * Basis(Vector3.UP, float(corner["toe"]) * side) * Basis(Vector3.BACK, float(corner["camber"]) * side)

	# Preserve the full authored attachment, including offset from the arm plane.
	var mount_upper: bool = corner["pushrod_mount"] == "upper"
	var mount_pose := _arm_pose(corner["uw_if"], corner["u_dir"], corner["ubj_rest"], ubj) if mount_upper else _arm_pose(corner["lw_if"], corner["l_dir"], corner["lbj_rest"], lbj)
	var pushrod_outer: Vector3 = mount_pose * corner["pushrod_outer_rest"]

	var rocker_solved := _rocker_solve(corner, pushrod_outer)
	var rocker_end: Vector3 = rocker_solved["point"]
	var rocker_angle: float = rocker_solved["angle"]
	var damper_end: Vector3 = corner["rocker_pivot"] + (corner["damper_arm_rest"] - corner["rocker_pivot"]).rotated(corner["rocker_axis"], rocker_angle)

	var driveshaft := {}
	var driveshaft_pose := Transform3D.IDENTITY
	if corner.get("has_driveshaft", false):
		var ds_outer: Vector3 = lbj + upright_basis * (corner["ds_outer_rest"] - corner["lbj_rest"])
		driveshaft_pose = _link_pose(corner["ds_inner"], corner["ds_outer_rest"], corner["ds_inner"], ds_outer)
		var shaft_axis: Vector3 = (ds_outer - corner["ds_inner"]).normalized()
		var shaft_spin := -spin_rad * signf(shaft_axis.dot(wheel_basis.x))
		driveshaft = {"inner": corner["ds_inner"], "outer": ds_outer, "spin": shaft_spin}

	var residuals := {
		"hub": absf(hub.y - hub_target.y),
		"hub_lateral": Vector3(hub.x - hub_target.x, 0.0, hub.z - hub_target.z).length(),
		"lower_radius": absf((lbj - corner["l_center"]).length() - corner["l_radius"]),
		"upper_radius": absf((ubj - corner["u_center"]).length() - corner["u_radius"]),
		"tri_lu": absf((lbj - ubj).length() - corner["d_lu"]),
		"tri_lh": absf((lbj - hub).length() - corner["d_lh"]),
		"tri_uh": absf((ubj - hub).length() - corner["d_uh"]),
		"pushrod": absf((rocker_end - pushrod_outer).length() - corner["L_pushrod"]),
		"trackrod": absf((rack - steer_arm).length() - corner["L_trackrod"]),
	}

	return {
		"present": true,
		"lower_pose": _arm_pose(corner["lw_if"], corner["l_dir"], corner["lbj_rest"], lbj),
		"upper_pose": _arm_pose(corner["uw_if"], corner["u_dir"], corner["ubj_rest"], ubj),
		"pushrod_pose": _link_pose(corner["pushrod_outer_rest"], corner["rocker_arm_rest"], pushrod_outer, rocker_end),
		"trackrod_pose": _link_pose(corner["trackrod_inner"], corner["trackrod_outer"], rack, steer_arm),
		"hub": hub,
		"hub_target": hub_target,
		"upright_origin": hub,
		"upright_basis": upright_basis,
		"wheel_basis": wheel_basis,
		"steering_clamped": steering["clamped"],
		"rocker_clamped": rocker_solved["clamped"],
		"lower": [corner["lw_if"], corner["lw_ir"], lbj],
		"upper": [corner["uw_if"], corner["uw_ir"], ubj],
		"trackrod": [rack, steer_arm],
		"pushrod": [pushrod_outer, rocker_end],
		"rocker": {"origin": corner["rocker_pivot"], "basis": Basis(corner["rocker_axis"], rocker_angle), "pushrod": rocker_end, "damper": damper_end},
		"damper": [corner["damper_chassis"], damper_end],
		"driveshaft": driveshaft,
		"driveshaft_pose": driveshaft_pose,
		"pushrod_mount": corner["pushrod_mount"],
		"residuals": residuals,
	}


func _arm_pose(pivot: Vector3, axis: Vector3, rest: Vector3, solved: Vector3) -> Transform3D:
	var center := _closest_point_on_axis(rest, pivot, axis)
	var rotation := Basis(axis, _signed_angle(rest - center, solved - center, axis))
	return Transform3D(rotation, pivot - rotation * pivot)


func _link_pose(rest_a: Vector3, rest_b: Vector3, a: Vector3, b: Vector3) -> Transform3D:
	var rotation := Basis(Quaternion((rest_b - rest_a).normalized(), (b - a).normalized()))
	return Transform3D(rotation, a - rotation * rest_a)


func _build_corner(raw: Dictionary, axle: Dictionary) -> Dictionary:
	var hub_center := _vec(raw.get("hub_center", []))
	var lw: Variant = raw.get("lower_wishbone", {})
	var uw: Variant = raw.get("upper_wishbone", {})
	if not lw is Dictionary or not uw is Dictionary:
		return {}

	var lbj_rest := _vec(lw.get("outer", hub_center))
	var ubj_rest := _vec(uw.get("outer", hub_center))
	var lw_if := _vec(lw.get("inner_front", hub_center))
	var lw_ir := _vec(lw.get("inner_rear", hub_center))
	var uw_if := _vec(uw.get("inner_front", hub_center))
	var uw_ir := _vec(uw.get("inner_rear", hub_center))

	var l_dir := (lw_ir - lw_if).normalized()
	var l_center := _closest_point_on_axis(lbj_rest, lw_if, l_dir)
	var l_radius := (lbj_rest - l_center).length()

	var u_dir := (uw_ir - uw_if).normalized()
	var u_center := _closest_point_on_axis(ubj_rest, uw_if, u_dir)
	var u_radius := (ubj_rest - u_center).length()

	var d_lh := (lbj_rest - hub_center).length()
	var d_uh := (ubj_rest - hub_center).length()
	var d_lu := (lbj_rest - ubj_rest).length()

	var l_mid := (lw_if + lw_ir) * 0.5
	var pushrod_outer_rest := _vec(raw.get("pushrod", {}).get("outer", []))
	if pushrod_outer_rest.is_zero_approx():
		pushrod_outer_rest = l_mid

	var rocker: Variant = raw.get("rocker", {})
	var rocker_pivot := _vec(rocker.get("pivot", []))
	var rocker_axis := _vec(rocker.get("axis", [])).normalized()
	if rocker_axis.is_zero_approx():
		rocker_axis = Vector3.RIGHT
	var rocker_arm_rest := _vec(rocker.get("pushrod_arm", []))
	var damper_arm_rest := _vec(rocker.get("damper_arm", []))
	var L_pushrod := (pushrod_outer_rest - rocker_arm_rest).length()

	var tr: Variant = raw.get("trackrod", {})
	var trackrod_inner := _vec(tr.get("inner", []))
	var trackrod_outer := _vec(tr.get("outer", []))
	var damper_chassis := _vec(raw.get("damper", {}).get("chassis", []))

	var ds: Variant = raw.get("driveshaft", {})
	var has_driveshaft: bool = ds is Dictionary and ds.has("inner") and ds.has("outer")
	var ds_inner := _vec(ds.get("inner", []))
	var ds_outer_rest := _vec(ds.get("outer", hub_center))

	var corner := {
		"valid": false,
		"hub_center": hub_center,
		"spring_len": float(axle.get("spring_length", 0.3)),
		"resting_ratio": float(axle.get("resting_ratio", 0.15)),
		"lw_if": lw_if, "lw_ir": lw_ir, "lbj_rest": lbj_rest,
		"l_dir": l_dir, "l_center": l_center, "l_radius": l_radius,
		"uw_if": uw_if, "uw_ir": uw_ir, "ubj_rest": ubj_rest,
		"u_dir": u_dir, "u_center": u_center, "u_radius": u_radius,
		"d_lh": d_lh, "d_uh": d_uh, "d_lu": d_lu,
		"pushrod_outer_rest": pushrod_outer_rest,
		"pushrod_mount": String(raw.get("pushrod", {}).get("attachment", "lower")),
		"rocker_pivot": rocker_pivot, "rocker_axis": rocker_axis,
		"rocker_arm_rest": rocker_arm_rest, "damper_arm_rest": damper_arm_rest,
		"L_pushrod": L_pushrod,
		"L_trackrod": trackrod_inner.distance_to(trackrod_outer),
		"camber": float(axle.get("camber", 0.0)),
		"toe": float(axle.get("toe", 0.0)),
		"trackrod_inner": trackrod_inner, "trackrod_outer": trackrod_outer,
		"damper_chassis": damper_chassis,
		"has_driveshaft": has_driveshaft,
		"ds_inner": ds_inner, "ds_outer_rest": ds_outer_rest,
	}

	var min_dim := 0.01
	corner["valid"] = l_radius > min_dim and u_radius > min_dim and d_lh > min_dim \
		and d_uh > min_dim and d_lu > min_dim and L_pushrod > min_dim \
		and l_dir.is_finite() and u_dir.is_finite() and hub_center.is_finite()
	return corner


func _mirror_dict(d: Dictionary) -> Dictionary:
	var out := {}
	for key in d:
		out[key] = _mirror_value(d[key])
	return out


func _mirror_value(v: Variant) -> Variant:
	if v is Array:
		if v.size() == 3 and v[0] is float and v[1] is float and v[2] is float:
			return [-float(v[0]), float(v[1]), float(v[2])]
		var out := []
		for item in v:
			out.append(_mirror_value(item))
		return out
	if v is Dictionary:
		return _mirror_dict(v)
	return v


func _constrain_triangle(lbj: Vector3, ubj: Vector3, hub: Vector3, corner: Dictionary) -> Array:
	var d_lu: float = corner["d_lu"]
	var d_lh: float = corner["d_lh"]
	var d_uh: float = corner["d_uh"]

	var dir_lu := lbj - ubj
	if dir_lu.length() > 1e-9:
		var mid := (lbj + ubj) * 0.5
		var d := dir_lu.normalized()
		lbj = mid + d * (d_lu * 0.5)
		ubj = mid - d * (d_lu * 0.5)

	# Hub participates freely in the plane so the linkage can develop its real
	# track change; only the Y component is driven externally.
	var dh := lbj - hub
	if dh.length() > 1e-9:
		var corr := (dh.length() - d_lh) * dh.normalized()
		lbj -= corr * 0.5
		hub += corr * 0.5
	var du := ubj - hub
	if du.length() > 1e-9:
		var corr2 := (du.length() - d_uh) * du.normalized()
		ubj -= corr2 * 0.5
		hub += corr2 * 0.5
	return [lbj, ubj, hub]


func _project_circle(p: Vector3, center: Vector3, axis: Vector3, radius: float) -> Vector3:
	var v := p - center
	var v_perp := v - axis * v.dot(axis)
	if v_perp.length() < 1e-9:
		v_perp = _perp(axis)
	return center + v_perp.normalized() * radius


func _perp(axis: Vector3) -> Vector3:
	var u := axis.cross(Vector3.UP)
	if u.length() < 1e-6:
		u = axis.cross(Vector3.FORWARD)
	if u.length() < 1e-6:
		u = Vector3.RIGHT
	return u.normalized()


func _closest_point_on_axis(p: Vector3, axis_point: Vector3, axis_dir: Vector3) -> Vector3:
	if axis_dir.length() < 1e-9:
		return axis_point
	return axis_point + axis_dir * ((p - axis_point).dot(axis_dir))


func _rocker_solve(corner: Dictionary, pushrod_outer: Vector3) -> Dictionary:
	return _circle_link(corner["rocker_pivot"], corner["rocker_axis"], corner["rocker_arm_rest"], pushrod_outer, corner["L_pushrod"], 0.0)


## Intersection of a rotating attachment circle and a fixed-length link sphere.
## Keep the axial offset. On unreachable input, stay on the circle and report it.
func _circle_link(pivot: Vector3, axis: Vector3, rest: Vector3, other: Vector3, length: float, preferred: float) -> Dictionary:
	var center := pivot + axis * (rest - pivot).dot(axis)
	var radial := rest - center
	var radius := radial.length()
	if radius < 1e-9:
		return {"point": rest, "angle": 0.0, "clamped": true}
	var offset := other - center
	var axial := offset.dot(axis)
	var planar := offset - axis * axial
	var distance := planar.length()
	if distance < 1e-9:
		return {"point": rest, "angle": 0.0, "clamped": true}
	var cosine := (radius * radius + offset.length_squared() - length * length) / (2.0 * radius * distance)
	var clamped := absf(cosine) > 1.0
	var base := _signed_angle(radial, planar, axis)
	var spread := acos(clampf(cosine, -1.0, 1.0))
	var first := wrapf(base + spread, -PI, PI)
	var second := wrapf(base - spread, -PI, PI)
	var angle := first if absf(angle_difference(preferred, first)) < absf(angle_difference(preferred, second)) else second
	return {"point": center + radial.rotated(axis, angle), "angle": angle, "clamped": clamped}


func _triangle_frame(lower: Vector3, upper: Vector3, hub: Vector3) -> Basis:
	var y := (upper - lower).normalized()
	var x := (hub - lower) - y * (hub - lower).dot(y)
	x = x.normalized()
	return Basis(x, y, x.cross(y)).orthonormalized()


## One rack displacement shared by both front corners, calibrated from the
## authored rest geometry rather than an unrelated meters/radian constant.
func _rack_displacement(steer: float) -> float:
	var total := 0.0
	var count := 0
	for i in range(2):
		if not is_valid(i):
			continue
		var c: Dictionary = _corners[i]
		var axis: Vector3 = (c["ubj_rest"] - c["lbj_rest"]).normalized()
		var outer: Vector3 = c["lbj_rest"] + (c["trackrod_outer"] - c["lbj_rest"]).rotated(axis, steer)
		var inner: Vector3 = c["trackrod_inner"]
		var offset := outer - inner
		var length: float = c["L_trackrod"]
		var dx := sqrt(maxf(0.0, length * length - offset.y * offset.y - offset.z * offset.z))
		var side: float = signf(c["trackrod_inner"].x - c["trackrod_outer"].x)
		total += outer.x + side * dx - inner.x
		count += 1
	return total / maxf(float(count), 1.0)


func _signed_angle(a: Vector3, b: Vector3, axis: Vector3) -> float:
	if a.length() < 1e-9 or b.length() < 1e-9:
		return 0.0
	return a.signed_angle_to(b, axis)


func _vec(v: Variant) -> Vector3:
	if v is Array and v.size() >= 3:
		return Vector3(float(v[0]), float(v[1]), float(v[2]))
	return Vector3.ZERO