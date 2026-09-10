extends SceneTree

const WHEELS := ["FrontLeftWheel", "FrontRightWheel", "RearLeftWheel", "RearRightWheel"]
const KEYS := ["FL", "FR", "RL", "RR"]

func _init() -> void:
	call_deferred("_run")

func _capture(viewport: Viewport, path: String) -> void:
	await process_frame
	await RenderingServer.frame_post_draw
	var error := viewport.get_texture().get_image().save_png(path)
	assert(error == OK, "Capture failed: " + path)
	print("CAPTURE ", ProjectSettings.globalize_path(path))

func _run() -> void:
	var compositor: Node = load("res://scenes/runtime/vehicle_test_session.tscn").instantiate()
	root.add_child(compositor)
	for frame in 240:
		await physics_frame
	await process_frame
	var vehicle := compositor.find_child("VehicleRigidBody", true, false) as RigidBody3D
	var driving := "--drive" in OS.get_cmdline_user_args()
	if driving:
		vehicle.call("set_automatic_transmission", true)
		vehicle.call("set_gear_request", 1)
		Input.action_press("Throttle", 0.6)
		for frame in 360:
			await physics_frame
		Input.action_release("Throttle")
	var controller := vehicle.get_node("WheelVisualController")
	controller.call("_physics_process", 1.0 / 60.0)
	for frame in 60:
		controller.call("_process", 1.0 / 60.0)
	var compressions: PackedFloat64Array = vehicle.call("get_wheel_compressions")
	var report: Dictionary = {}
	var failures := 0
	var speed_kmh: float = vehicle.call("get_speed_kmh")
	print("SPEED_KMH ", speed_kmh)
	if driving and speed_kmh < 5.0:
		failures += 1
	for index in 4:
		var hub := vehicle.get_node(WHEELS[index]) as Node3D
		var ray := vehicle.get_node("RayCast_%s_Mid" % KEYS[index]) as RayCast3D
		ray.force_raycast_update()
		var anchor: Vector3 = vehicle.call("get_wheel_anchor_local", index)
		var gap := INF
		if ray.is_colliding():
			var up := vehicle.global_basis.y.normalized()
			gap = (hub.global_position - ray.get_collision_point()).dot(up) - 0.33
		if not is_finite(gap) or absf(gap) > 0.025:
			failures += 1
		report[KEYS[index]] = {"hub_local": var_to_str(hub.position), "anchor_local": var_to_str(anchor), "compression_mm": compressions[index], "tire_contact_gap_m": gap}
	print("ALIGNMENT ", JSON.stringify(report))
	var suffix := "before" if "--before" in OS.get_cmdline_user_args() else "after"
	if driving:
		suffix = "moving_after"
	var file := FileAccess.open("user://f1_2030_runtime_alignment_%s.json" % suffix, FileAccess.WRITE)
	file.store_string(JSON.stringify(report, "\t"))
	file.close()
	var session := compositor.find_child("RaceSession", true, false)
	session.call("toggle_camera")
	await process_frame
	paused = true
	await _capture(root, "user://f1_2030_runtime_tcam_%s.png" % suffix)
	var camera := Camera3D.new()
	vehicle.add_child(camera)
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.size = 3.3
	camera.position = Vector3(-7, 0.12, 0)
	camera.look_at(vehicle.to_global(Vector3(0, 0.12, 0)), vehicle.global_basis.y.normalized())
	camera.make_current()
	await _capture(vehicle.get_viewport(), "user://f1_2030_runtime_side_%s.png" % suffix)
	camera.size = 2.1
	camera.position = Vector3(0, 0.18, -7)
	camera.look_at(vehicle.to_global(Vector3(0, 0.05, -0.8)), vehicle.global_basis.y.normalized())
	await _capture(vehicle.get_viewport(), "user://f1_2030_runtime_front_%s.png" % suffix)
	paused = false
	print("CONTACT_FAILURES ", failures)
	quit(failures)
