extends SceneTree

const VISUAL := "res://scenes/vehicles/f1_2026_b/f1_2026_b_visual.tscn"
const CAR := "res://scenes/vehicles/f1_2026_b_car.tscn"


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	for scene_path in [VISUAL, CAR]:
		print("################ %s ################" % scene_path)
		var packed := load(scene_path) as PackedScene
		if packed == null:
			printerr("[FAIL] no load %s" % scene_path)
			quit(1)
			return
		var inst := packed.instantiate()
		root.add_child(inst)
		await process_frame
		_dump(inst, 0)
		inst.queue_free()
		await process_frame
	print("DONE")
	quit(0)


func _dump(node: Node, depth: int) -> void:
	var indent := "  ".repeat(depth)
	if node is Node3D:
		var n3 := node as Node3D
		var global := n3.global_transform
		if node.get_child_count() == 0 or node is MeshInstance3D:
			print("%s%s  global_origin=%s  basis_axes X=%s Y=%s Z=%s" % [
				indent, node.name,
				Vector3(global.origin.x, global.origin.y, global.origin.z),
				Vector3(global.basis.x.x, global.basis.x.y, global.basis.x.z),
				Vector3(global.basis.y.x, global.basis.y.y, global.basis.y.z),
				Vector3(global.basis.z.x, global.basis.z.y, global.basis.z.z),
			])
		elif depth < 3:
			print("%s%s" % [indent, node.name])
	for child in node.get_children():
		_dump(child, depth + 1)
