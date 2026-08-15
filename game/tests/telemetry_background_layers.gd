extends SceneTree
const SESSION_SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"
func _init(): call_deferred("_run")
func _run():
	print("=== TELEMETRY BG LAYERS ===")
	var packed := load(SESSION_SCENE_PATH) as PackedScene
	if packed==null:
		printerr("INCONCLUSIVE load fail"); quit(1); return
	var comp := packed.instantiate()
	root.add_child(comp)
	for i in 6: await process_frame
	var rs := comp.get_node_or_null("WorldViewport/RaceSession") as RaceSession
	if rs==null:
		printerr("INCONCLUSIVE no RaceSession"); comp.queue_free(); quit(1); return
	print("track:%s vehicle:%s" % [rs.active_track.name if rs.active_track else "null", rs.active_vehicle_root.name if rs.active_vehicle_root else "null"])
	var bg := rs.background_controller
	if bg==null:
		printerr("FAIL no BackgroundController"); comp.queue_free(); quit(1); return
	print("bg_controller global_pos=%s is_active=%s preset=%s layers=%d cam_valid=%s" % [str(bg.global_position), str(bg.is_active), str(bg.get_active_preset().id) if bg.get_active_preset() else "null", bg.get_layer_instances().size(), str(bg.get_debug_info()["camera_valid"])])
	for inst in bg.get_layer_instances():
		var tex_size = inst.texture.get_size() if inst.texture else Vector2.ZERO
		var aabb = inst.get_aabb()
		print(" layer %s: pos=%s gpos=%s px=%.2f scale=%s tex=%s vis=%s aabb=%s render_prio=%d" % [inst.name, str(inst.position), str(inst.global_position), inst.pixel_size, str(inst.scale), str(tex_size), str(inst.visible and inst.is_visible_in_tree()), str(aabb), inst.render_priority])
	var legacy := rs.active_track.get_node_or_null("SourceSkyboxRig")
	if legacy: print(" legacy SourceSkyboxRig vis=%s" % str(legacy.visible))
	var env := rs.active_track.get_node_or_null("WorldEnvironment") as WorldEnvironment
	if env: print(" WorldEnvironment env=%s bg_mode=%s sky=%s" % [str(env.environment!=null), str(env.environment.background_mode) if env.environment else "null", str(env.environment.sky!=null) if env.environment else "null"])
	var cam := rs.get_node_or_null("CameraRig/Camera3D") as Camera3D
	if cam: print(" camera gpos=%s fov=%.1f" % [str(cam.global_position), cam.fov])
	comp.queue_free()
	quit(0)
