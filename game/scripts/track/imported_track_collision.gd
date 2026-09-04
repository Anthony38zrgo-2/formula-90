extends Node

## Turns the converter's surface-aware collision GLB into tagged StaticBody3D
## nodes understood by the Formula90s wheel raycasts.

@export var collision_root: Node

const SURFACE_GROUPS := {
	"road": "Road",
	"curb": "Curb",
	"dirt": "Dirt",
	"grass": "Grass",
	"gravel": "Gravel",
	"sand": "Sand",
	"wall": "Wall",
	"metal": "Metal",
}

func _ready() -> void:
	var root := collision_root if collision_root != null else get_parent()
	_build_recursive(root)

func _build_recursive(node: Node) -> void:
	if node is MeshInstance3D:
		var mesh_instance := node as MeshInstance3D
		mesh_instance.visible = false
		mesh_instance.create_trimesh_collision()
		var surface := _surface_from_node(mesh_instance)
		for child in mesh_instance.get_children():
			if child is StaticBody3D:
				var body := child as StaticBody3D
				body.set_meta("surface_type", surface)
				body.add_to_group(SURFACE_GROUPS.get(surface, "Road"))
	for child in node.get_children():
		if not (node is MeshInstance3D and child is StaticBody3D):
			_build_recursive(child)

func _surface_from_node(node: Node) -> String:
	if node.has_meta("surface_type"):
		return String(node.get_meta("surface_type")).to_lower()
	var parts := String(node.name).to_lower().split("__")
	if parts.size() >= 2 and SURFACE_GROUPS.has(parts[1]):
		return parts[1]
	return "road"
