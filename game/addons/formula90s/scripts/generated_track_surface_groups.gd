extends Node

## Tags StaticBody3D nodes imported from generated Blender GLBs so GEVP surface
## lookup continues to receive the same Road/Curb/Grass/Wall groups as authored
## Formula90s test scenes.

@export var generated_track_root: Node

func _ready() -> void:
	var root := generated_track_root if generated_track_root != null else get_parent()
	if root == null:
		push_warning("Generated track surface group tagger has no root.")
		return
	_tag_recursive(root)

func _tag_recursive(node: Node) -> void:
	if node is StaticBody3D:
		var lower := node.name.to_lower()
		if "road" in lower:
			node.add_to_group("Road")
		elif "curb" in lower:
			node.add_to_group("Curb")
		elif "grass" in lower:
			node.add_to_group("Grass")
		elif "guardrail" in lower:
			node.add_to_group("Wall")
	for child in node.get_children():
		_tag_recursive(child)
