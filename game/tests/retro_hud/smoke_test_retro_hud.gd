extends SceneTree

func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var failures: Array[String] = []
	var display_scene := load("res://features/retro_hud/scenes/retro_hud_display.tscn") as PackedScene
	if display_scene == null:
		printerr("[FAIL] Retro HUD display scene could not load.")
		quit(1)
		return
	var display := display_scene.instantiate()
	root.add_child(display)
	await process_frame
	if display.base.texture == null:
		failures.append("generated retro HUD base texture is missing")
	if display.tachometer == null:
		failures.append("tachometer overlay is missing")
	if display.speed_value == null or display.gear_value == null:
		failures.append("manual glyph readouts are missing")
	display.set_readout(254.4, 11950.0, "4")
	await process_frame
	if display.speed_value.text != "254" or display.gear_value.text != "4":
		failures.append("state did not update dynamic readouts")
	var glyph_script := load("res://features/retro_hud/scripts/retro_hud_glyph_renderer.gd")
	if glyph_script == null or not glyph_script.STROKES.has("R") or not glyph_script.STROKES.has("N") or not glyph_script.STROKES.has("%"):
		failures.append("manual glyph catalogue is incomplete")
	if display.config.rpm_max <= display.config.rpm_redline:
		failures.append("tachometer configuration is invalid")
	display.queue_free()
	var demo_scene := load("res://features/retro_hud/scenes/retro_hud_demo.tscn") as PackedScene
	if demo_scene == null:
		failures.append("standalone retro HUD demo could not load")
	else:
		var demo := demo_scene.instantiate()
		root.add_child(demo)
		await process_frame
		var demo_display := demo.get_node_or_null("RetroHudDisplay")
		if demo_display == null or demo_display.get("state").rpm <= 0.0:
			failures.append("standalone mock provider did not drive the retro HUD")
		demo.queue_free()
	if failures.is_empty():
		print("[PASS] Retro HUD loads its generated base, config, state, tachometer, and manual glyphs.")
	else:
		for failure in failures:
			printerr("[FAIL] " + failure)
	quit(failures.size())
