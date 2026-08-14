extends SceneTree

func _init() -> void:
	call_deferred("_run")

func _run() -> void:
	var failures: Array[String] = []
	var packed := load("res://scenes/runtime/vehicle_test_session.tscn") as PackedScene
	var compositor := packed.instantiate()
	root.add_child(compositor)
	for _frame in 4:
		await process_frame
	var session := compositor.get_node_or_null("WorldViewport/RaceSession") as RaceSession
	var hud := compositor.get_node_or_null("HudLayer/DebugHud") as ArcadeRaceHud
	var minimap := compositor.get_node_or_null("HudLayer/DebugHud/Minimap") as TrackMinimapController
	var aid_message := compositor.get_node_or_null("HudLayer/DebugHud/AidMessage") as Label
	if session == null or hud == null or minimap == null or aid_message == null:
		failures.append("runtime composition or HUD nodes missing")
	else:
		if minimap.get("_target") != session.active_vehicle or minimap.map_data == null:
			failures.append("minimap direct binding missing")
		var before := minimap.get_player_map_position()
		session.active_vehicle.freeze = true
		session.active_vehicle.global_position.x += 2.0
		await process_frame
		if before.distance_to(minimap.get_player_map_position()) < 0.01:
			failures.append("minimap marker did not move")
		session.driving_aids.toggle(1)
		await process_frame
		if not aid_message.visible or aid_message.text != "AYUDA ESTAB ACTIVADA":
			failures.append("aid notification missing")
	compositor.queue_free()
	if failures.is_empty():
		print("[PASS] Arcade HUD is directly bound to the active RaceSession.")
	else:
		for failure in failures:
			printerr("[FAIL] " + failure)
	quit(failures.size())
