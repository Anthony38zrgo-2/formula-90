extends SceneTree

const BODY := "res://assets/models/vehicles/f1_2026_b/f1_2026_b_body.glb"
const WHEELS := [
	"res://assets/models/vehicles/f1_2026_b/wheel_fl.glb",
	"res://assets/models/vehicles/f1_2026_b/wheel_fr.glb",
	"res://assets/models/vehicles/f1_2026_b/wheel_rl.glb",
	"res://assets/models/vehicles/f1_2026_b/wheel_rr.glb",
]


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	# 1) Body rotated -90 about X: measure AABB (must be length->Z, height->Y)
	var body := (load(BODY) as PackedScene).instantiate() as Node3D
	root.add_child(body)
	await process_frame
	print("BODY identity AABB: %s" % _collect_aabb(body))
	body.rotate_object_local(Vector3.RIGHT, -PI / 2.0)
	await process_frame
	print("BODY rot -90x AABB: %s" % _collect_aabb(body))
	body.queue_free()
	await process_frame

	# 2) wheel radius per GLB: max radial extent about X axle
	for path in WHEELS:
		var w := (load(path) as PackedScene).instantiate() as Node3D
		root.add_child(w)
		await process_frame
		var mi := w.find_child("FL", true, false) as Node3D
		if mi == null:
			mi = w
		var r := _wheel_radius(mi)
		print("%s radius=%.5f" % [path, r])
		w.queue_free()
		await process_frame
	print("DONE")
	quit(0)


func _wheel_radius(node: Node3D) -> float:
	var maxr := 0.0
	for child in node.find_children("*", "MeshInstance3D", true, false):
		var mesh := (child as MeshInstance3D).mesh
		if mesh == null:
			continue
		for surface in mesh.get_surface_count():
			var arrays := mesh.surface_get_arrays(surface)
			var verts := arrays[Mesh.ARRAY_VERTEX] as PackedVector3Array
			for v in verts:
				var gv := (child as MeshInstance3D).global_transform * v
				var local := node.global_transform.affine_inverse() * gv
				var rad := sqrt(local.y * local.y + local.z * local.z)
				maxr = maxf(maxr, rad)
	return maxr


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
			for v in verts:
				var wv := mi.global_transform * v
				if first:
					result = AABB(wv, Vector3.ZERO)
					first = false
				else:
					result = result.expand(wv)
	return null if first else result
