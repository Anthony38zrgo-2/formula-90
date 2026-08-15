extends SceneTree

## Smoke Test BG3-006: Validacion y capturas de La Chutana Background con F1-94
## Valida:
## 1. Configuracion y carga del preset la_chutana_snes_day en runtime.
## 2. Skybox desacoplado (BackgroundSkybox) instanciado por separado.
## 3. Invariantes de las 2 capas parallax: Nearest, unshaded, alpha, depth/render_priority y ausencia de colisiones.
## 4. Integracion en RaceSession con vehiculo F1-94 y desactivacion del rig legacy.
## 5. Captura visual y verificacion de parallax: Recta, Giro Izquierda (+30 deg), Giro Derecha (-30 deg).

const SESSION_SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"

var _failures: Array[String] = []


func _init() -> void:
	call_deferred("_run_smoke_test")


func _fail(msg: String) -> void:
	printerr("[FAIL] " + msg)
	_failures.append(msg)


func _run_smoke_test() -> void:
	print("=== INICIANDO SMOKE TEST BG3-006: LA CHUTANA 3-LAYER BACKGROUND ===")

	var packed := load(SESSION_SCENE_PATH) as PackedScene
	if packed == null:
		printerr("[INCONCLUSIVE] No se pudo cargar la escena de sesion de prueba: " + SESSION_SCENE_PATH)
		quit(1)
		return

	var compositor := packed.instantiate()
	root.add_child(compositor)

	for _frame in 6:
		await process_frame

	var race_session := compositor.get_node_or_null("WorldViewport/RaceSession") as RaceSession
	if race_session == null:
		printerr("[INCONCLUSIVE] RaceSession no encontrado en WorldViewport.")
		compositor.queue_free()
		quit(1)
		return

	var vehicle := race_session.active_vehicle
	if vehicle == null:
		_fail("F1-94 active_vehicle no esta instanciado en RaceSession.")

	var bg_controller := race_session.background_controller
	if bg_controller == null:
		_fail("BackgroundController no fue instanciado en RaceSession.")
		_finish(compositor)
		return

	# 1. Validar preset activo
	var active_preset := bg_controller.get_active_preset()
	if active_preset == null or active_preset.id != &"la_chutana_snes_day":
		_fail("El preset activo no es la_chutana_snes_day.")

	# 1b. Validar skybox desacoplado
	if race_session.background_skybox == null:
		_fail("BackgroundSkybox desacoplado no fue instanciado en RaceSession.")
	else:
		print("[OK] BackgroundSkybox desacoplado activo, independiente del controller de capas.")

	# 2. Validar que el rig legacy quedo oculto
	if race_session.active_track != null:
		var legacy_sky := race_session.active_track.get_node_or_null("SourceSkyboxRig") as Node3D
		if legacy_sky != null and legacy_sky.visible:
			_fail("SourceSkyboxRig legacy deberia estar oculto/desactivado.")
		else:
			print("[OK] Rig legacy ocultado correctamente frente al nuevo sistema multicapa.")

	# 3. Validar capas e invariantes estructurales
	var instances := bg_controller.get_layer_instances()
	if instances.size() != 2:
		_fail("Se esperaban exactamente 2 capas parallax instanciadas, encontradas: %d" % instances.size())
	else:
		var far_inst := bg_controller.get_layer_instance_by_id(&"far_mountains")
		var near_inst := bg_controller.get_layer_instance_by_id(&"near_mountains")

		if far_inst == null or near_inst == null:
			_fail("Una o mas capas obligatorias (far_mountains, near_mountains) no se pudieron obtener por ID.")
		else:
			# Chequeo de orden de profundidad y prioridades
			if far_inst.render_priority != 0 or near_inst.render_priority != 1:
				_fail("Render priorities incorrectas: far=%d, near=%d" % [far_inst.render_priority, near_inst.render_priority])
			
			if not (far_inst.position.z < near_inst.position.z and near_inst.position.z < 0.0):
				_fail("Orden de distancia Z incorrecto: far=%.1f, near=%.1f" % [far_inst.position.z, near_inst.position.z])

			for inst in [far_inst, near_inst]:
				if inst.texture == null:
					_fail("La capa '%s' no tiene textura asignada." % inst.name)
				if inst.texture_filter != BaseMaterial3D.TEXTURE_FILTER_NEAREST:
					_fail("La capa '%s' no utiliza filtrado Nearest." % inst.name)
				if inst.shaded:
					_fail("La capa '%s' tiene shaded activo (debe ser unshaded)." % inst.name)
				if inst.cast_shadow != GeometryInstance3D.SHADOW_CASTING_SETTING_OFF:
					_fail("La capa '%s' proyecta sombras." % inst.name)
				if inst.gi_mode != GeometryInstance3D.GI_MODE_DISABLED:
					_fail("La capa '%s' tiene GI activo." % inst.name)

			print("[OK] Invariantes estructurales (2 capas parallax, Nearest, Unshaded, sin sombras/GI, orden Z) verificados.")

	# 4. Validar ausencia de nodos de colision
	for child in bg_controller.find_children("*", "", true, false):
		if child is CollisionObject3D or child is NavigationRegion3D:
			_fail("BackgroundController contiene nodo no-visual: %s" % child.get_path())

	# 5. Capturas visuales y prueba dinamica de parallax con F1-94
	var camera_rig := race_session.get_node_or_null("CameraRig") as Node3D
	var camera3d := camera_rig.get_node_or_null("Camera3D") as Camera3D if camera_rig != null else null

	if camera3d == null:
		_fail("No se encontro Camera3D en CameraRig.")
		_finish(compositor)
		return

	var viewport: Viewport = compositor.get_node_or_null("WorldViewport") as SubViewport
	if viewport == null:
		viewport = root

	var test_angles := [
		{"deg": 0.0, "label": "straight"},
		{"deg": 30.0, "label": "turn_left"},
		{"deg": -30.0, "label": "turn_right"}
	]

	for step in test_angles:
		var deg: float = step["deg"]
		var label: String = step["label"]
		var rad := deg_to_rad(deg)

		# Rotar temporalmente la camara para simular el giro del F1-94
		camera3d.rotation = Vector3(0.0, rad, 0.0)
		bg_controller._update_layers_parallax()

		for _frame in 4:
			await process_frame
		RenderingServer.force_draw(false, 0.0)

		var far_inst := bg_controller.get_layer_instance_by_id(&"far_mountains")
		var near_inst := bg_controller.get_layer_instance_by_id(&"near_mountains")

		if far_inst != null and near_inst != null and deg != 0.0:
			var abs_far := absf(far_inst.position.x)
			var abs_near := absf(near_inst.position.x)

			if not (abs_far < abs_near):
				_fail("Jerarquia de parallax violada en %s: Far=%.2f, Near=%.2f" % [label, abs_far, abs_near])
			else:
				print("[OK] Parallax dinamico verificado en %s (deg=%.1f): Far=%.2f < Near=%.2f" % [label, deg, abs_far, abs_near])

		# Guardar captura
		if DisplayServer.get_name() != "headless":
			var img := viewport.get_texture().get_image()
			if img != null:
				var save_path := "user://la_chutana_3_layer_%s.png" % label
				var err := img.save_png(save_path)
				if err == OK:
					print("     -> Captura guardada: ", ProjectSettings.globalize_path(save_path))

	_finish(compositor)


func _finish(compositor: Node) -> void:
	if compositor != null:
		compositor.queue_free()

	print("\n=== RESUMEN SMOKE TEST BG3-006 ===")
	if _failures.is_empty():
		print("[PASS] La Chutana 3-Layer Background cumple 100% el contrato de arquitectura y render.")
		quit(0)
	else:
		printerr("[FAIL] Fallos detectados en smoke test (%d):" % _failures.size())
		for f in _failures:
			printerr("  - " + f)
		quit(_failures.size())
