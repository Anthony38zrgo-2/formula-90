extends SceneTree

## Suite de pruebas para BackgroundController y BackgroundLayerInstance (BG3-004)

const VALID_SKY_TEX := "res://assets/backgrounds/spike_la_chutana_v3/sky.png"
const VALID_FAR_TEX := "res://assets/backgrounds/spike_la_chutana_v3/far_mountains.png"
const VALID_NEAR_TEX := "res://assets/backgrounds/spike_la_chutana_v3/near_mountains.png"

var _failures := 0


func _init() -> void:
	call_deferred("_run_tests")


func _run_tests() -> void:
	print("=== INICIANDO PRUEBAS DE BG3-004: BACKGROUND CONTROLLER ===")

	var root_node := Node3D.new()
	root_node.name = "TestRoot"
	root.add_child(root_node)

	var camera := Camera3D.new()
	camera.name = "TestCamera"
	camera.position = Vector3(10.0, 2.0, 5.0)
	root_node.add_child(camera)

	var controller := BackgroundController.new()
	controller.name = "BackgroundController"
	root_node.add_child(controller)
	controller.set_camera_source(camera)

	_test_load_preset(controller)
	_test_camera_follow(controller, camera)
	_test_parallax_motion(controller, camera)
	_test_enable_toggle(controller)
	_test_reject_invalid(controller)
	_test_extensible_layers(controller)

	root_node.queue_free()

	print("\n=== RESUMEN DE PRUEBAS BG3-004 ===")
	if _failures == 0:
		print("[PASS] Todas las pruebas de BackgroundController pasaron exitosamente.")
		quit(0)
	else:
		printerr("[FAIL] Se detectaron %d fallos en las pruebas del BackgroundController." % _failures)
		quit(1)


func _create_sample_preset() -> BackgroundPreset:
	var preset := BackgroundPreset.new()
	preset.id = &"sample_snes_preset"
	preset.display_name = "Sample SNES Preset"

	var l0 := BackgroundLayerConfig.new()
	l0.id = &"sky"
	l0.texture_path = VALID_SKY_TEX
	l0.depth = 0
	l0.parallax_x = 0.02
	l0.distance_z = -800.0
	preset.layers.append(l0)

	var l1 := BackgroundLayerConfig.new()
	l1.id = &"far"
	l1.texture_path = VALID_FAR_TEX
	l1.depth = 1
	l1.parallax_x = 0.08
	l1.distance_z = -700.0
	preset.layers.append(l1)

	var l2 := BackgroundLayerConfig.new()
	l2.id = &"near"
	l2.texture_path = VALID_NEAR_TEX
	l2.depth = 2
	l2.parallax_x = 0.18
	l2.distance_z = -600.0
	preset.layers.append(l2)

	return preset


func _test_load_preset(controller: BackgroundController) -> void:
	var preset := _create_sample_preset()
	var success := controller.load_preset(preset)
	if not success:
		printerr("[FAIL] _test_load_preset: load_preset retorno false.")
		_failures += 1
		return

	var instances := controller.get_layer_instances()
	if instances.size() != 3:
		printerr("[FAIL] _test_load_preset: Se esperaban 3 instancias de capa, obtenidas %d." % instances.size())
		_failures += 1
		return

	var sky_inst := controller.get_layer_instance_by_id(&"sky")
	var far_inst := controller.get_layer_instance_by_id(&"far")
	var near_inst := controller.get_layer_instance_by_id(&"near")

	if sky_inst == null or far_inst == null or near_inst == null:
		printerr("[FAIL] _test_load_preset: No se pudieron obtener las capas por ID.")
		_failures += 1
		return

	if sky_inst.render_priority != 0 or far_inst.render_priority != 1 or near_inst.render_priority != 2:
		printerr("[FAIL] _test_load_preset: Render priorities no coinciden con depths.")
		_failures += 1
		return

	print("[OK] _test_load_preset: 3 capas cargadas y verificadas correctamente.")


func _test_camera_follow(controller: BackgroundController, camera: Camera3D) -> void:
	camera.global_position = Vector3(150.0, 10.0, -320.0)
	controller._follow_active_camera()

	if not controller.global_position.is_equal_approx(Vector3(150.0, 0.0, -320.0)) and not controller.global_position.is_equal_approx(camera.global_position):
		printerr("[FAIL] _test_camera_follow: El controlador no siguio la posicion de la camara: %s" % controller.global_position)
		_failures += 1
	else:
		print("[OK] _test_camera_follow: El controlador sigue la traslacion de la camara.")


func _test_parallax_motion(controller: BackgroundController, camera: Camera3D) -> void:
	# Rotar camara a +30 grados en yaw
	camera.rotation = Vector3(0.0, deg_to_rad(30.0), 0.0)
	controller._update_layers_parallax()

	var sky_inst := controller.get_layer_instance_by_id(&"sky")
	var far_inst := controller.get_layer_instance_by_id(&"far")
	var near_inst := controller.get_layer_instance_by_id(&"near")

	var sky_x := absf(sky_inst.position.x)
	var far_x := absf(far_inst.position.x)
	var near_x := absf(near_inst.position.x)

	if not (sky_x < far_x and far_x < near_x):
		printerr("[FAIL] _test_parallax_motion: Jerarquia de parallax incorrecta (Sky=%.2f, Far=%.2f, Near=%.2f)." % [sky_x, far_x, near_x])
		_failures += 1
	else:
		print("[OK] _test_parallax_motion: Jerarquia de parallax correcta: Sky(%.2f) < Far(%.2f) < Near(%.2f)." % [sky_x, far_x, near_x])


func _test_enable_toggle(controller: BackgroundController) -> void:
	controller.set_enabled(false)
	if controller.is_enabled() or controller.visible:
		printerr("[FAIL] _test_enable_toggle: set_enabled(false) no desactivo visibilidad/estado.")
		_failures += 1
		return

	controller.set_enabled(true)
	if not controller.is_enabled() or not controller.visible:
		printerr("[FAIL] _test_enable_toggle: set_enabled(true) no restauro visibilidad/estado.")
		_failures += 1
		return

	print("[OK] _test_enable_toggle: Activar/desactivar funciona correctamente.")


func _test_reject_invalid(controller: BackgroundController) -> void:
	var invalid_preset := BackgroundPreset.new()
	invalid_preset.id = &"" # Invalido

	var success := controller.load_preset(invalid_preset)
	if success:
		printerr("[FAIL] _test_reject_invalid: load_preset deberia haber retornado false para preset invalido.")
		_failures += 1
	else:
		print("[OK] _test_reject_invalid: El controlador rechazo el preset invalido de forma segura.")


func _test_extensible_layers(controller: BackgroundController) -> void:
	var multi_preset := BackgroundPreset.new()
	multi_preset.id = &"multi_5_layers"

	for i in range(5):
		var l := BackgroundLayerConfig.new()
		l.id = StringName("layer_%d" % i)
		l.texture_path = VALID_SKY_TEX
		l.depth = i
		l.parallax_x = 0.02 * (i + 1)
		l.distance_z = -800.0 + (i * 50.0)
		multi_preset.layers.append(l)

	var success := controller.load_preset(multi_preset)
	if not success or controller.get_layer_instances().size() != 5:
		printerr("[FAIL] _test_extensible_layers: No se pudieron cargar 5 capas dinamicamente.")
		_failures += 1
	else:
		print("[OK] _test_extensible_layers: El sistema admite N capas arbitrarias sin cambios de codigo (5/5 capas creadas).")
