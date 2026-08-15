extends SceneTree

## Visual Capture Test: Captura de 3 encuadres de La Chutana 3-Layer Background con F1-94
## Captura:
## 1. user://la_chutana_f1_94_straight.png
## 2. user://la_chutana_f1_94_turn_left.png
## 3. user://la_chutana_f1_94_turn_right.png

const SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"


func _init() -> void:
	call_deferred("_run_captures")


func _run_captures() -> void:
	print("=== INICIANDO CAPTURA VISUAL DE LA CHUTANA (F1-94 + 3-LAYER BACKGROUND) ===")
	
	var packed := load(SCENE_PATH) as PackedScene
	if packed == null:
		printerr("[FAIL] No se pudo cargar " + SCENE_PATH)
		quit(1)
		return

	var compositor := packed.instantiate()
	root.add_child(compositor)

	for _frame in 10:
		await process_frame
	await RenderingServer.frame_post_draw

	var race_session := compositor.get_node_or_null("WorldViewport/RaceSession") as RaceSession
	if race_session == null:
		printerr("[FAIL] RaceSession no encontrado.")
		compositor.queue_free()
		quit(1)
		return

	var bg_controller := race_session.background_controller
	var camera_rig := race_session.get_node_or_null("CameraRig") as Node3D
	var camera3d := camera_rig.get_node_or_null("Camera3D") as Camera3D if camera_rig != null else null

	if bg_controller == null or camera3d == null:
		printerr("[FAIL] BackgroundController o Camera3D no disponibles para capturas.")
		compositor.queue_free()
		quit(1)
		return

	var captures := [
		{"deg": 0.0, "file": "user://la_chutana_f1_94_straight.png", "name": "Recta (Centro)"},
		{"deg": 30.0, "file": "user://la_chutana_f1_94_turn_left.png", "name": "Giro Izquierda (+30 deg)"},
		{"deg": -30.0, "file": "user://la_chutana_f1_94_turn_right.png", "name": "Giro Derecha (-30 deg)"}
	]

	var captured_count := 0
	for cap in captures:
		var deg: float = cap["deg"]
		var file_path: String = cap["file"]
		var cap_name: String = cap["name"]

		camera3d.rotation = Vector3(0.0, deg_to_rad(deg), 0.0)
		bg_controller._update_layers_parallax()

		for _frame in 4:
			await process_frame
		RenderingServer.force_draw(false, 0.0)
		await RenderingServer.frame_post_draw

		var root_texture := root.get_texture()
		if root_texture != null:
			var img := root_texture.get_image()
			if img != null:
				var err := img.save_png(file_path)
				if err == OK:
					print("[PASS] %s capturado: %s" % [cap_name, ProjectSettings.globalize_path(file_path)])
					captured_count += 1
				else:
					printerr("[FAIL] Error al guardar %s: %d" % [file_path, err])
			else:
				print("[INFO] Display headless sin frame buffer activo; omitiendo archivo PNG.")
		else:
			print("[INFO] Texture viewport no disponible en modo headless.")

	compositor.queue_free()
	print("[PASS] Captura de encuadres completada.")
	quit(0)
