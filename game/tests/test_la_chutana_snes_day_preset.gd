extends SceneTree

## Suite de validacion para el preset la_chutana_snes_day y su integracion en pista (BG3-005)

const PRESET_PATH := "res://assets/backgrounds/la_chutana_snes_day/background.json"
const TRACK_DEF_PATH := "res://data/tracks/la_chutana.tres"
const SESSION_CONFIG_PATH := "res://data/race_sessions/f1_94_la_chutana.tres"

var _failures := 0


func _init() -> void:
	call_deferred("_run_tests")


func _run_tests() -> void:
	print("=== INICIANDO PRUEBAS DE BG3-005: PRESET LA_CHUTANA_SNES_DAY ===")

	_test_preset_json_and_validation()
	_test_track_definition_link()
	_test_race_session_composition()
	_test_legacy_fallback_behavior()

	print("\n=== RESUMEN DE PRUEBAS BG3-005 ===")
	if _failures == 0:
		print("[PASS] El preset la_chutana_snes_day y la integracion desacoplada pasaron exitosamente.")
		quit(0)
	else:
		printerr("[FAIL] Se detectaron %d fallos en las pruebas del preset." % _failures)
		quit(1)


func _test_preset_json_and_validation() -> void:
	var preset := BackgroundPreset.load_from_json_file(PRESET_PATH)
	if preset == null:
		printerr("[FAIL] _test_preset_json_and_validation: No se pudo cargar '%s'." % PRESET_PATH)
		_failures += 1
		return

	var val := BackgroundValidator.validate_preset(preset)
	if not val.is_valid:
		printerr("[FAIL] _test_preset_json_and_validation: Fallo la validacion: %s" % val.get_error_summary())
		_failures += 1
		return

	if preset.id != &"la_chutana_snes_day":
		printerr("[FAIL] _test_preset_json_and_validation: ID incorrecto '%s'." % preset.id)
		_failures += 1
		return

	if preset.skybox == null:
		printerr("[FAIL] _test_preset_json_and_validation: El preset no declara skybox desacoplado.")
		_failures += 1
		return

	var sorted := preset.get_layers_sorted_by_depth()
	if sorted.size() != 2:
		printerr("[FAIL] _test_preset_json_and_validation: Se esperaban 2 capas parallax, encontradas %d." % sorted.size())
		_failures += 1
		return

	var l_far := sorted[0]
	var l_near := sorted[1]

	if l_far.id != &"far_mountains" or l_near.id != &"near_mountains":
		printerr("[FAIL] _test_preset_json_and_validation: Orden o IDs de capas incorrectos.")
		_failures += 1
		return

	if not (l_far.parallax_x < l_near.parallax_x):
		printerr("[FAIL] _test_preset_json_and_validation: Jerarquia de parallax configurada no cumple far < near.")
		_failures += 1
		return

	print("[OK] _test_preset_json_and_validation: Preset JSON valido, skybox desacoplado y parallax (%.2f < %.2f) verificado." % [l_far.parallax_x, l_near.parallax_x])


func _test_track_definition_link() -> void:
	var track_res := load(TRACK_DEF_PATH) as TrackDefinition
	if track_res == null:
		printerr("[FAIL] _test_track_definition_link: No se pudo cargar TrackDefinition desde '%s'." % TRACK_DEF_PATH)
		_failures += 1
		return

	var effective_preset := track_res.get_effective_background_preset()
	if effective_preset == null:
		printerr("[FAIL] _test_track_definition_link: get_effective_background_preset retorno null.")
		_failures += 1
		return

	if effective_preset.id != &"la_chutana_snes_day":
		printerr("[FAIL] _test_track_definition_link: Preset retornado '%s' no coincide." % effective_preset.id)
		_failures += 1
		return

	print("[OK] _test_track_definition_link: TrackDefinition de La Chutana resuelve correctamente el preset desacoplado.")


func _test_race_session_composition() -> void:
	var session_cfg := load(SESSION_CONFIG_PATH) as RaceSessionConfig
	if session_cfg == null:
		printerr("[FAIL] _test_race_session_composition: No se pudo cargar RaceSessionConfig.")
		_failures += 1
		return

	var session := RaceSession.new()
	session.name = "TestRaceSession"
	
	# Crear TrackContainer y VehicleContainer necesarios para RaceSession
	var tc := Node3D.new()
	tc.name = "TrackContainer"
	session.add_child(tc)
	
	var vc := Node3D.new()
	vc.name = "VehicleContainer"
	session.add_child(vc)
	
	root.add_child(session)
	var success := session.compose(session_cfg)
	if not success:
		printerr("[FAIL] _test_race_session_composition: session.compose retorno false.")
		_failures += 1
		session.queue_free()
		return

	if session.background_controller == null:
		printerr("[FAIL] _test_race_session_composition: background_controller no fue instanciado.")
		_failures += 1
	else:
		if session.background_controller.get_layer_instances().size() != 2:
			printerr("[FAIL] _test_race_session_composition: Se esperaban 2 instancias de capa parallax en runtime.")
			_failures += 1
		else:
			print("[OK] _test_race_session_composition: BackgroundController activo con 2 capas parallax instanciadas.")

	if session.background_skybox == null:
		printerr("[FAIL] _test_race_session_composition: background_skybox desacoplado no fue instanciado.")
		_failures += 1
	else:
		print("[OK] _test_race_session_composition: BackgroundSkybox desacoplado activo junto al controller.")

	if session.active_track != null:
		var legacy_sky := session.active_track.get_node_or_null("SourceSkyboxRig") as Node3D
		if legacy_sky != null and legacy_sky.visible:
			printerr("[FAIL] _test_race_session_composition: SourceSkyboxRig legacy deberia estar oculto/desactivado.")
			_failures += 1
		else:
			print("[OK] _test_race_session_composition: Rig legacy ocultado correctamente al activarse el nuevo sistema.")

	session.queue_free()


func _test_legacy_fallback_behavior() -> void:
	# Verificar que una pista sin preset conserva el legacy visible
	var session := RaceSession.new()
	session.name = "FallbackTestSession"
	
	var tc := Node3D.new()
	tc.name = "TrackContainer"
	session.add_child(tc)
	
	var vc := Node3D.new()
	vc.name = "VehicleContainer"
	session.add_child(vc)
	
	var custom_cfg := RaceSessionConfig.new()
	var base_cfg := load(SESSION_CONFIG_PATH) as RaceSessionConfig
	custom_cfg.selected_vehicle = base_cfg.selected_vehicle

	var track_without_preset := TrackDefinition.new()
	track_without_preset.id = &"track_no_preset"
	track_without_preset.display_name = "Track No Preset"
	track_without_preset.track_scene = base_cfg.selected_track.track_scene
	custom_cfg.selected_track = track_without_preset

	root.add_child(session)
	var success := session.compose(custom_cfg)
	if not success:
		printerr("[FAIL] _test_legacy_fallback_behavior: session.compose fallo.")
		_failures += 1
	else:
		if session.background_controller != null:
			printerr("[FAIL] _test_legacy_fallback_behavior: No debio crearse BackgroundController si la pista no tiene preset.")
			_failures += 1
		
		if session.background_skybox != null:
			printerr("[FAIL] _test_legacy_fallback_behavior: No debio crearse BackgroundSkybox si la pista no tiene preset.")
			_failures += 1
		
		var legacy_sky := session.active_track.get_node_or_null("SourceSkyboxRig") as Node3D
		if legacy_sky != null and not legacy_sky.visible:
			printerr("[FAIL] _test_legacy_fallback_behavior: SourceSkyboxRig legacy deberia permanecer visible como fallback.")
			_failures += 1
		else:
			print("[OK] _test_legacy_fallback_behavior: Fallback legacy preservado cuando la pista no declara preset.")

	session.queue_free()
