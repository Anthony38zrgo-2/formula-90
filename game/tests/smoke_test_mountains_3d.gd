extends SceneTree

## Smoke test: factory-authored mountain integration with La Chutana.
## Near/Far must come exclusively from the published track GLB.

const SESSION_SCENE_PATH := "res://scenes/runtime/vehicle_test_session_2026.tscn"
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

	if rs.background_mountains_3d != null:
		_fail("BackgroundMountains3D runtime debe permanecer desactivado.")
	else:
		print("[OK] BackgroundMountains3D runtime desactivado.")

	var factory_mountains := _count_factory_mountains(rs.active_track)
	if factory_mountains != 2:
		_fail("Se esperaban 2 montañas dentro del GLB de fábrica; encontradas: %d" % factory_mountains)
	else:
		print("[OK] Near/Far provienen exclusivamente del GLB de fábrica.")

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


func _count_factory_mountains(track: Node) -> int:
	var count := 0
	var pending: Array[Node] = [track]
	while not pending.is_empty():
		var node: Node = pending.pop_back() as Node
		pending.append_array(node.get_children())
		var mesh_instance := node as MeshInstance3D
		if mesh_instance == null or mesh_instance.mesh == null:
			continue
		var size := mesh_instance.mesh.get_aabb().size
		if size.x >= 2500.0 and size.z >= 2500.0 and size.y <= 300.0 and mesh_instance.visible:
			count += 1
	return count


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
