extends SceneTree

const SCENE_PATH := "res://scenes/tests/vehicle_track_combinations/jordan_197_handling_test.tscn"
const VEHICLE_PATH := NodePath("Jordan197/VehicleRigidBody")
const WHEEL_NAMES := ["WheelFrontLeft", "WheelFrontRight", "WheelRearLeft", "WheelRearRight"]


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var packed_scene := load(SCENE_PATH) as PackedScene
	if packed_scene == null:
		printerr("GEOMETRY_PROBE_ERROR unable to load ", SCENE_PATH)
		quit(2)
		return

	var scene_root := packed_scene.instantiate()
	root.add_child(scene_root)
	await process_frame
	_print_snapshot(scene_root, "after_process_frame")

	for _frame in range(60):
		await physics_frame
	_print_snapshot(scene_root, "after_60_physics_frames")

	scene_root.queue_free()
	await process_frame
	quit(0)


func _print_snapshot(scene_root: Node, phase: String) -> void:
	var vehicle := scene_root.get_node_or_null(VEHICLE_PATH) as Node3D
	if vehicle == null:
		printerr("GEOMETRY_PROBE_ERROR vehicle missing")
		return

	var chassis := vehicle.get_node_or_null("ChassisVisual") as Node3D
	var snapshot: Dictionary = {
		"phase": phase,
		"vehicle_origin": _vec(vehicle.position),
		"chassis_transform_origin": _vec(chassis.position) if chassis else null,
		"chassis_forward": _vec((chassis.basis * Vector3.FORWARD).normalized()) if chassis else null,
		"wheels": {},
	}

	for wheel_name in WHEEL_NAMES:
		var wheel := vehicle.get_node_or_null(wheel_name) as RayCast3D
		if wheel == null:
			continue
		var wheel_node: Node3D = wheel.get("wheel_node") as Node3D
		var visual: Node = wheel_node.get_node_or_null("Visual") if wheel_node else null
		var visual_bounds: Variant = _world_bounds(visual) if visual else null
		var visual_center: Vector3 = Vector3.ZERO
		var has_visual_center: bool = false
		if visual_bounds != null:
			visual_center = visual_bounds.position + visual_bounds.size * 0.5
			has_visual_center = true
		var wheel_data := {
			"raycast_local": _vec(wheel.position),
			"raycast_global": _vec(wheel.global_position),
			"wheel_node_path": String(wheel_node.get_path()) if wheel_node else "",
			"wheel_node_local": _vec(wheel_node.position) if wheel_node else null,
			"wheel_node_global": _vec(wheel_node.global_position) if wheel_node else null,
			"visual_scene": String(visual.scene_file_path) if visual else "",
			"visual_center_global": _vec(visual_center) if has_visual_center else null,
			"visual_minus_raycast": _vec(visual_center - wheel.global_position) if has_visual_center else null,
			"visual_bounds": {
				"min": _vec(visual_bounds.position),
				"max": _vec(visual_bounds.position + visual_bounds.size),
			} if visual_bounds != null else null,
		}
		snapshot["wheels"][wheel_name] = wheel_data

	print("GEOMETRY_PROBE ", JSON.stringify(snapshot))


func _world_bounds(node: Node) -> AABB:
	var bounds := AABB()
	var has_point := false
	var pending: Array[Node] = [node]
	while not pending.is_empty():
		var current: Node = pending.pop_back()
		var mesh_instance: MeshInstance3D = current as MeshInstance3D
		if mesh_instance != null and mesh_instance.mesh != null:
			var local_bounds: AABB = mesh_instance.get_aabb()
			for x in [local_bounds.position.x, local_bounds.end.x]:
				for y in [local_bounds.position.y, local_bounds.end.y]:
					for z in [local_bounds.position.z, local_bounds.end.z]:
						var point: Vector3 = mesh_instance.global_transform * Vector3(x, y, z)
						if not has_point:
							bounds = AABB(point, Vector3.ZERO)
							has_point = true
						else:
							bounds = bounds.expand(point)
		for child in current.get_children():
			pending.append(child)
	return bounds if has_point else AABB()


func _vec(value: Vector3) -> Array[float]:
	return [value.x, value.y, value.z]
