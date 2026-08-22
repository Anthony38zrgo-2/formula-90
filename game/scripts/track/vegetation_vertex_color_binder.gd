extends Node3D

## Restores glTF COLOR_0 pigmentation on generated semantic vegetation.
## Godot keeps the vertex-color arrays but imports the generated untextured
## materials with vertex_color_use_as_albedo disabled.


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
		if not (colors is PackedColorArray) or colors.is_empty():
			continue
		var source := mesh_instance.get_active_material(surface)
		if source is StandardMaterial3D:
			var material := (source as StandardMaterial3D).duplicate() as StandardMaterial3D
			material.vertex_color_use_as_albedo = true
			material.vertex_color_is_srgb = true
			mesh_instance.set_surface_override_material(surface, material)
