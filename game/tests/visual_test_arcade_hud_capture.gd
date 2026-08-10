extends SceneTree


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var bootstrap_scene := load("res://scenes/bootstrap/bootstrap.tscn") as PackedScene
	if bootstrap_scene == null:
		printerr("[FAIL] Bootstrap scene could not load.")
		quit(1)
		return

	var bootstrap := bootstrap_scene.instantiate()
	root.add_child(bootstrap)
	await _await_render()
	bootstrap.start_game()
	await _await_render()
	await _await_render()

	var compositor := bootstrap.get_node_or_null("Content")
	var minimap := compositor.get_node_or_null("HudLayer/DebugHud/Minimap") as TrackMinimapController if compositor != null else null
	var vehicle := compositor.get_node_or_null("WorldViewport/WorldContent/VehicleController/VehicleRigidBody") as Node3D if compositor != null else null
	var aids := compositor.get_node_or_null("WorldViewport/WorldContent/DrivingAids") if compositor != null else null

	if compositor == null or minimap == null or vehicle == null or aids == null:
		printerr("[FAIL] Bootstrap HUD nodes missing for visual capture.")
		quit(1)
		return

	var before := minimap.get_player_map_position()
	var teleport_transform: Transform3D = vehicle.global_transform
	teleport_transform.origin = Vector3(100.0, teleport_transform.origin.y, -150.0)
	PhysicsServer3D.body_set_state(vehicle.get_rid(), PhysicsServer3D.BODY_STATE_TRANSFORM, teleport_transform)
	await physics_frame
	await physics_frame
	await _await_render()

	var after := minimap.get_player_map_position()
	if before.distance_to(after) < 1.0:
		printerr("[FAIL] Minimap player marker did not update position in visual capture test.")
		quit(1)
		return

	aids.call("toggle", 1)
	await _await_render()
	paused = true

	var save_error := root.get_texture().get_image().save_png("user://arcade_hud_16x9.png")
	paused = false
	bootstrap.queue_free()
	if save_error != OK:
		printerr("[FAIL] Could not save arcade HUD capture.")
		quit(1)
		return

	print("[PASS] Captured ", ProjectSettings.globalize_path("user://arcade_hud_16x9.png"))
	print("[PASS] Capture size ", root.size)
	quit(0)


func _await_render() -> void:
	await process_frame
	await RenderingServer.frame_post_draw
