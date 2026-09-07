extends SceneTree

func _init() -> void:
	call_deferred("_run")

func _run() -> void:
	var failures: Array[String] = []
	var packed := load("res://scenes/runtime/world_hud_compositor.tscn") as PackedScene
	var compositor := packed.instantiate()
	root.add_child(compositor)
	for _frame in 4:
		await process_frame
	var viewport := compositor.get_node_or_null("WorldViewport") as SubViewport
	var presenter := compositor.get_node_or_null("DisplayAspect/DisplayStage/WorldPresenter") as TextureRect
	var display_stage := compositor.get_node_or_null("DisplayAspect/DisplayStage") as Control
	var hud_layer := compositor.get_node_or_null("DisplayAspect/DisplayStage/HudLayer") as Control
	var session := compositor.get_node_or_null("WorldViewport/RaceSession") as RaceSession
	var hud := compositor.get_node_or_null("DisplayAspect/DisplayStage/HudLayer/DebugHud") as ArcadeRaceHud
	var psx_art := compositor.get_node_or_null("PsxArtController") as PsxArtController
	var expected_viewport_size := Vector2i(psx_art.get_internal_width(), psx_art.get_internal_height())
	if psx_art.get_upscale_mode() == 3:
		expected_viewport_size = Vector2i(roundi(presenter.size.x), roundi(presenter.size.y))
	if viewport == null or viewport.size != expected_viewport_size or viewport.msaa_3d != Viewport.MSAA_2X or presenter == null or presenter.texture != viewport.get_texture():
		failures.append("fixed-resolution world presentation missing")
	var presenter_aspect := presenter.size.x / presenter.size.y if presenter != null and presenter.size.y > 0.0 else 0.0
	if presenter == null or not is_equal_approx(presenter_aspect, 16.0 / 9.0) or hud == null or hud.size != presenter.size:
		failures.append("16:9 world and HUD layout missing (stage=%s presenter=%s layer=%s hud=%s aspect=%f)" % [display_stage.size, presenter.size, hud_layer.size, hud.size, presenter_aspect])
	if presenter == null or presenter.texture_filter != CanvasItem.TEXTURE_FILTER_NEAREST:
		failures.append("world presenter must preserve nearest texture filtering")
	var filtered_materials := {"total": 0, "high_quality": 0}
	_scan_texture_filters(session if session != null else null, filtered_materials)
	if filtered_materials.total > 0 and filtered_materials.high_quality != filtered_materials.total:
		failures.append("runtime 3D materials must preserve mipmapped anisotropic filtering")
	if session == null or session.active_vehicle == null or session.active_track == null or viewport.get_camera_3d() == null:
		failures.append("RaceSession composition missing")
	elif session.active_track != null:
		var lighting_stats := {"geometry": 0, "shadow_off": 0, "full_bright": 0, "shadow_receiver": 0}
		_scan_environment_lighting(session.active_track, lighting_stats)
		if lighting_stats.geometry > 0 and lighting_stats.shadow_off != lighting_stats.geometry:
			failures.append("non-vehicle geometry must not cast shadows")
		if lighting_stats.full_bright == 0:
			failures.append("non-vehicle materials must use max full bright")
		if lighting_stats.shadow_receiver == 0:
			failures.append("track ground must remain a shadow receiver")
		var vehicle_shadow_casters := {"count": 0}
		_scan_vehicle_shadow_casters(session.active_vehicle, vehicle_shadow_casters)
		if vehicle_shadow_casters.count == 0:
			failures.append("vehicle shadow caster missing")
		var visual_stats := {"geometry": 0, "lod_125": 0, "alpha_scissor": 0, "alpha_to_coverage": 0}
		_scan_visual_quality(session.active_track.get_node_or_null("GeneratedTrack"), visual_stats)
		_scan_visual_quality(session.active_track.get_node_or_null("GeneratedVegetation"), visual_stats)
		if visual_stats.geometry == 0 or visual_stats.lod_125 != visual_stats.geometry:
			failures.append("canonical track LOD bias missing")
		if visual_stats.alpha_scissor == 0 or visual_stats.alpha_to_coverage != visual_stats.alpha_scissor:
			failures.append("vegetation alpha-to-coverage missing")
	if hud == null or hud.get("_vehicle") != session.active_vehicle or hud.get("_aids") != session.driving_aids:
		failures.append("root HUD direct binding missing")
	if session != null and session.get_node_or_null("DebugHud") != null:
		failures.append("HUD leaked into 3D session")
	compositor.queue_free()
	if failures.is_empty():
		print("[PASS] WorldHudCompositor presents RaceSession and binds HUD outside the 3D viewport.")
	else:
		for failure in failures:
			printerr("[FAIL] " + failure)
	quit(failures.size())


func _scan_visual_quality(node: Node, stats: Dictionary) -> void:
	if node == null:
		return
	if node is GeometryInstance3D:
		stats.geometry += 1
		if is_equal_approx((node as GeometryInstance3D).lod_bias, 1.25):
			stats.lod_125 += 1
	if node is MeshInstance3D:
		var mesh_instance := node as MeshInstance3D
		if mesh_instance.mesh != null:
			for surface in mesh_instance.mesh.get_surface_count():
				var material := mesh_instance.get_active_material(surface)
				if material is StandardMaterial3D and material.transparency == BaseMaterial3D.TRANSPARENCY_ALPHA_SCISSOR:
					stats.alpha_scissor += 1
					if material.alpha_antialiasing_mode == BaseMaterial3D.ALPHA_ANTIALIASING_ALPHA_TO_COVERAGE:
						stats.alpha_to_coverage += 1
	for child in node.get_children():
		_scan_visual_quality(child, stats)


func _scan_texture_filters(node: Node, stats: Dictionary) -> void:
	if node == null:
		return
	if node is MeshInstance3D:
		var mesh_instance := node as MeshInstance3D
		if mesh_instance.mesh != null:
			for surface in range(mesh_instance.mesh.get_surface_count()):
				var material := mesh_instance.get_active_material(surface)
				if material is StandardMaterial3D:
					stats.total += 1
					if material.texture_filter == BaseMaterial3D.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS_ANISOTROPIC:
						stats.high_quality += 1
	for child in node.get_children():
		_scan_texture_filters(child, stats)


func _scan_environment_lighting(node: Node, stats: Dictionary) -> void:
	if node == null:
		return
	if node is GeometryInstance3D:
		stats.geometry += 1
		if (node as GeometryInstance3D).cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_OFF:
			stats.shadow_off += 1
	if node is MeshInstance3D and (node as MeshInstance3D).mesh != null:
		var mesh_instance := node as MeshInstance3D
		for surface in range(mesh_instance.mesh.get_surface_count()):
			var material := mesh_instance.get_active_material(surface)
			if material is BaseMaterial3D:
				if _is_shadow_receiver(mesh_instance):
					if (material as BaseMaterial3D).shading_mode != BaseMaterial3D.SHADING_MODE_UNSHADED:
						stats.shadow_receiver += 1
				elif (material as BaseMaterial3D).shading_mode == BaseMaterial3D.SHADING_MODE_UNSHADED:
					stats.full_bright += 1
	for child in node.get_children():
		_scan_environment_lighting(child, stats)


func _is_shadow_receiver(node: Node) -> bool:
	var lower := node.name.to_lower()
	return lower.begins_with("trak") or "road" in lower or "asphalt" in lower or "ground" in lower


func _scan_vehicle_shadow_casters(node: Node, stats: Dictionary) -> void:
	if node == null:
		return
	if node is GeometryInstance3D and (node as GeometryInstance3D).cast_shadow != GeometryInstance3D.SHADOW_CASTING_SETTING_OFF:
		stats.count += 1
	for child in node.get_children():
		_scan_vehicle_shadow_casters(child, stats)
