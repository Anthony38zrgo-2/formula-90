extends SceneTree

func _init() -> void:
	call_deferred("capture_camera_views")

func capture_camera_views() -> void:
	var capture_arguments := OS.get_cmdline_user_args()
	if capture_arguments.is_empty():
		printerr("Cockpit capture requires an output directory.")
		quit(1)
		return
	var output_directory := capture_arguments[0]
	var compositor := (load("res://scenes/runtime/vehicle_test_session.tscn") as PackedScene).instantiate()
	root.add_child(compositor)
	for settle_frame in range(40):
		await process_frame
	var session := compositor.find_child("RaceSession", true, false)
	var vehicle := session.get("active_vehicle") as Node3D
	vehicle.set("freeze", true)
	session.call("toggle_camera")
	session.call("toggle_camera")
	for settle_frame in range(12):
		await process_frame
	await RenderingServer.frame_post_draw
	var cockpit_image := root.get_texture().get_image()
	var cockpit_result := cockpit_image.save_png(output_directory.path_join("cockpit_neutral.png"))
	var world_viewport := compositor.get_node("WorldViewport") as SubViewport
	var world_result := world_viewport.get_texture().get_image().save_png(output_directory.path_join("cockpit_world.png"))
	var driver_controller := vehicle.get_node("DriverVisualController")
	var driver_eye_point := driver_controller.call("get_driver_eye_point") as Node3D
	var road_query := PhysicsRayQueryParameters3D.create(driver_eye_point.global_position, driver_eye_point.global_position + Vector3.DOWN * 10.0, 1)
	var road_intersection := vehicle.get_world_3d().direct_space_state.intersect_ray(road_query)
	if not road_intersection.is_empty():
		print("COCKPIT_BASELINE_EYE_HEIGHT_ABOVE_ROAD_METERS=" + str(driver_eye_point.global_position.y - road_intersection["position"].y))
	session.call("toggle_camera")
	var external_camera := Camera3D.new()
	session.add_child(external_camera)
	external_camera.global_position = vehicle.global_transform * Vector3(-0.6, 0.6, -0.8)
	external_camera.look_at(driver_eye_point.global_position, Vector3.UP)
	external_camera.fov = 55.0
	external_camera.near = 0.015
	external_camera.current = true
	for settle_frame in range(12):
		await process_frame
	await RenderingServer.frame_post_draw
	var external_result := world_viewport.get_texture().get_image().save_png(output_directory.path_join("driver_external.png"))
	print("COCKPIT_CAPTURE_DIRECTORY=" + output_directory)
	print("COCKPIT_CAPTURE_ERRORS=" + str([cockpit_result, world_result, external_result]))
	quit(0 if cockpit_result == OK and world_result == OK and external_result == OK else 1)
