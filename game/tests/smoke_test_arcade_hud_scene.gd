extends SceneTree


class StandaloneVehicle extends Node3D:
	var speed := 0.0
	var current_gear := 0


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var failures := 0
	var hud_scene := load("res://scenes/ui/debug_hud.tscn") as PackedScene
	if hud_scene == null:
		printerr("[FAIL] Arcade HUD scene could not load.")
		quit(1)
		return

	var standalone_world := Node3D.new()
	standalone_world.name = "StandaloneWorld"
	root.add_child(standalone_world)
	var vehicle_controller := Node3D.new()
	vehicle_controller.name = "VehicleController"
	standalone_world.add_child(vehicle_controller)
	var vehicle := StandaloneVehicle.new()
	vehicle.name = "VehicleRigidBody"
	vehicle_controller.add_child(vehicle)

	var hud := hud_scene.instantiate()
	standalone_world.add_child(hud)
	await process_frame
	await process_frame

	var minimap := hud.get_node_or_null("Minimap") as TrackMinimapController
	var lap_timing_panel := hud.get_node_or_null("LapTimingPanel") as LapTimingPanel
	var lap_and_map_background := hud.get_node_or_null("LapAndMapBackground") as ColorRect
	var speed_gauge := hud.get_node_or_null("SpeedGauge")
	var retro_hud := hud.get_node_or_null("RetroHud")
	var aid_message := hud.get_node_or_null("AidMessage") as Label
	if minimap == null or minimap.get("map_data") == null:
		printerr("[FAIL] Arcade HUD minimap or its track data is missing.")
		failures += 1
	elif not minimap.get("map_data").is_valid_map():
		printerr("[FAIL] Arcade HUD minimap data is invalid.")
		failures += 1
	elif not is_equal_approx(minimap.scale.x, 2.25) or not is_equal_approx(minimap.scale.y, 2.25):
		printerr("[FAIL] Minimap scale does not match the requested 25 percent reduction.")
		failures += 1
	elif minimap.get_node_or_null(minimap.target_path) != vehicle:
		printerr("[FAIL] Standalone HUD minimap did not resolve its local vehicle target.")
		failures += 1
	else:
		var player_before := minimap.get_player_map_position()
		vehicle.global_position = Vector3(150.0, 0.0, -250.0)
		await process_frame
		var player_after := minimap.get_player_map_position()
		if player_before.distance_to(player_after) < 1.0:
			printerr("[FAIL] Standalone HUD minimap player marker did not move with the vehicle.")
			failures += 1
	if lap_timing_panel == null or lap_and_map_background == null or minimap == null:
		printerr("[FAIL] Lap timing and map block is incomplete.")
		failures += 1
	else:
		var timing_rectangle := lap_timing_panel.get_global_rect()
		var map_rectangle := minimap.get_global_rect()
		var background_rectangle := lap_and_map_background.get_global_rect()
		var viewport_rectangle := Rect2(Vector2.ZERO, hud.get_viewport_rect().size)
		if timing_rectangle.end.y >= map_rectangle.position.y or map_rectangle.position.y - timing_rectangle.end.y > 24.0:
			printerr("[FAIL] Lap timing is not directly above the minimap.")
			failures += 1
		if absf(timing_rectangle.get_center().x - map_rectangle.get_center().x) > 1.0:
			printerr("[FAIL] Lap timing and minimap are not centered together.")
			failures += 1
		if not background_rectangle.encloses(timing_rectangle) or not background_rectangle.encloses(map_rectangle) or not viewport_rectangle.encloses(background_rectangle):
			printerr("[FAIL] Lap timing and minimap do not fit inside their shared left block.")
			failures += 1
		if not (lap_timing_panel.get_theme_stylebox("panel") is StyleBoxEmpty) or minimap.background_color.a > 0.0:
			printerr("[FAIL] Lap timing or minimap still draws a separate background.")
			failures += 1
	if speed_gauge == null or not speed_gauge.has_method("set_readout"):
		printerr("[FAIL] Arcade speed gauge is missing its readout API.")
		failures += 1
	if retro_hud == null or not retro_hud.has_method("set_readout"):
		printerr("[FAIL] Retro HUD is missing its decoupled readout API.")
		failures += 1
	elif retro_hud.get("state").gear_label != "N":
		printerr("[FAIL] Retro HUD did not receive the standalone adapter state.")
		failures += 1
	elif not is_equal_approx(retro_hud.scale.x, 1.1):
		printerr("[FAIL] Embedded Retro HUD scale was not applied from hud_config.json (expected 1.1).")
		failures += 1
	var tire_panel := hud.get_node_or_null("TireStatusPanel") as TireStatusPanel
	if tire_panel == null:
		printerr("[FAIL] Runtime-created Tyres panel is missing.")
		failures += 1
	elif not is_equal_approx(tire_panel.scale.x, 1.188):
		printerr("[FAIL] Tyres panel scale was not applied from hud_config.json (expected 1.188).")
		failures += 1
	if retro_hud is Control and tire_panel != null:
		var tachometer_rectangle := (retro_hud as Control).get_global_rect()
		var tire_panel_rectangle := tire_panel.get_global_rect()
		var viewport_size: Vector2 = hud.get_viewport_rect().size
		if tire_panel_rectangle.end.y > tachometer_rectangle.position.y:
			printerr("[FAIL] Tyres panel overlaps the tachometer.")
			failures += 1
		if tachometer_rectangle.end.x > viewport_size.x or tachometer_rectangle.end.y > viewport_size.y or tire_panel_rectangle.end.x > viewport_size.x or tire_panel_rectangle.position.y < 0.0:
			printerr("[FAIL] Enlarged HUD extends beyond the viewport.")
			failures += 1
		print("[HUD] Tachometer: ", tachometer_rectangle, " Tyres: ", tire_panel_rectangle, " Viewport: ", viewport_size)
	if aid_message == null or aid_message.visible:
		printerr("[FAIL] Aid message should begin hidden until an aid state changes.")
		failures += 1

	standalone_world.queue_free()
	if failures == 0:
		print("[PASS] Arcade HUD scene, map data, gauge, and hidden notification state are valid.")
	quit(failures)
