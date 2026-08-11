extends SceneTree


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var compositor_scene := load("res://scenes/runtime/world_hud_compositor.tscn") as PackedScene
	if compositor_scene == null:
		printerr("[FAIL] World/HUD compositor scene could not load.")
		quit(1)
		return

	var compositor := compositor_scene.instantiate()
	compositor.world_scene_path = "res://scenes/tracks/test_field/jordan_handling_test.tscn"
	root.add_child(compositor)
	await _await_render()
	await _await_render()
	paused = true

	var baseline_error := root.get_texture().get_image().save_png("user://world_hud_compositor_baseline.png")
	var world_presenter := compositor.get_node_or_null("WorldPresenter") as TextureRect
	if world_presenter == null:
		printerr("[FAIL] World presentation control is missing.")
		paused = false
		quit(1)
		return

	world_presenter.modulate = Color(0.55, 0.9, 1.0, 1.0)
	await _await_render()
	await _await_render()
	var tinted_error := root.get_texture().get_image().save_png("user://world_hud_compositor_world_tinted.png")
	world_presenter.modulate = Color.WHITE
	paused = false
	compositor.queue_free()

	if baseline_error != OK or tinted_error != OK:
		printerr("[FAIL] Could not save compositor captures.")
		quit(1)
		return

	print("[PASS] Captured ", ProjectSettings.globalize_path("user://world_hud_compositor_baseline.png"))
	print("[PASS] Captured ", ProjectSettings.globalize_path("user://world_hud_compositor_world_tinted.png"))
	print("[PASS] Capture size ", root.size)
	quit(0)


func _await_render() -> void:
	await process_frame
	await RenderingServer.frame_post_draw
