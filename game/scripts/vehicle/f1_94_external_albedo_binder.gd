class_name F194ExternalAlbedoBinder
extends Node

@export var visual_root: Node
@export_dir var albedo_directory := "res://assets/models/vehicles/f1_94/decoupled/textures/albedo"

var failures: Array[String] = []

func _ready() -> void:
	var root := visual_root if visual_root != null else get_parent()
	_bind_recursive(root)
	if not failures.is_empty():
		push_error("F1-94 external materials incomplete: %s" % "; ".join(failures))

func _bind_recursive(node: Node) -> void:
	if node is MeshInstance3D and String(node.name).begins_with("GEO_"):
		_bind_mesh(node as MeshInstance3D)
	for child in node.get_children():
		_bind_recursive(child)

func _bind_mesh(mesh_instance: MeshInstance3D) -> void:
	var texture_path := "%s/%s.png" % [albedo_directory, mesh_instance.name]
	var texture := load(texture_path) as Texture2D
	if texture == null:
		failures.append("%s -> %s" % [mesh_instance.get_path(), texture_path])
		return
	var material := StandardMaterial3D.new()
	material.albedo_texture = texture
	material.texture_filter = BaseMaterial3D.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS_ANISOTROPIC
	material.metallic = 0.0
	material.roughness = 1.0
	material.cull_mode = BaseMaterial3D.CULL_BACK
	for surface_index in range(mesh_instance.get_surface_override_material_count()):
		mesh_instance.set_surface_override_material(surface_index, material)
	if mesh_instance.mesh != null and mesh_instance.get_surface_override_material_count() == 0:
		for surface_index in range(mesh_instance.mesh.get_surface_count()):
			mesh_instance.set_surface_override_material(surface_index, material)
