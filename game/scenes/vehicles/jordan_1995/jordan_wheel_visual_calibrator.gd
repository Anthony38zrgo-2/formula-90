extends Node

## Jordan 1995 visual-wheel calibration.
##
## The source GLBs do not all have perfectly centered/symmetric bounds. Because
## GEVP rotates the visual wheel around the local X axis, an off-center mesh can
## look like a bent axle or unstable suspension even when the RayCast3D physics
## are behaving correctly. This node normalizes each imported wheel visual to
## the physical tire width/radius configured on the Vehicle and recenters its
## geometry around the rotation axis. GEVP physics are not modified.

@export var front_left_visual_path := NodePath("../WheelFrontLeft/Pivot/Visual")
@export var front_right_visual_path := NodePath("../WheelFrontRight/Pivot/Visual")
@export var rear_left_visual_path := NodePath("../WheelRearLeft/Pivot/Visual")
@export var rear_right_visual_path := NodePath("../WheelRearRight/Pivot/Visual")

func _ready() -> void:
	# Imported PackedScenes are present by _ready(), but defer once so all mesh
	# global transforms are settled before calculating combined local bounds.
	call_deferred("_apply_calibration")

func _apply_calibration() -> void:
	var vehicle := get_parent()
	if not vehicle:
		push_warning("Jordan wheel calibrator has no vehicle parent")
		return

	var front_width_m := float(vehicle.get("front_tire_width")) * 0.001
	var rear_width_m := float(vehicle.get("rear_tire_width")) * 0.001
	var front_diameter_m := float(vehicle.get("front_tire_radius")) * 2.0
	var rear_diameter_m := float(vehicle.get("rear_tire_radius")) * 2.0

	_normalize_visual(front_left_visual_path, front_width_m, front_diameter_m)
	_normalize_visual(front_right_visual_path, front_width_m, front_diameter_m)
	_normalize_visual(rear_left_visual_path, rear_width_m, rear_diameter_m)
	_normalize_visual(rear_right_visual_path, rear_width_m, rear_diameter_m)

func _normalize_visual(path: NodePath, target_width: float, target_diameter: float) -> void:
	var visual := get_node_or_null(path) as Node3D
	if not visual:
		push_warning("Jordan wheel visual missing: %s" % path)
		return

	# Always calculate from the unmodified imported visual transform.
	visual.position = Vector3.ZERO
	visual.scale = Vector3.ONE

	var bounds := _combined_mesh_bounds(visual)
	if bounds.size.x <= 0.0001 or bounds.size.y <= 0.0001 or bounds.size.z <= 0.0001:
		push_warning("Jordan wheel visual has invalid bounds: %s" % path)
		return

	# X is the axle/width axis. Y/Z form the radial plane because GEVP spins
	# wheel_node around local X.
	var correction_scale := Vector3(
		target_width / bounds.size.x,
		target_diameter / bounds.size.y,
		target_diameter / bounds.size.z
	)
	var center := bounds.position + bounds.size * 0.5

	visual.scale = correction_scale
	# Scale is around the imported origin; compensate the scaled geometric center
	# so that it lands exactly on the GEVP rotation axis.
	visual.position = Vector3(
		-center.x * correction_scale.x,
		-center.y * correction_scale.y,
		-center.z * correction_scale.z
	)

func _combined_mesh_bounds(root: Node3D) -> AABB:
	var min_point := Vector3(INF, INF, INF)
	var max_point := Vector3(-INF, -INF, -INF)
	var found_mesh := false
	var root_inverse := root.global_transform.affine_inverse()

	for node in root.find_children("*", "MeshInstance3D", true, false):
		var mesh_instance := node as MeshInstance3D
		if not mesh_instance or not mesh_instance.mesh:
			continue

		var local_aabb := mesh_instance.mesh.get_aabb()
		var to_root := root_inverse * mesh_instance.global_transform
		for xi in range(2):
			for yi in range(2):
				for zi in range(2):
					var corner := local_aabb.position + Vector3(
						local_aabb.size.x * xi,
						local_aabb.size.y * yi,
						local_aabb.size.z * zi
					)
					var point := to_root * corner
					min_point = Vector3(
						minf(min_point.x, point.x),
						minf(min_point.y, point.y),
						minf(min_point.z, point.z)
					)
					max_point = Vector3(
						maxf(max_point.x, point.x),
						maxf(max_point.y, point.y),
						maxf(max_point.z, point.z)
					)
					found_mesh = true

	if not found_mesh:
		return AABB()
	return AABB(min_point, max_point - min_point)
