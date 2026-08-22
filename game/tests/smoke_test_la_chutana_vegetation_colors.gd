extends SceneTree


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var packed := load("res://scenes/tracks/test_field/la_chutana_generated.tscn") as PackedScene
	if packed == null:
		printerr("[FAIL] La Chutana generated scene could not load")
		quit(1)
		return
	var track := packed.instantiate()
	root.add_child(track)
	await process_frame
	await process_frame
	var vegetation := track.get_node_or_null("GeneratedVegetation")
	var colored := 0
	var pigment_enabled := 0
	for node in vegetation.find_children("*", "MeshInstance3D", true, false):
		var mesh_instance := node as MeshInstance3D
		for surface in range(mesh_instance.mesh.get_surface_count()):
			var colors = mesh_instance.mesh.surface_get_arrays(surface)[Mesh.ARRAY_COLOR]
			if not (colors is PackedColorArray) or colors.is_empty():
				continue
			colored += 1
			var material := mesh_instance.get_active_material(surface)
			if material is StandardMaterial3D and (material as StandardMaterial3D).vertex_color_use_as_albedo:
				pigment_enabled += 1
			elif material is ShaderMaterial and bool((material as ShaderMaterial).get_shader_parameter("use_vertex_colors_in_albedo")):
				pigment_enabled += 1
	if colored == 0 or pigment_enabled != colored:
		printerr("[FAIL] vegetation pigmentation binding %d/%d" % [pigment_enabled, colored])
		track.free()
		quit(1)
		return
	print("[PASS] vegetation pigmentation binding %d/%d" % [pigment_enabled, colored])
	track.free()
	quit(0)
