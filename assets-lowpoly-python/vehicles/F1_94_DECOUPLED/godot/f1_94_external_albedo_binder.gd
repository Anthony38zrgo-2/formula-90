extends Node
class_name F194ExternalAlbedoBinder

@export_dir var albedo_root: String = "res://assets/vehicles/F1_94/textures/albedo"

func _ready() -> void:
    apply_to(self)

func apply_to(root: Node) -> void:
    for node in root.find_children("GEO_*", "MeshInstance3D", true, false):
        _bind_mesh(node as MeshInstance3D)

func _bind_mesh(mesh_instance: MeshInstance3D) -> void:
    var texture_path := albedo_root.path_join(mesh_instance.name + ".png")
    if not ResourceLoader.exists(texture_path):
        push_warning("F1_94: missing external albedo for " + mesh_instance.name + ": " + texture_path)
        return

    var material := StandardMaterial3D.new()
    material.resource_name = "MAT_" + mesh_instance.name
    material.albedo_texture = load(texture_path)
    material.texture_filter = BaseMaterial3D.TEXTURE_FILTER_NEAREST
    material.metallic = 0.0
    material.roughness = 1.0
    material.cull_mode = BaseMaterial3D.CULL_BACK
    material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA_SCISSOR
    material.alpha_scissor_threshold = 0.5
    mesh_instance.material_override = material
