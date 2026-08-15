extends SceneTree

## Smoke test: BackgroundMountains3D integration with La Chutana.
## Verifies that the procedural 3D mountain system loads correctly.

const SESSION_SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"
var _failures: Array[String] = []

func _init():
	call_deferred("_run")

func _run():
	print("=== SMOKE TEST: MOUNTAINS 3D ===")

	var packed := load(SESSION_SCENE_PATH) as PackedScene
	if packed == null:
		_fail("No se pudo cargar la escena de sesion.")
		_report()
		return

	var comp := packed.instantiate()
	root.add_child(comp)

	# Wait for frames to settle
	for i in 10:
		await process_frame

	var rs := comp.get_node_or_null("WorldViewport/RaceSession") as RaceSession
	if rs == null:
		_fail("No se encontro RaceSession.")
		_report_and_quit(comp)
		return

	# Check BackgroundMountains3D exists
	var mountains := rs.background_mountains_3d
	if mountains == null:
		_fail("BackgroundMountains3D no esta instanciado.")
		_report_and_quit(comp)
		return

	# Check it's active
	if not mountains.is_active():
		_fail("BackgroundMountains3D no esta activo.")
	else:
		print("[OK] BackgroundMountains3D activo.")

	# Check debug info
	var info := mountains.get_debug_info()
	if not info.sky_dome:
		_fail("SkyDome no cargado.")
	else:
		print("[OK] SkyDome cargado.")

	if not info.far_ring:
		_fail("FarMountains ring no cargado.")
	else:
		print("[OK] FarMountains ring cargado.")

	if not info.near_ring:
		_fail("NearMountains ring no cargado.")
	else:
		print("[OK] NearMountains ring cargado.")

	if info.waterfalls < 3:
		_fail("Se esperaban 3 cascadas, encontradas: %d" % info.waterfalls)
	else:
		print("[OK] %d cascadas instanciadas." % info.waterfalls)

	# Check legacy is hidden
	var legacy := rs.active_track.get_node_or_null("SourceSkyboxRig") as Node3D if rs.active_track != null else null
	if legacy != null and legacy.visible:
		_fail("Legacy SourceSkyboxRig sigue visible.")
	else:
		print("[OK] Legacy SourceSkyboxRig oculto.")

	# Check no legacy 2D system
	if rs.background_skybox != null:
		_fail("BackgroundSkybox (legacy 2D) no deberia existir.")
	else:
		print("[OK] BackgroundSkybox no instanciado (deprecado).")

	if rs.background_controller != null:
		_fail("BackgroundController (legacy 2D) no deberia existir.")
	else:
		print("[OK] BackgroundController no instanciado (deprecado).")

	_report_and_quit(comp)


func _fail(msg: String) -> void:
	_failures.append(msg)
	printerr("[FAIL] %s" % msg)


func _report() -> void:
	print("\n=== RESUMEN SMOKE TEST MOUNTAINS 3D ===")
	if _failures.is_empty():
		print("Todos los checks pasaron.")
	else:
		print("Fallos detectados: %d" % _failures.size())
		for f in _failures:
			print("  - %s" % f)
	quit(1 if not _failures.is_empty() else 0)


func _report_and_quit(comp: Node) -> void:
	_report()
	comp.queue_free()
