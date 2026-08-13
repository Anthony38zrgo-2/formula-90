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
	await process_frame
	bootstrap.start_game()
	await process_frame
	await process_frame

	var compositor := bootstrap.get_node_or_null("Content")
	if compositor == null or compositor.get_node_or_null("WorldViewport/WorldContent") == null or compositor.get_node_or_null("HudLayer/DebugHud") == null:
		printerr("[FAIL] GameBootstrap did not start the world/HUD compositor.")
		quit(1)
		return

	var minimap := compositor.get_node_or_null("HudLayer/DebugHud/Minimap") as TrackMinimapController
	var vehicle := compositor.get_node_or_null("WorldViewport/WorldContent/Jordan191/VehicleRigidBody") as Node3D
	if vehicle == null:
		vehicle = compositor.get_node_or_null("WorldViewport/WorldContent/VehicleController/VehicleRigidBody") as Node3D
	var tuner := compositor.get_node_or_null("HudLayer/DebugHud/HandlingTuningPanel") as Control

	if minimap == null or vehicle == null or tuner == null:
		printerr("[FAIL] Minimap, vehicle, or handling tuner missing in default bootstrap route.")
		quit(1)
		return

	var tuning_panel := tuner.get_node_or_null("LiveTuningPanel") as Control
	var key_event := InputEventKey.new()
	key_event.keycode = KEY_F10
	key_event.pressed = true
	Input.parse_input_event(key_event)
	await process_frame
	if tuning_panel == null or not tuning_panel.visible:
		printerr("[FAIL] F10 did not open the handling tuner in the default bootstrap route.")
		quit(1)
		return

	if minimap.get("_target") != vehicle:
		printerr("[FAIL] Active minimap target is not the injected vehicle.")
		quit(1)
		return

	var before := minimap.get_player_map_position()
	var teleport_transform: Transform3D = vehicle.global_transform
	teleport_transform.origin = Vector3(150.0, teleport_transform.origin.y, -250.0)
	PhysicsServer3D.body_set_state(vehicle.get_rid(), PhysicsServer3D.BODY_STATE_TRANSFORM, teleport_transform)
	await physics_frame
	await physics_frame

	var after := minimap.get_player_map_position()
	if before.distance_to(after) < 1.0:
		printerr("[FAIL] Projected minimap position did not change after vehicle movement.")
		quit(1)
		return

	bootstrap.queue_free()
	print("[PASS] GameBootstrap routes gameplay through the world/HUD compositor with dynamic minimap tracking.")
	quit(0)
