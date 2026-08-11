extends SceneTree

const DEFAULT_RESOURCE := "res://assets/generated/tracks/la_chutana/la_chutana.glb"


func _init() -> void:
	var arguments := OS.get_cmdline_user_args()
	var resource_path := arguments[0] if not arguments.is_empty() else DEFAULT_RESOURCE
	print("PROBE_PHASE load ", resource_path)
	var packed := load(resource_path) as PackedScene
	if packed == null:
		push_error("Unable to load %s" % resource_path)
		quit(2)
		return
	print("PROBE_PHASE instantiate ", resource_path)
	var root := packed.instantiate()
	var counts := {
		"static_bodies": 0,
		"collision_shapes": 0,
		"null_shapes": 0,
		"mesh_instances": 0,
	}
	_scan(root, counts)
	print("PROBE_RESULT ", resource_path, " ", JSON.stringify(counts))
	root.free()
	quit(0)


func _scan(node: Node, counts: Dictionary) -> void:
	if node is StaticBody3D:
		counts.static_bodies += 1
	elif node is CollisionShape3D:
		counts.collision_shapes += 1
		if (node as CollisionShape3D).shape == null:
			counts.null_shapes += 1
	elif node is MeshInstance3D:
		counts.mesh_instances += 1
	for child in node.get_children():
		_scan(child, counts)
