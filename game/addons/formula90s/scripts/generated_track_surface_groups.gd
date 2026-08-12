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
	_apply_psx_native_shadow_budget(root)

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


func _apply_psx_native_shadow_budget(root: Node) -> void:
	# Preserve the strong, readable shadows from the car, road and containment
	# barrier. Small cards and distant scenery do not cast shadows on PSX-era
	# hardware, and disabling them avoids soft/expensive visual noise.
	_disable_shadow_recursive(root.get_node_or_null("GeneratedVegetation"))
	_disable_shadow_recursive(root.get_node_or_null("SourceSkyboxRig"))
	_disable_shadow_by_name(root, ["indexed_", "spectator", "marshal", "photographer", "flag", "sign"])


func _disable_shadow_recursive(node: Node) -> void:
	if node == null:
		return
	if node is GeometryInstance3D:
		node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		node.gi_mode = GeometryInstance3D.GI_MODE_DISABLED
	for child in node.get_children():
		_disable_shadow_recursive(child)


func _disable_shadow_by_name(node: Node, prefixes: Array[String]) -> void:
	if node is GeometryInstance3D:
		var lower := node.name.to_lower()
		for prefix in prefixes:
			if lower.contains(prefix):
				node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
				node.gi_mode = GeometryInstance3D.GI_MODE_DISABLED
				break
	for child in node.get_children():
		_disable_shadow_by_name(child, prefixes)
