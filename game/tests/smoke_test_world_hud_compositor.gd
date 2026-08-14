extends SceneTree

func _init() -> void:
	call_deferred("_run")

func _run() -> void:
	var failures: Array[String] = []
	var packed := load("res://scenes/runtime/world_hud_compositor.tscn") as PackedScene
	var compositor := packed.instantiate()
	root.add_child(compositor)
	for _frame in 4:
		await process_frame
	var viewport := compositor.get_node_or_null("WorldViewport") as SubViewport
	var presenter := compositor.get_node_or_null("WorldPresenter") as TextureRect
	var session := compositor.get_node_or_null("WorldViewport/RaceSession") as RaceSession
	var hud := compositor.get_node_or_null("HudLayer/DebugHud") as ArcadeRaceHud
	if viewport == null or viewport.size != Vector2i(640, 360) or presenter == null or presenter.texture != viewport.get_texture():
		failures.append("fixed-resolution world presentation missing")
	if session == null or session.active_vehicle == null or session.active_track == null or viewport.get_camera_3d() == null:
		failures.append("RaceSession composition missing")
	if hud == null or hud.get("_vehicle") != session.active_vehicle or hud.get("_aids") != session.driving_aids:
		failures.append("root HUD direct binding missing")
	if session != null and session.get_node_or_null("DebugHud") != null:
		failures.append("HUD leaked into 3D session")
	compositor.queue_free()
	if failures.is_empty():
		print("[PASS] WorldHudCompositor presents RaceSession and binds HUD outside the 3D viewport.")
	else:
		for failure in failures:
			printerr("[FAIL] " + failure)
	quit(failures.size())
