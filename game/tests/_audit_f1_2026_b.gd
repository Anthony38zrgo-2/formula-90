extends SceneTree

const GLBS := [
	"res://assets/models/vehicles/f1_2026_b/f1_2026_b_body.glb",
	"res://assets/models/vehicles/f1_2026_b/wheel_fl.glb",
	"res://assets/models/vehicles/f1_2026_b/wheel_fr.glb",
	"res://assets/models/vehicles/f1_2026_b/wheel_rl.glb",
	"res://assets/models/vehicles/f1_2026_b/wheel_rr.glb",
]


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var failures := 0
	for path in GLBS:
		var packed := load(path) as PackedScene
		if packed == null:
			printerr("[FAIL] no load: %s" % path)
			failures += 1
			continue
		var inst := packed.instantiate()
		if inst == null:
			printerr("[FAIL] no instantiate: %s" % path)
			failures += 1
			continue
		root.add_child(inst)
		await process_frame
		print("=== %s ===" % path)
		_dump(inst, 0)
		var aabb := _collect_aabb(inst)
		if aabb != null:
			print("AABB min=%s max=%s size=%s" % [aabb.position, aabb.end, aabb.size])
		inst.queue_free()
		await process_frame
	if failures == 0:
		quit(0)
	else:
		quit(1)


func _collect_aabb(node: Node) -> AABB:
	var result: AABB
	var first := true
	for child in node.find_children("*", "MeshInstance3D", true, false):
		var mi := child as MeshInstance3D
		var mesh := mi.mesh
		if mesh == null:
			continue
		for surface in mesh.get_surface_count():
			var arrays := mesh.surface_get_arrays(surface)
			var verts := arrays[Mesh.ARRAY_VERTEX] as PackedVector3Array
			if verts == null or verts.is_empty():
				continue
			for v in verts:
				var wv := mi.global_transform * v
				if first:
					result = AABB(wv, Vector3.ZERO)
					first = false
				else:
					result = result.expand(wv)
	return null if first else result


func _dump(node: Node, depth: int) -> void:
	var indent := "  ".repeat(depth)
	var t := ""
	if node is Node3D:
		var n3 := node as Node3D
		t = " transform=%s" % n3.transform
	print("%s%s%s" % [indent, node.name, t])
	for child in node.get_children():
		_dump(child, depth + 1)
