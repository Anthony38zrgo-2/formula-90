extends SceneTree


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var packed := load("res://scenes/runtime/vehicle_test_session.tscn") as PackedScene
	if packed == null:
		printerr("[FAIL] F1-94 La Chutana scene could not load.")
		quit(1)
		return
	var compositor := packed.instantiate()
	root.add_child(compositor)
	for _frame in 12:
		await process_frame
	RenderingServer.force_draw(false, 0.0)
	var output := "user://f1_94_la_chutana_hud.png"
	var root_texture := root.get_texture()
	if root_texture == null:
		printerr("[FAIL] Active display driver does not expose a root viewport texture.")
		quit(1)
		return
	var image := root_texture.get_image()
	if image == null:
		printerr("[FAIL] Active display driver returned no root viewport image.")
		quit(1)
		return
	var error := image.save_png(output)
	compositor.queue_free()
	if error != OK:
		printerr("[FAIL] Could not save F1-94 visual capture.")
		quit(1)
		return
	print("[PASS] Captured ", ProjectSettings.globalize_path(output))
	quit(0)
