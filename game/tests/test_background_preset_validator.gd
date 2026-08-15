extends SceneTree

## Suite de pruebas para BackgroundPreset, BackgroundLayerConfig y BackgroundValidator (BG3-003)

const VALID_SKY_TEX := "res://assets/backgrounds/spike_la_chutana_v3/sky.png"
const VALID_FAR_TEX := "res://assets/backgrounds/spike_la_chutana_v3/far_mountains.png"
const VALID_NEAR_TEX := "res://assets/backgrounds/spike_la_chutana_v3/near_mountains.png"

var _failures := 0


func _init() -> void:
	call_deferred("_run_tests")


func _run_tests() -> void:
	print("=== INICIANDO PRUEBAS DE BG3-003: BACKGROUND PRESET Y VALIDATOR ===")

	_test_valid_preset()
	_test_missing_preset_id()
	_test_empty_layers()
	_test_duplicate_layer_id()
	_test_duplicate_depth()
	_test_missing_texture_path()
	_test_non_positive_scale()
	_test_positive_distance_z()
	_test_negative_parallax()
	_test_depth_sorting()
	_test_valid_skybox()
	_test_missing_skybox_texture()
	_test_skybox_gradient_mode()
	_test_procedural_layer()
	_test_json_roundtrip()

	print("\n=== RESUMEN DE PRUEBAS BG3-003 ===")
	if _failures == 0:
		print("[PASS] Todas las pruebas de validacion y presets pasaron exitosamente.")
		quit(0)
	else:
		printerr("[FAIL] Se detectaron %d fallos en las pruebas de validacion." % _failures)
		quit(1)


func _test_valid_preset() -> void:
	var preset := BackgroundPreset.new()
	preset.id = &"test_snes_day"
	preset.display_name = "Test SNES Day"

	var l0 := BackgroundLayerConfig.new()
	l0.id = &"sky"
	l0.texture_path = VALID_SKY_TEX
	l0.depth = 0
	l0.parallax_x = 0.02
	l0.distance_z = -800.0
	preset.layers.append(l0)

	var l1 := BackgroundLayerConfig.new()
	l1.id = &"far_mountains"
	l1.texture_path = VALID_FAR_TEX
	l1.depth = 1
	l1.parallax_x = 0.08
	l1.distance_z = -700.0
	preset.layers.append(l1)

	var l2 := BackgroundLayerConfig.new()
	l2.id = &"near_mountains"
	l2.texture_path = VALID_NEAR_TEX
	l2.depth = 2
	l2.parallax_x = 0.18
	l2.distance_z = -600.0
	preset.layers.append(l2)

	var result := BackgroundValidator.validate_preset(preset)
	if not result.is_valid:
		printerr("[FAIL] _test_valid_preset deberia ser valido, pero fallo: %s" % result.get_error_summary())
		_failures += 1
	else:
		print("[OK] _test_valid_preset paso correctamente.")


func _test_missing_preset_id() -> void:
	var preset := BackgroundPreset.new()
	preset.id = &"" # Vacio

	var l0 := BackgroundLayerConfig.new()
	l0.id = &"sky"
	l0.texture_path = VALID_SKY_TEX
	l0.depth = 0
	l0.distance_z = -800.0
	preset.layers.append(l0)

	var result := BackgroundValidator.validate_preset(preset)
	if result.is_valid:
		printerr("[FAIL] _test_missing_preset_id deberia haber fallado.")
		_failures += 1
	else:
		print("[OK] _test_missing_preset_id detecto ID vacio correctamente: %s" % result.errors[0])


func _test_empty_layers() -> void:
	var preset := BackgroundPreset.new()
	preset.id = &"empty_preset"
	preset.layers = []

	var result := BackgroundValidator.validate_preset(preset)
	if result.is_valid:
		printerr("[FAIL] _test_empty_layers deberia haber fallado.")
		_failures += 1
	else:
		print("[OK] _test_empty_layers detecto lista vacia correctamente: %s" % result.errors[0])


func _test_duplicate_layer_id() -> void:
	var preset := BackgroundPreset.new()
	preset.id = &"dup_id_preset"

	var l0 := BackgroundLayerConfig.new()
	l0.id = &"sky"
	l0.texture_path = VALID_SKY_TEX
	l0.depth = 0
	l0.distance_z = -800.0
	preset.layers.append(l0)

	var l1 := BackgroundLayerConfig.new()
	l1.id = &"sky" # Duplicado!
	l1.texture_path = VALID_FAR_TEX
	l1.depth = 1
	l1.distance_z = -700.0
	preset.layers.append(l1)

	var result := BackgroundValidator.validate_preset(preset)
	if result.is_valid:
		printerr("[FAIL] _test_duplicate_layer_id deberia haber fallado.")
		_failures += 1
	else:
		print("[OK] _test_duplicate_layer_id detecto duplicado correctamente: %s" % result.errors[0])


func _test_duplicate_depth() -> void:
	var preset := BackgroundPreset.new()
	preset.id = &"dup_depth_preset"

	var l0 := BackgroundLayerConfig.new()
	l0.id = &"sky"
	l0.texture_path = VALID_SKY_TEX
	l0.depth = 0
	l0.distance_z = -800.0
	preset.layers.append(l0)

	var l1 := BackgroundLayerConfig.new()
	l1.id = &"far_mountains"
	l1.texture_path = VALID_FAR_TEX
	l1.depth = 0 # Duplicado!
	l1.distance_z = -700.0
	preset.layers.append(l1)

	var result := BackgroundValidator.validate_preset(preset)
	if result.is_valid:
		printerr("[FAIL] _test_duplicate_depth deberia haber fallado.")
		_failures += 1
	else:
		print("[OK] _test_duplicate_depth detecto conflicto de orden/depth correctamente: %s" % result.errors[0])


func _test_missing_texture_path() -> void:
	var preset := BackgroundPreset.new()
	preset.id = &"missing_tex_preset"

	var l0 := BackgroundLayerConfig.new()
	l0.id = &"sky"
	l0.texture_path = "res://assets/backgrounds/non_existent_sky_image.png"
	l0.depth = 0
	l0.distance_z = -800.0
	preset.layers.append(l0)

	var result := BackgroundValidator.validate_preset(preset)
	if result.is_valid:
		printerr("[FAIL] _test_missing_texture_path deberia haber fallado.")
		_failures += 1
	else:
		print("[OK] _test_missing_texture_path detecto textura inexistente: %s" % result.errors[0])


func _test_non_positive_scale() -> void:
	var preset := BackgroundPreset.new()
	preset.id = &"invalid_scale_preset"

	var l0 := BackgroundLayerConfig.new()
	l0.id = &"sky"
	l0.texture_path = VALID_SKY_TEX
	l0.depth = 0
	l0.scale = Vector2(-1.0, 1.0) # Invalido
	l0.distance_z = -800.0
	preset.layers.append(l0)

	var result := BackgroundValidator.validate_preset(preset)
	if result.is_valid:
		printerr("[FAIL] _test_non_positive_scale deberia haber fallado.")
		_failures += 1
	else:
		print("[OK] _test_non_positive_scale detecto escala <= 0: %s" % result.errors[0])


func _test_positive_distance_z() -> void:
	var preset := BackgroundPreset.new()
	preset.id = &"positive_z_preset"

	var l0 := BackgroundLayerConfig.new()
	l0.id = &"sky"
	l0.texture_path = VALID_SKY_TEX
	l0.depth = 0
	l0.distance_z = 100.0 # Invalido: delante de la camara
	preset.layers.append(l0)

	var result := BackgroundValidator.validate_preset(preset)
	if result.is_valid:
		printerr("[FAIL] _test_positive_distance_z deberia haber fallado.")
		_failures += 1
	else:
		print("[OK] _test_positive_distance_z detecto distance_z >= 0: %s" % result.errors[0])


func _test_negative_parallax() -> void:
	var preset := BackgroundPreset.new()
	preset.id = &"neg_parallax_preset"

	var l0 := BackgroundLayerConfig.new()
	l0.id = &"sky"
	l0.texture_path = VALID_SKY_TEX
	l0.depth = 0
	l0.parallax_x = -0.1 # Invalido: invertiria la direccion
	l0.distance_z = -800.0
	preset.layers.append(l0)

	var result := BackgroundValidator.validate_preset(preset)
	if result.is_valid:
		printerr("[FAIL] _test_negative_parallax deberia haber fallado.")
		_failures += 1
	else:
		print("[OK] _test_negative_parallax detecto parallax negativo: %s" % result.errors[0])


func _test_depth_sorting() -> void:
	var preset := BackgroundPreset.new()
	preset.id = &"sort_test"

	var l_near := BackgroundLayerConfig.new()
	l_near.id = &"near"
	l_near.texture_path = VALID_NEAR_TEX
	l_near.depth = 2
	l_near.distance_z = -600.0
	preset.layers.append(l_near)

	var l_sky := BackgroundLayerConfig.new()
	l_sky.id = &"sky"
	l_sky.texture_path = VALID_SKY_TEX
	l_sky.depth = 0
	l_sky.distance_z = -800.0
	preset.layers.append(l_sky)

	var l_far := BackgroundLayerConfig.new()
	l_far.id = &"far"
	l_far.texture_path = VALID_FAR_TEX
	l_far.depth = 1
	l_far.distance_z = -700.0
	preset.layers.append(l_far)

	var sorted := preset.get_layers_sorted_by_depth()
	if sorted[0].id != &"sky" or sorted[1].id != &"far" or sorted[2].id != &"near":
		printerr("[FAIL] _test_depth_sorting ordeno incorrectamente: %s, %s, %s" % [sorted[0].id, sorted[1].id, sorted[2].id])
		_failures += 1
	else:
		print("[OK] _test_depth_sorting ordeno capas por profundidad: sky (0) -> far (1) -> near (2).")


func _test_valid_skybox() -> void:
	var preset := BackgroundPreset.new()
	preset.id = &"skybox_valid_preset"
	preset.skybox = BackgroundSkyboxConfig.new()
	preset.skybox.mode = "gradient"
	preset.skybox.distance = 800.0
	preset.skybox.pixel_size = 0.0
	preset.skybox.gradient = {"zenith_color": [0.18, 0.42, 0.82], "horizon_color": [0.95, 0.92, 0.82]}

	var l0 := BackgroundLayerConfig.new()
	l0.id = &"far_mountains"
	l0.texture_path = VALID_FAR_TEX
	l0.depth = 0
	l0.distance_z = -700.0
	preset.layers.append(l0)

	var result := BackgroundValidator.validate_preset(preset)
	if not result.is_valid:
		printerr("[FAIL] _test_valid_skybox deberia ser valido, pero fallo: %s" % result.get_error_summary())
		_failures += 1
	else:
		print("[OK] _test_valid_skybox: skybox gradient desacoplado valido aceptado correctamente.")


func _test_missing_skybox_texture() -> void:
	var preset := BackgroundPreset.new()
	preset.id = &"skybox_missing_tex_preset"
	preset.skybox = BackgroundSkyboxConfig.new()
	preset.skybox.texture_path = "res://assets/backgrounds/non_existent_sky.png"
	preset.skybox.distance = 800.0

	var l0 := BackgroundLayerConfig.new()
	l0.id = &"far_mountains"
	l0.texture_path = VALID_FAR_TEX
	l0.depth = 0
	l0.distance_z = -700.0
	preset.layers.append(l0)

	var result := BackgroundValidator.validate_preset(preset)
	if result.is_valid:
		printerr("[FAIL] _test_missing_skybox_texture deberia haber fallado.")
		_failures += 1
	else:
		print("[OK] _test_missing_skybox_texture detecto textura de skybox inexistente: %s" % result.errors[0])


func _test_skybox_gradient_mode() -> void:
	var preset := BackgroundPreset.new()
	preset.id = &"skybox_gradient_preset"
	preset.skybox = BackgroundSkyboxConfig.new()
	preset.skybox.mode = "gradient"
	preset.skybox.distance = 800.0
	preset.skybox.gradient = {"zenith_color": [0.18, 0.42, 0.82], "horizon_color": [0.95, 0.92, 0.82]}

	var l0 := BackgroundLayerConfig.new()
	l0.id = &"far_mountains"
	l0.texture_path = VALID_FAR_TEX
	l0.depth = 0
	l0.distance_z = -700.0
	preset.layers.append(l0)

	var result := BackgroundValidator.validate_preset(preset)
	if not result.is_valid:
		printerr("[FAIL] _test_skybox_gradient_mode deberia ser valido: %s" % result.get_error_summary())
		_failures += 1
	else:
		print("[OK] _test_skybox_gradient_mode: skybox gradient aceptado correctamente.")


func _test_procedural_layer() -> void:
	var preset := BackgroundPreset.new()
	preset.id = &"procedural_layer_preset"

	var l0 := BackgroundLayerConfig.new()
	l0.id = &"far_mountains"
	l0.texture_path = VALID_FAR_TEX
	l0.depth = 0
	l0.distance_z = -700.0
	preset.layers.append(l0)

	var l1 := BackgroundLayerConfig.new()
	l1.id = &"clouds"
	l1.depth = 1
	l1.procedural = true
	l1.shader_path = "res://addons/formula90s/shaders/cloud_layer.gdshader"
	l1.distance_z = -500.0
	l1.pixel_size = 0.0
	preset.layers.append(l1)

	var result := BackgroundValidator.validate_preset(preset)
	if not result.is_valid:
		printerr("[FAIL] _test_procedural_layer deberia ser valido: %s" % result.get_error_summary())
		_failures += 1
	else:
		print("[OK] _test_procedural_layer: capa procedural aceptada correctamente.")


func _test_json_roundtrip() -> void:
	var json_dict := {
		"id": "json_preset_test",
		"display_name": "JSON Preset Test",
		"skybox": {
			"mode": "gradient",
			"distance": 800.0,
			"pixel_size": 0.0,
			"gradient": {
				"zenith_color": [0.18, 0.42, 0.82],
				"horizon_color": [0.95, 0.92, 0.82]
			},
			"time_of_day": 11.0
		},
		"layers": [
			{
				"id": "far_mountains",
				"texture": VALID_FAR_TEX,
				"depth": 0,
				"parallax_x": 0.08,
				"parallax_y": 0.01,
				"scale": 1.0,
				"offset_x": 0.0,
				"offset_y": 76.0,
				"repeat_x": false,
				"repeat_y": false,
				"pixel_snap": true,
				"distance_z": -700.0,
				"pixel_size": 1.5
			},
			{
				"id": "clouds",
				"depth": 2,
				"procedural": true,
				"shader_path": "res://addons/formula90s/shaders/cloud_layer.gdshader",
				"parallax_x": 0.28,
				"parallax_y": 0.0,
				"scale": 1.0,
				"offset_x": 0.0,
				"offset_y": 30.0,
				"repeat_x": false,
				"repeat_y": false,
				"pixel_snap": false,
				"distance_z": -500.0,
				"pixel_size": 0.0,
				"uniforms": {"cloud_scale": 0.012, "cloud_opacity": 0.6}
			}
		]
	}

	var preset := BackgroundPreset.from_dict(json_dict)
	var validation := BackgroundValidator.validate_preset(preset)
	if not validation.is_valid:
		printerr("[FAIL] _test_json_roundtrip fallo validacion: %s" % validation.get_error_summary())
		_failures += 1
		return

	var out_dict := preset.to_dict()
	if out_dict["id"] != "json_preset_test" or out_dict["layers"].size() != 2:
		printerr("[FAIL] _test_json_roundtrip no serializo correctamente.")
		_failures += 1
		return
	if out_dict["skybox"]["mode"] != "gradient" or out_dict["skybox"]["time_of_day"] != 11.0:
		printerr("[FAIL] _test_json_roundtrip no serializo el skybox gradient correctamente.")
		_failures += 1
		return
	if out_dict["layers"][1]["procedural"] != true or out_dict["layers"][1]["shader_path"] != "res://addons/formula90s/shaders/cloud_layer.gdshader":
		printerr("[FAIL] _test_json_roundtrip no serializo la capa procedural correctamente.")
		_failures += 1
	else:
		print("[OK] _test_json_roundtrip deserializo y serializo JSON (skybox gradient + capas procedural) correctamente.")
