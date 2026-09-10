extends SceneTree

const VEHICLE_SCENE := "res://scenes/vehicles/f1_2030_v10/f1_2030_v10_rust.tscn"
const WHEELS := ["FrontLeftWheel", "FrontRightWheel", "RearLeftWheel", "RearRightWheel"]
const CENTERS := [Vector3(-0.753, 0, -1.475), Vector3(0.753, 0, -1.475), Vector3(-0.716, 0, 1.475), Vector3(0.716, 0, 1.475)]
const WIDTHS := [0.33, 0.33, 0.42, 0.42]

func _init() -> void:
	call_deferred("_run")

func _fail(message: String, failures: Array[String]) -> void:
	printerr("[FAIL] " + message)
	failures.append(message)

func _mesh_bounds_in(mesh_instance: MeshInstance3D, local_root: Node3D) -> AABB:
	var source := mesh_instance.mesh.get_aabb()
	var to_local := local_root.global_transform.affine_inverse() * mesh_instance.global_transform
	var minimum := Vector3(INF, INF, INF)
	var maximum := Vector3(-INF, -INF, -INF)
	for corner_index in range(8):
		var corner := source.position + Vector3(
			source.size.x if corner_index & 1 else 0.0,
			source.size.y if corner_index & 2 else 0.0,
			source.size.z if corner_index & 4 else 0.0
		)
		var point := to_local * corner
		minimum = minimum.min(point)
		maximum = maximum.max(point)
	return AABB(minimum, maximum - minimum)

func _run() -> void:
	var failures: Array[String] = []
	var packed := load(VEHICLE_SCENE) as PackedScene
	if packed == null:
		_fail("Canonical F1 2030 V10 scene could not load.", failures)
		quit(1)
		return

	var instance := packed.instantiate()
	root.add_child(instance)
	await process_frame
	await physics_frame

	var vehicle := instance.get_node_or_null("VehicleRigidBody")
	var controller := vehicle.get_node_or_null("WheelVisualController") if vehicle != null else null
	if vehicle == null or controller == null:
		_fail("VehicleRigidBody or WheelVisualController is missing.", failures)
	else:
		for wheel_name in WHEELS:
			var hub := vehicle.get_node_or_null(wheel_name)
			var steer := hub.get_node_or_null("SteerPivot") if hub != null else null
			var camber := steer.get_node_or_null("CamberPivot") if steer != null else null
			var visual := camber.get_node_or_null("Visual") if camber != null else null
			var spinner := visual.get_node_or_null("SpinVisual") if visual != null else null
			var brake_static := visual.get_node_or_null("BrakeStatic") if visual != null else null
			if hub == null or steer == null or camber == null or spinner == null or visual == null or brake_static == null:
				_fail("Incomplete wheel visual hierarchy at %s." % wheel_name, failures)
			elif brake_static.find_children("*_NUT_05", "MeshInstance3D", true, false).size() != 1 \
				or brake_static.find_children("*_RIM_06", "MeshInstance3D", true, false).size() != 1:
				_fail("BrakeStatic must contain one caliper and one inboard duct at %s." % wheel_name, failures)
			elif not spinner.find_children("*_NUT_05", "MeshInstance3D", true, false).is_empty() \
				or not spinner.find_children("*_RIM_06", "MeshInstance3D", true, false).is_empty():
				_fail("Caliper or brake duct remains inside SpinVisual at %s." % wheel_name, failures)
			elif hub.get_node_or_null("TireSmokeEmitter") == null:
				_fail("Shared tire-smoke emitter is missing at %s." % wheel_name, failures)

		for native_path_name in [
			"front_left_wheel_node", "front_right_wheel_node",
			"rear_left_wheel_node", "rear_right_wheel_node",
		]:
			var native_path: Variant = vehicle.get(native_path_name)
			if native_path is NodePath and not (native_path as NodePath).is_empty():
				_fail("Native wheel writer remains enabled: %s." % native_path_name, failures)

		for _frame in 8:
			await physics_frame
		var controller_script: Variant = controller.get_script()
		if controller_script == null:
			_fail("WheelVisualController has no script.", failures)
		else:
			# A valid hierarchy can still hide every wheel inside the chassis.
			# Sample and render once so transforms match the telemetry checked here.
			controller.call("_physics_process", 1.0 / 60.0)
			for frame in range(60):
				controller.call("_process", 1.0 / 60.0)
			var compressions: PackedFloat64Array = vehicle.call("get_wheel_compressions")
			for index in range(4):
				var hub := vehicle.get_node(WHEELS[index]) as Node3D
				var visual := hub.get_node("SteerPivot/CamberPivot/Visual") as Node3D
				var spinner := visual.get_node("SpinVisual") as Node3D
				var brake_static := visual.get_node("BrakeStatic") as Node3D
				var planar_error := Vector2(hub.position.x - CENTERS[index].x, hub.position.z - CENTERS[index].z).length()
				if planar_error > 0.01:
					_fail("Wheel moved off its axle: %s at %s." % [WHEELS[index], hub.position], failures)
				var anchor: Vector3 = vehicle.call("get_wheel_anchor_local", index)
				var spring: float = controller.get("front_spring_length" if index < 2 else "rear_spring_length")
				var expected_y := anchor.y - spring + clampf(compressions[index] * 0.001, 0.0, spring)
				if absf(hub.position.y - expected_y) > 0.01:
					_fail("Wheel height disagrees with compression in mm: %s." % WHEELS[index], failures)
				if absf(hub.position.y - CENTERS[index].y) > 0.08:
					_fail("Wheel center is implausibly displaced from its native axle: %s at y=%.3f." % [WHEELS[index], hub.position.y], failures)
				var visible_meshes := 0
				var tire_mesh: MeshInstance3D = null
				for child in hub.find_children("*", "MeshInstance3D", true, false):
					var mesh_instance := child as MeshInstance3D
					if mesh_instance.mesh != null and mesh_instance.is_visible_in_tree():
						visible_meshes += 1
						if mesh_instance.name.ends_with("_TIRE"):
							tire_mesh = mesh_instance
				if visible_meshes == 0:
					_fail("No visible wheel mesh: %s." % WHEELS[index], failures)
				if tire_mesh == null:
					_fail("Tire mesh is missing from independent GLB: %s." % WHEELS[index], failures)
				else:
					# Measure inside the imported GLB root so steering, camber and toe
					# applied by the shared controller cannot inflate the axis-aligned box.
					var tire_bounds := _mesh_bounds_in(tire_mesh, visual)
					if absf(tire_bounds.size.x - WIDTHS[index]) > 0.015 or absf(tire_bounds.size.y - 0.66) > 0.015 or absf(tire_bounds.size.z - 0.66) > 0.015:
						_fail("Imported tire axes/dimensions are wrong at %s: %s." % [WHEELS[index], tire_bounds.size], failures)
				var static_transform := brake_static.global_transform
				spinner.rotate_x(0.5)
				if not brake_static.global_transform.is_equal_approx(static_transform):
					_fail("Brake caliper inherits wheel spin: %s." % WHEELS[index], failures)

	instance.queue_free()
	if failures.is_empty():
		print("[PASS] F1 2030 V10 wheel meshes stay visible on their axles at the telemetry height.")
	quit(failures.size())
