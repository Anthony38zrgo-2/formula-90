extends SceneTree

const VEHICLE_SCENE := "res://scenes/vehicles/f1_2026_2008/f1_2026_2008_rust.tscn"
const WHEELS := ["FrontLeftWheel", "FrontRightWheel", "RearLeftWheel", "RearRightWheel"]
const CENTERS := [Vector3(-0.854, 0, -1.75), Vector3(0.854, 0, -1.75), Vector3(-0.795, 0, 1.75), Vector3(0.795, 0, 1.75)]

func _init() -> void:
	call_deferred("_run")

func _fail(message: String, failures: Array[String]) -> void:
	printerr("[FAIL] " + message)
	failures.append(message)

func _run() -> void:
	var failures: Array[String] = []
	var packed := load(VEHICLE_SCENE) as PackedScene
	if packed == null:
		_fail("Canonical F1-2026-2008 scene could not load.", failures)
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
			var spinner := camber.get_node_or_null("Spinner") if camber != null else null
			var visual := spinner.get_node_or_null("Visual") if spinner != null else null
			if hub == null or steer == null or camber == null or spinner == null or visual == null:
				_fail("Incomplete wheel visual hierarchy at %s." % wheel_name, failures)

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
				var planar_error := Vector2(hub.position.x - CENTERS[index].x, hub.position.z - CENTERS[index].z).length()
				if planar_error > 0.01:
					_fail("Wheel moved off its axle: %s at %s." % [WHEELS[index], hub.position], failures)
				var anchor: Vector3 = vehicle.call("get_wheel_anchor_local", index)
				var spring: float = controller.get("front_spring_length" if index < 2 else "rear_spring_length")
				var expected_y := anchor.y - spring + clampf(compressions[index] * 0.001, 0.0, spring)
				if absf(hub.position.y - expected_y) > 0.01:
					_fail("Wheel height disagrees with compression in mm: %s." % WHEELS[index], failures)
				var visible_meshes := 0
				for child in hub.find_children("*", "MeshInstance3D", true, false):
					var mesh_instance := child as MeshInstance3D
					if mesh_instance.mesh != null and mesh_instance.is_visible_in_tree():
						visible_meshes += 1
				if visible_meshes == 0:
					_fail("No visible wheel mesh: %s." % WHEELS[index], failures)

	instance.queue_free()
	if failures.is_empty():
		print("[PASS] F1-2026-2008 wheel meshes stay visible on their axles at the telemetry height.")
	quit(failures.size())
