extends SceneTree

const SCENE := preload("res://scenes/tests/vehicle_track_combinations/jordan_197_handling_test.tscn")
const MANIFEST_PATH := "res://assets/models/vehicles/jordan_197/vehicle_runtime_manifest.json"
const SOURCE_ASSEMBLY_PATH := "res://assets/models/vehicles/jordan_197/source_assembly.json"
const WHEEL_SCENES := {
	"WheelFrontLeft/Pivot/Visual": "res://assets/models/vehicles/jordan_197/jordan_197_wheel_fl.glb",
	"WheelFrontRight/Pivot/Visual": "res://assets/models/vehicles/jordan_197/jordan_197_wheel_fr.glb",
	"WheelRearLeft/Pivot/Visual": "res://assets/models/vehicles/jordan_197/jordan_197_wheel_rl.glb",
	"WheelRearRight/Pivot/Visual": "res://assets/models/vehicles/jordan_197/jordan_197_wheel_rr.glb",
}
const AXLE_TOLERANCE_M := 0.01
const FORWARD_DOT_TOLERANCE := 0.99
const WHEEL_CENTER_TOLERANCE_M := 0.01


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var scene_root := SCENE.instantiate()
	var failures: Array[String] = []
	var authored_vehicle := scene_root.get_node_or_null("Jordan197/VehicleRigidBody")
	_check_authored_alignment(authored_vehicle, failures)
	root.add_child(scene_root)
	await process_frame

	var track := scene_root.get_node_or_null("Track") as Node3D
	var controller := scene_root.get_node_or_null("Jordan197") as Node3D
	var vehicle := scene_root.get_node_or_null("Jordan197/VehicleRigidBody")
	var camera := scene_root.get_node_or_null("CameraRig") as Node
	var aids := scene_root.get_node_or_null("DrivingAids") as Node
	var tuner := scene_root.get_node_or_null("DebugHud/HandlingTuningPanel") as Node
	if track == null:
		failures.append("La Chutana Track instance is missing")
	if controller == null or controller.get_script() == null or controller.get_script().resource_path != "res://addons/gevp/scripts/vehicle_controllergd.gd":
		failures.append("Jordan197 root is not using the canonical vehicle controller")
	if vehicle == null:
		failures.append("canonical Jordan197/VehicleRigidBody is missing")
	elif not vehicle is RigidBody3D:
		failures.append("Jordan197 chassis owner is not a RigidBody3D")
	elif vehicle.get_script() == null or vehicle.get_script().resource_path != "res://addons/gevp/scripts/vehicle.gd":
		failures.append("Jordan197 RigidBody3D is not using the canonical vehicle script")
	if camera == null:
		failures.append("CameraRig is missing")
	if aids == null:
		failures.append("DrivingAids is missing")
	if tuner == null:
		failures.append("HandlingTuningPanel is missing")
	else:
		var vp := str(tuner.get("vehicle_path"))
		if vp != "../Jordan197/VehicleRigidBody" and vp != "../Jordan191/VehicleRigidBody" and vp != "":
			# Allow resolver fallback: check if tuner discovered vehicle via _find_vehicle_in_scene
			if tuner.get("_vehicle") == null:
				failures.append("HandlingTuningPanel vehicle_path is invalid: %s" % vp)
		var expected_tunables := [
			"front_brake_bias",
			"max_steering_angle",
			"max_torque",
			"motor_drag",
			"coefficient_of_drag",
			"frontal_area",
			"air_density",
		]
		if tuner.call("get_tunable_ids") != expected_tunables:
			failures.append("HandlingTuningPanel allowlist changed unexpectedly")
		var panel := tuner.get_node_or_null("LiveTuningPanel") as Control
		if panel == null:
			failures.append("HandlingTuningPanel UI was not created")
		else:
			var key_event := InputEventKey.new()
			key_event.keycode = KEY_F10
			key_event.pressed = true
			Input.parse_input_event(key_event)
			await process_frame
			if not panel.visible:
				failures.append("F10 did not open HandlingTuningPanel")
	if vehicle != null:
		for wheel_name in ["WheelFrontLeft", "WheelFrontRight", "WheelRearLeft", "WheelRearRight"]:
			var wheel := vehicle.get_node_or_null(wheel_name)
			if wheel == null:
				failures.append("missing wheel: %s" % wheel_name)
			elif not wheel is RayCast3D or wheel.get_script() == null or wheel.get_script().resource_path != "res://addons/gevp/scripts/wheel.gd":
				failures.append("wheel does not use the canonical RayCast controller: %s" % wheel_name)

		var chassis_stats := _mesh_stats(vehicle.get_node_or_null("ChassisVisual"))
		var front_stats := _mesh_stats(vehicle.get_node_or_null("WheelFrontLeft/Pivot/Visual"))
		var rear_stats := _mesh_stats(vehicle.get_node_or_null("WheelRearLeft/Pivot/Visual"))
		if chassis_stats.surfaces < 18 or chassis_stats.materials < 18:
			failures.append("Jordan197 chassis import collapsed its materials: %s" % [chassis_stats])
		if front_stats.surfaces < 4 or front_stats.materials < 4:
			failures.append("Jordan197 front wheel import collapsed its materials: %s" % [front_stats])
		if rear_stats.surfaces < 4 or rear_stats.materials < 4:
			failures.append("Jordan197 rear wheel import collapsed its materials: %s" % [rear_stats])
		_check_visual_alignment(vehicle, failures)
	scene_root.queue_free()
	if failures.is_empty():
		print("PASS: canonical Jordan 197 handling scene")
		quit(0)
	else:
		push_error("; ".join(failures))
		quit(1)


func _check_visual_alignment(vehicle: Node, failures: Array[String]) -> void:
	var chassis := vehicle.get_node_or_null("ChassisVisual") as Node3D
	var front_left := vehicle.get_node_or_null("WheelFrontLeft") as Node3D
	var front_right := vehicle.get_node_or_null("WheelFrontRight") as Node3D
	var rear_left := vehicle.get_node_or_null("WheelRearLeft") as Node3D
	var rear_right := vehicle.get_node_or_null("WheelRearRight") as Node3D
	if chassis == null or front_left == null or front_right == null or rear_left == null or rear_right == null:
		failures.append("Jordan197 visual alignment cannot be checked because chassis or axle nodes are missing")
		return

	for wheel_path in WHEEL_SCENES:
		_check_wheel_scene(vehicle, str(wheel_path), str(WHEEL_SCENES[wheel_path]), failures)
	_check_wheel_centers(vehicle, failures)

	var manifest := _load_manifest(failures)
	if manifest.is_empty():
		return
	var source_assembly := _load_json_file(SOURCE_ASSEMBLY_PATH, "source assembly", failures)
	if source_assembly.is_empty():
		return
	var expected_front_axle := _source_anchor_runtime(
		source_assembly, manifest, "DATUM_FRONT_AXLE_CENTER", failures
	)
	var expected_rear_axle := _source_anchor_runtime(
		source_assembly, manifest, "DATUM_REAR_AXLE_CENTER", failures
	)
	if not failures.is_empty():
		return

	var runtime_forward := Vector3(0.0, 0.0, -1.0)
	var front_static_drop := float(vehicle.get("front_spring_length")) * (1.0 - float(vehicle.get("front_resting_ratio")))
	var rear_static_drop := float(vehicle.get("rear_spring_length")) * (1.0 - float(vehicle.get("rear_resting_ratio")))
	var physical_front_axle := (front_left.position + front_right.position) * 0.5 + Vector3.DOWN * front_static_drop
	var physical_rear_axle := (rear_left.position + rear_right.position) * 0.5 + Vector3.DOWN * rear_static_drop
	var physical_forward := (physical_front_axle - physical_rear_axle).normalized()
	var visual_forward := (chassis.basis * runtime_forward).normalized()
	var forward_dot := visual_forward.dot(physical_forward)
	if forward_dot < FORWARD_DOT_TOLERANCE:
		failures.append("Jordan197 visual forward disagrees with physical forward: dot=%.6f" % forward_dot)

	if expected_front_axle.distance_to(physical_front_axle) > AXLE_TOLERANCE_M:
		failures.append(
			"Jordan197 front axle no longer matches T_vehicle(source): expected=%s actual=%s"
			% [expected_front_axle, physical_front_axle]
		)
	if expected_rear_axle.distance_to(physical_rear_axle) > AXLE_TOLERANCE_M:
		failures.append(
			"Jordan197 rear axle no longer matches T_vehicle(source): expected=%s actual=%s"
			% [expected_rear_axle, physical_rear_axle]
		)

	var wheel_contract := {
		"WheelFrontLeft": ["DATUM_HUB_FL", front_static_drop],
		"WheelFrontRight": ["DATUM_HUB_FR", front_static_drop],
		"WheelRearLeft": ["DATUM_HUB_RL", rear_static_drop],
		"WheelRearRight": ["DATUM_HUB_RR", rear_static_drop],
	}
	for wheel_name in wheel_contract:
		var wheel := vehicle.get_node_or_null(wheel_name) as Node3D
		if wheel == null:
			continue
		var contract: Array = wheel_contract[wheel_name]
		var expected_hub := _source_anchor_runtime(source_assembly, manifest, contract[0], failures)
		var actual_static_hub := wheel.position + Vector3.DOWN * float(contract[1])
		var error := expected_hub.distance_to(actual_static_hub)
		if error > WHEEL_CENTER_TOLERANCE_M:
			failures.append(
				"Jordan197 %s violates SOURCE hub relationship: error=%.3f mm"
				% [wheel_name, error * 1000.0]
			)


func _check_wheel_scene(vehicle: Node, node_path: String, expected_scene: String, failures: Array[String]) -> void:
	var visual := vehicle.get_node_or_null(node_path)
	if visual == null:
		return
	if visual.scene_file_path != expected_scene:
		failures.append("Jordan197 wheel %s uses %s instead of %s" % [node_path, visual.scene_file_path, expected_scene])


func _check_authored_alignment(vehicle: Node, failures: Array[String]) -> void:
	if vehicle == null:
		failures.append("Jordan197 authored visual alignment cannot be checked because VehicleRigidBody is missing")
		return
	var chassis := vehicle.get_node_or_null("ChassisVisual") as Node3D
	if chassis == null:
		failures.append("Jordan197 authored chassis visual is missing")
	elif chassis.position.length() > 0.0001:
		failures.append("Jordan197 chassis visual has an authored translation: %s" % chassis.position)
	for pivot_path in [
		"WheelFrontLeft/Pivot",
		"WheelFrontRight/Pivot",
		"WheelRearLeft/Pivot",
		"WheelRearRight/Pivot",
	]:
		var pivot := vehicle.get_node_or_null(pivot_path) as Node3D
		if pivot == null:
			failures.append("Jordan197 wheel pivot is missing: %s" % pivot_path)
		elif pivot.position.length() > 0.0001:
			failures.append("Jordan197 wheel pivot has an authored translation: %s=%s" % [pivot_path, pivot.position])


func _check_wheel_centers(vehicle: Node, failures: Array[String]) -> void:
	for wheel_path in [
		"WheelFrontLeft/Pivot/Visual",
		"WheelFrontRight/Pivot/Visual",
		"WheelRearLeft/Pivot/Visual",
		"WheelRearRight/Pivot/Visual",
	]:
		var visual := vehicle.get_node_or_null(wheel_path)
		var raycast_path: String = wheel_path.get_slice("/", 0)
		var raycast := vehicle.get_node_or_null(raycast_path) as RayCast3D
		var wheel_node: Node3D = raycast.get("wheel_node") as Node3D if raycast else null
		if visual == null or raycast == null or wheel_node == null:
			continue
		var bounds := _world_bounds(visual)
		if bounds.size == Vector3.ZERO:
			failures.append("Jordan197 wheel visual has no mesh bounds: %s" % wheel_path)
			continue
		var visual_center := bounds.position + bounds.size * 0.5
		var center_error := visual_center.distance_to(wheel_node.global_position)
		if center_error > WHEEL_CENTER_TOLERANCE_M:
			failures.append("Jordan197 wheel visual is displaced from its GEVP pivot: %s error=%.6f" % [wheel_path, center_error])


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


func _load_manifest(failures: Array[String]) -> Dictionary:
	return _load_json_file(MANIFEST_PATH, "manifest", failures)


func _load_json_file(path: String, label: String, failures: Array[String]) -> Dictionary:
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		failures.append("Jordan197 %s cannot be opened: %s" % [label, path])
		return {}
	var parsed: Variant = JSON.parse_string(file.get_as_text())
	if not parsed is Dictionary:
		failures.append("Jordan197 %s is not valid JSON: %s" % [label, path])
		return {}
	return parsed as Dictionary


func _source_anchor_runtime(
	source_assembly: Dictionary,
	manifest: Dictionary,
	anchor_name: String,
	failures: Array[String]
) -> Vector3:
	var anchors: Dictionary = source_assembly.get("anchors_source_m", {})
	var source_point: Array = anchors.get(anchor_name, [])
	var runtime: Dictionary = manifest.get("runtime", {})
	var rotation: Array = runtime.get("source_to_runtime_rotation_3x3", [])
	var translation: Array = runtime.get("translation_after_rotation", [])
	if source_point.size() != 3 or rotation.size() != 3 or translation.size() != 3:
		failures.append("Jordan197 cannot transform SOURCE anchor %s" % anchor_name)
		return Vector3.ZERO
	var source := Vector3(float(source_point[0]), float(source_point[1]), float(source_point[2]))
	return Vector3(
		float(rotation[0][0]) * source.x + float(rotation[0][1]) * source.y + float(rotation[0][2]) * source.z + float(translation[0]),
		float(rotation[1][0]) * source.x + float(rotation[1][1]) * source.y + float(rotation[1][2]) * source.z + float(translation[1]),
		float(rotation[2][0]) * source.x + float(rotation[2][1]) * source.y + float(rotation[2][2]) * source.z + float(translation[2])
	)


func _mesh_stats(parent: Node) -> Dictionary:
	var stats := {"surfaces": 0, "materials": 0}
	if parent == null:
		return stats
	var material_ids: Dictionary = {}
	var pending: Array[Node] = [parent]
	while not pending.is_empty():
		var current: Node = pending.pop_back()
		if current is MeshInstance3D and current.mesh != null:
			for surface_index in current.mesh.get_surface_count():
				stats.surfaces += 1
				var material: Material = current.get_active_material(surface_index)
				if material != null:
					material_ids[material.get_instance_id()] = true
		for child in current.get_children():
			pending.append(child)
	stats.materials = material_ids.size()
	return stats
