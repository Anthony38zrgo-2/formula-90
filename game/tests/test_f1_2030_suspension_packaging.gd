extends SceneTree

## Visual-only regression; run in an isolated project without native DLLs.
## Checks rendered volumes, rigid rod closure and continuity through travel.
const CONFIG := "res://data/vehicles/f1_2030/f1_2030_v10_geometric.json"
var failures: Array[String] = []

func check(ok: bool, message: String) -> void:
	if not ok and not failures.has(message):
		failures.append(message)
		printerr(message)

func _init() -> void:
	call_deferred("run")

func run() -> void:
	var geometry := SuspensionGeometry.from_json_path(CONFIG)
	var links := SuspensionLinkVisual.new()
	root.add_child(links)
	links.setup(geometry)
	var dump: Array = []
	var minimum_y := INF
	var maximum_y := -INF
	for wheel in range(2):
		var corner := geometry.get_corner(wheel)
		var rest: float = corner["spring_len"] * corner["resting_ratio"]
		var rest_pose := links.visual_pose(wheel, geometry.solve(wheel, rest, 0.0, 0.0, 0.0))
		var length: float = rest_pose["pushrod"][0].distance_to(rest_pose["pushrod"][1])
		for steer in [-0.35, 0.0, 0.35]:
			var previous := Vector3.ZERO
			for sample in range(41):
				var travel := lerpf(corner["travel_min"], corner["travel_max"], sample / 40.0)
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
						# The outboard blade intentionally leaves the side of the nose.
						if path == "Authored_pushrod" and absf(point.x) > 0.145:
							continue
						minimum_y = minf(minimum_y, point.y)
						maximum_y = maxf(maximum_y, point.y)
						# Conservative volume inside the GLB nose, also verified by
						# vertical triangle rays on the exported vertex dump in Blender.
						check(point.y < 0.235 and point.y > -0.08 and absf(point.x) < 0.15, "Rendered inboard component escaped nose clearance")
						if steer == 0.0 and sample % 2 == 0:
							dump.append([point.x, point.y, point.z])
	var args := OS.get_cmdline_user_args()
	if not args.is_empty():
		var file := FileAccess.open(args[0], FileAccess.WRITE)
		file.store_string(JSON.stringify(dump))
	print("[RESULT] Rendered packaging: %d failure(s); 246 poses; height [%.5f, %.5f]" % [failures.size(), minimum_y, maximum_y])
	links.free()
	quit(0 if failures.is_empty() else 1)
