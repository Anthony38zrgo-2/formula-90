extends Node3D

## Restores glTF COLOR_0 pigmentation on generated semantic vegetation and
## enables alpha-to-coverage on imported alpha-scissor foliage materials.

var _material_cache: Dictionary = {}


func _enter_tree() -> void:
	_bind_vertex_colors(self)


func _bind_vertex_colors(root: Node) -> void:
	for child in root.get_children():
		if child is MeshInstance3D:
			_bind_mesh(child as MeshInstance3D)
		_bind_vertex_colors(child)


func _bind_mesh(mesh_instance: MeshInstance3D) -> void:
	if mesh_instance.mesh == null:
		return
	for surface in range(mesh_instance.mesh.get_surface_count()):
		var arrays := mesh_instance.mesh.surface_get_arrays(surface)
		var colors = arrays[Mesh.ARRAY_COLOR]
		var source := mesh_instance.get_active_material(surface)
		if source is StandardMaterial3D:
			var has_vertex_colors: bool = colors is PackedColorArray and not colors.is_empty()
			var cache_key := "%d:%s" % [source.get_instance_id(), has_vertex_colors]
			if _material_cache.has(cache_key):
				mesh_instance.set_surface_override_material(surface, _material_cache[cache_key])
				continue
			var material := (source as StandardMaterial3D).duplicate() as StandardMaterial3D
			var changed := false
			if has_vertex_colors:
				material.vertex_color_use_as_albedo = true
				material.vertex_color_is_srgb = true
				changed = true
			var uses_alpha_cutout := material.albedo_texture != null and material.transparency != BaseMaterial3D.TRANSPARENCY_DISABLED
			if uses_alpha_cutout:
				material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA_SCISSOR
				material.alpha_antialiasing_mode = BaseMaterial3D.ALPHA_ANTIALIASING_ALPHA_TO_COVERAGE
				material.alpha_antialiasing_edge = material.alpha_scissor_threshold
				changed = true
			if changed:
				_material_cache[cache_key] = material
				mesh_instance.set_surface_override_material(surface, material)
