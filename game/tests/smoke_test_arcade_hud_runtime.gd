extends SceneTree


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var failures := 0
	var compositor_scene := load("res://scenes/runtime/world_hud_compositor.tscn") as PackedScene
	if compositor_scene == null:
		printerr("[FAIL] World/HUD compositor scene could not load.")
		quit(1)
		return

	var compositor := compositor_scene.instantiate()
	compositor.world_scene_path = "res://scenes/tracks/test_field/jordan_handling_test.tscn"
	root.add_child(compositor)
	await process_frame
	await process_frame
	await process_frame

	var hud := compositor.get_node_or_null("HudLayer/DebugHud")
	var minimap := compositor.get_node_or_null("HudLayer/DebugHud/Minimap") as TrackMinimapController
	var speed_gauge := compositor.get_node_or_null("HudLayer/DebugHud/SpeedGauge")
	var aid_message := compositor.get_node_or_null("HudLayer/DebugHud/AidMessage") as Label
	var vehicle := compositor.get_node_or_null("WorldViewport/WorldContent/VehicleController/VehicleRigidBody") as Node3D
	var aids := compositor.get_node_or_null("WorldViewport/WorldContent/DrivingAids")

	if hud == null or minimap == null or speed_gauge == null or aid_message == null:
		printerr("[FAIL] Arcade HUD nodes were not extracted into HudLayer.")
		failures += 1
	if vehicle == null or aids == null:
		printerr("[FAIL] Runtime vehicle or driving aids are missing.")
		failures += 1
	if minimap != null and vehicle != null and minimap.get_node_or_null(minimap.get("target_path")) != vehicle:
		printerr("[FAIL] Dynamic minimap target path did not resolve to the vehicle.")
		failures += 1

	if minimap != null and vehicle != null:
		var before: Vector2 = minimap.get_player_map_position()
		vehicle.global_position = Vector3(150.0, vehicle.global_position.y, -250.0)
		var after: Vector2 = minimap.get_player_map_position()
		if before.distance_to(after) < 1.0:
			printerr("[FAIL] Dynamic minimap player marker did not move with the vehicle.")
			failures += 1

	if aids != null and aid_message != null:
		aids.call("toggle", 1)
		await process_frame
		if not aid_message.visible or aid_message.text != "AYUDA ESTAB ACTIVADA":
			printerr("[FAIL] Aid toggle did not produce the expected top-right notification.")
			failures += 1
		await create_timer(2.8).timeout
		if aid_message.visible:
			printerr("[FAIL] Aid notification did not fade out after its transient display period.")
			failures += 1

	if speed_gauge == null or not speed_gauge.has_method("set_readout"):
		printerr("[FAIL] Arcade speed gauge is not available at runtime.")
		failures += 1

	compositor.queue_free()
	if failures == 0:
		print("[PASS] Arcade HUD tracks the player and displays transient aid notifications in the root canvas.")
	quit(failures)
