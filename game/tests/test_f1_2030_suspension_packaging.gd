extends SceneTree

## Visual-only regression; run in an isolated project without native DLLs.
## Checks rendered volumes, rigid rod closure and continuity through travel.
const CONFIG := "res://data/vehicles/f1_2030/profiles/geometric/suspension_presentation.json"
const CHASSIS_MODEL := "res://assets/models/vehicles/f1-2030/f1_2030_v10_chassis.glb"
const PHYSICAL_CONFIGURATION := "res://data/vehicles/f1_2030/profiles/geometric/suspension.json"
var failures: Array[String] = []

func check(ok: bool, message: String) -> void:
	if not ok and not failures.has(message):
		failures.append(message)
		printerr(message)

func _init() -> void:
	call_deferred("run")

func run() -> void:
	var arguments := OS.get_cmdline_user_args()
	var chassis_path: String = arguments[1] if arguments.size() > 1 else CHASSIS_MODEL
	var collision_body := create_chassis_collision(chassis_path)
	if collision_body == null:
		quit(1)
		return
	await physics_frame
	await physics_frame
	var collision_space := root.get_world_3d().direct_space_state
	var physical_document: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(PHYSICAL_CONFIGURATION))
	var presentation_document: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(CONFIG))
	physical_document["suspension"]["geometry"] = presentation_document["suspension"]["geometry"]
	var geometry := SuspensionGeometry.from_json_dict(physical_document)
	var links := SuspensionLinkVisual.new()
	root.add_child(links)
	var use_physical_geometry := arguments.size() > 2 and arguments[2] == "physical"
	links.setup(geometry, use_physical_geometry)
	var dump: Array = []
	var minimum_y := INF
	var maximum_y := -INF
	for wheel in range(2):
		var corner := geometry.get_corner(wheel)
		var rest: float = corner["spring_len"] * corner["resting_ratio"]
		var physical_front: Dictionary = physical_document["suspension"]["geometry_physical"]["front"]
		var minimum_travel := maxf(corner["travel_min"], rest - float(physical_front["wheel_droop_m"]))
		var maximum_travel := minf(corner["travel_max"], rest + float(physical_front["wheel_bump_m"]))
		var rest_pose := links.visual_pose(wheel, geometry.solve(wheel, rest, 0.0, 0.0, 0.0))
		var length: float = rest_pose["pushrod"][0].distance_to(rest_pose["pushrod"][1])
		for steer in [-0.35, 0.0, 0.35]:
			var previous := Vector3.ZERO
			for sample in range(41):
				var travel := lerpf(minimum_travel, maximum_travel, sample / 40.0)
				var physical := geometry.solve(wheel, travel, steer, 0.0, 0.0)
				var pose := links.visual_pose(wheel, physical)
				check(not pose["rocker_clamped"], "Compact rocker cannot reach travel")
				check(absf(pose["pushrod"][0].distance_to(pose["pushrod"][1]) - length) < 0.00001, "Rendered pushrod stretched")
				check(pose["hub"] == physical["hub"] and pose["lower"] == physical["lower"] and pose["upper"] == physical["upper"], "Packaging changed wheel or wishbones")
				if sample > 0:
					check(previous.distance_to(pose["pushrod"][1]) < 0.025, "Compact rocker changed solution branch")
				previous = pose["pushrod"][1]
				links.update_wheel(wheel, physical)
				var assembly := links.get_node("Susp_" + SuspensionGeometry.WHEEL_KEYS[wheel])
				for path in ["ROCKER", "DAMPER/Body", "DAMPER/Piston", "Joint_09", "Joint_10", "Joint_11", "Joint_12", "Authored_pushrod"]:
					var mesh := assembly.get_node(path) as MeshInstance3D
					var vertices: PackedVector3Array = mesh.mesh.surface_get_arrays(0)[Mesh.ARRAY_VERTEX]
					for vertex in vertices:
						var point := mesh.global_transform * vertex
						if path == "Authored_pushrod" and point.distance_to(pose["pushrod"][1]) > 0.025:
							continue
						minimum_y = minf(minimum_y, point.y)
						maximum_y = maxf(maximum_y, point.y)
						var side_direction := -1.0 if point.x < 0.0 else 1.0
						var ray_query := PhysicsRayQueryParameters3D.create(Vector3(side_direction, point.y, point.z), Vector3(0.0, point.y, point.z), 1)
						var chassis_intersection := collision_space.intersect_ray(ray_query)
						check(not chassis_intersection.is_empty(), "Front component has no surrounding chassis surface: " + path)
						if not chassis_intersection.is_empty():
							check(absf(point.x) <= absf(chassis_intersection["position"].x) + 0.000001, "Front component protrudes through the current chassis: " + path)
						if steer == 0.0 and sample % 2 == 0:
							dump.append([point.x, point.y, point.z])
	var args := OS.get_cmdline_user_args()
	if not args.is_empty():
		var file := FileAccess.open(args[0], FileAccess.WRITE)
		file.store_string(JSON.stringify(dump))
	print("[RESULT] Rendered packaging: %d failure(s); 246 poses; height [%.5f, %.5f]" % [failures.size(), minimum_y, maximum_y])
	links.free()
	collision_body.free()
	quit(0 if failures.is_empty() else 1)

func create_chassis_collision(model_path: String) -> StaticBody3D:
	var document := GLTFDocument.new()
	var state := GLTFState.new()
	if document.append_from_file(model_path, state) != OK:
		check(false, "Cannot load the current chassis geometry")
		return null
	var chassis := document.generate_scene(state)
	root.add_child(chassis)
	var chassis_faces := PackedVector3Array()
	for candidate in chassis.find_children("*", "MeshInstance3D", true, false):
		var mesh_instance := candidate as MeshInstance3D
		if mesh_instance.name != "BodyworkNose" and mesh_instance.name != "BodyworkMonocoqueExterior":
			continue
		for vertex in mesh_instance.mesh.get_faces():
			chassis_faces.append(mesh_instance.global_transform * vertex)
	check(not chassis_faces.is_empty(), "Current chassis nose and monocoque surfaces are missing")
	var collision_body := StaticBody3D.new()
	var collision_shape := CollisionShape3D.new()
	var triangle_shape := ConcavePolygonShape3D.new()
	triangle_shape.backface_collision = true
	triangle_shape.set_faces(chassis_faces)
	collision_shape.shape = triangle_shape
	collision_body.add_child(collision_shape)
	root.add_child(collision_body)
	chassis.queue_free()
	return collision_body
