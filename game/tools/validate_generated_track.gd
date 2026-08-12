extends SceneTree

## Headless Godot-load validation for generated F90 track GLB pairs.
##
## Loads the environment (physics/collision) and vegetation (collision-free)
## GLBs produced by the normalized Blender track pipeline through
## GLTFDocument, instantiates both in a self-contained validation scene and
## asserts the runtime split contract:
##
## * both GLBs parse and load without error;
## * the environment GLB owns the expected collision proxies and visual meshes
##   (GrassTerrainCollision/RoadCollision/GrassSafetyFloor/GrassEdgeCollision
##   Left+Right, plus at least one Barrier_*-colonly simplified proxy);
## * every collision proxy node is collidable (StaticBody3D or a mesh);
## * the vegetation GLB owns collision-free asset roots only;
## * no collision proxy leaks into the vegetation GLB;
## * scene trees are connected (no orphan/empty roots) with non-empty meshes.
##
## This script never writes to the runtime game assets and never touches the
## real Godot runtime: it only reads the two GLB files passed on the command
## line and prints a PASS/FAIL report for each invariant.
##
## Usage (Windows console build):
##
##     Godot_v4.7.1-stable_win64_console.exe --headless --path <project> \
##         --script res://tools/validate_generated_track.gd -- \
##         --env-glb <env.glb> --veg-glb <veg.glb> [--track-id <id>]
##
## Exit codes: 0 = PASS, 1 = FAIL, 2 = usage/argument error.

const COLLISION_SUFFIX := "-colonly"

const REQUIRED_ENVIRONMENT_COLLISION: Array[String] = [
	"GrassTerrainCollision-colonly",
	"RoadCollision-colonly",
	"GrassSafetyFloor-colonly",
	"GrassEdgeCollisionLeft-colonly",
	"GrassEdgeCollisionRight-colonly",
]

const REQUIRED_ENVIRONMENT_VISUALS: Array[String] = [
	"GrassTerrainVisual",
	"RoadVisual",
]

var _failures := 0


func _init() -> void:
	var args := _parse_args(OS.get_cmdline_user_args())
	if args.is_empty():
		_print_usage()
		quit(2)
		return
	var report: Array[String] = _validate(args)
	for line in report:
		print(line)
	if _failures == 0:
		print("RESULT: PASS")
		quit(0)
		return
	print("RESULT: FAIL (%d invariant(s) failed)" % _failures)
	quit(1)


func _parse_args(raw: PackedStringArray) -> Dictionary:
	var env_glb := ""
	var veg_glb := ""
	var track_id := ""
	var index := 0
	while index < raw.size():
		var arg := raw[index]
		match arg:
			"--env-glb":
				if index + 1 < raw.size():
					env_glb = raw[index + 1]
					index += 1
			"--veg-glb":
				if index + 1 < raw.size():
					veg_glb = raw[index + 1]
					index += 1
			"--track-id":
				if index + 1 < raw.size():
					track_id = raw[index + 1]
					index += 1
		index += 1
	if env_glb.is_empty() or veg_glb.is_empty():
		return {}
	return {"env_glb": env_glb, "veg_glb": veg_glb, "track_id": track_id}


func _validate(args: Dictionary) -> Array[String]:
	var report: Array[String] = []
	report.append("GODOT-LOAD VALIDATION")
	var track_id := str(args["track_id"])
	report.append("track_id: %s" % (track_id if not track_id.is_empty() else "n/a"))
	report.append("env_glb: %s" % str(args["env_glb"]))
	report.append("veg_glb: %s" % str(args["veg_glb"]))

	var env_loaded := _load_scene(str(args["env_glb"]))
	var env_root = env_loaded[0] as Node
	if env_root == null:
		report.append("[FAIL] environment GLB failed to load: error=%d" % int(env_loaded[1]))
		_failures += 1
	else:
		report.append("[PASS] environment GLB loads")
		_report_environment(report, env_root)
		env_root.free()

	var veg_loaded := _load_scene(str(args["veg_glb"]))
	var veg_root = veg_loaded[0] as Node
	if veg_root == null:
		report.append("[FAIL] vegetation GLB failed to load: error=%d" % int(veg_loaded[1]))
		_failures += 1
	else:
		report.append("[PASS] vegetation GLB loads")
		_report_vegetation(report, veg_root)
		veg_root.free()

	return report


func _load_scene(path: String) -> Array:
	var gltf := GLTFDocument.new()
	var state := GLTFState.new()
	var err := gltf.append_from_file(path, state)
	if err != OK:
		return [null, err]
	return [gltf.generate_scene(state), OK]


func _report_environment(report: Array[String], env_root: Node) -> void:
	var nodes := _all_nodes(env_root)
	var mesh_count := _count_meshes(nodes)
	report.append("[INFO] environment scene nodes: %d, meshes: %d" % [nodes.size(), mesh_count])
	if env_root.get_child_count() < 1:
		report.append("[FAIL] environment scene has no child nodes (empty/orphan root)")
		_failures += 1
	else:
		report.append("[PASS] environment scene tree is connected and non-empty")

	for required in REQUIRED_ENVIRONMENT_COLLISION:
		_check_collision_node(report, env_root, required)
	for required in REQUIRED_ENVIRONMENT_VISUALS:
		_check_visual_node(report, env_root, required)

	var barrier_visuals := 0
	var barrier_collisions := 0
	for node in nodes:
		var lower := node.name.to_lower()
		if lower.begins_with("barrier_"):
			if lower.ends_with(COLLISION_SUFFIX):
				barrier_collisions += 1
			else:
				barrier_visuals += 1
	if barrier_collisions < 1:
		report.append("[FAIL] environment GLB contains no Barrier_*-colonly collision proxy")
		_failures += 1
	else:
		report.append("[PASS] environment GLB contains %d Barrier_*-colonly collision proxy(ies)" % barrier_collisions)
	if barrier_visuals < 1:
		report.append("[WARN] environment GLB contains no barrier visuals (track has no guardrails)")


func _check_collision_node(report: Array[String], root: Node, name: String) -> void:
	var node := _find_node(root, name)
	if node == null:
		report.append("[FAIL] collision proxy missing: %s" % name)
		_failures += 1
		return
	if _is_collidable(node):
		report.append("[PASS] collision proxy present and collidable: %s (%s)" % [name, node.get_class()])
	else:
		report.append("[FAIL] collision proxy node is not collidable: %s (%s)" % [name, node.get_class()])
		_failures += 1


func _check_visual_node(report: Array[String], root: Node, name: String) -> void:
	var node := _find_node(root, name)
	if node == null:
		report.append("[FAIL] visual node missing: %s" % name)
		_failures += 1
		return
	if _is_mesh(node):
		report.append("[PASS] visual node present with mesh: %s" % name)
	else:
		report.append("[FAIL] visual node has no mesh: %s" % name)
		_failures += 1


func _report_vegetation(report: Array[String], veg_root: Node) -> void:
	var nodes := _all_nodes(veg_root)
	var mesh_count := _count_meshes(nodes)
	var asset_count := 0
	var leaks: Array[String] = []
	for node in nodes:
		var lower := node.name.to_lower()
		if "colonly" in lower or "collision" in lower or node is CollisionObject3D:
			leaks.append(node.name)
		if node.name.begins_with("Asset_") and _is_mesh(node):
			asset_count += 1
	report.append("[INFO] vegetation scene nodes: %d, meshes: %d" % [nodes.size(), mesh_count])
	if veg_root.get_child_count() < 1:
		report.append("[FAIL] vegetation scene has no child nodes (empty/orphan root)")
		_failures += 1
	if asset_count < 1:
		report.append("[FAIL] vegetation GLB contains no collision-free asset roots")
		_failures += 1
	else:
		report.append("[PASS] vegetation GLB contains %d collision-free asset root(s)" % asset_count)
	if leaks.is_empty():
		report.append("[PASS] vegetation GLB contains no collision proxies")
	else:
		for leak in leaks:
			report.append("[FAIL] vegetation GLB leaks collision proxy: %s" % leak)
			_failures += 1


func _all_nodes(node: Node) -> Array[Node]:
	var out: Array[Node] = [node]
	for child in node.get_children():
		out.append_array(_all_nodes(child))
	return out


func _find_node(root: Node, name: String) -> Node:
	for node in _all_nodes(root):
		if node.name == name:
			return node
	return null


func _is_collidable(node: Node) -> bool:
	if node is CollisionObject3D:
		return true
	if node is MeshInstance3D:
		var mesh_instance := node as MeshInstance3D
		return mesh_instance.mesh != null and mesh_instance.mesh.get_surface_count() > 0
	return false


func _is_mesh(node: Node) -> bool:
	if node is MeshInstance3D:
		var mesh_instance := node as MeshInstance3D
		return mesh_instance.mesh != null and mesh_instance.mesh.get_surface_count() > 0
	return false


func _count_meshes(nodes: Array[Node]) -> int:
	var count := 0
	for node in nodes:
		if _is_mesh(node):
			count += 1
	return count


func _print_usage() -> void:
	print("usage: --env-glb <path> --veg-glb <path> [--track-id <id>]")
