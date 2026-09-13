extends SceneTree

## Native-free regression: rigid attachment closure and the actual visual owner.
const CONFIG := "res://data/vehicles/f1_2030/f1_2030_v10_physics.json"
var failures: Array[String] = []

func _init() -> void:
	call_deferred("run")

func check(ok: bool, message: String) -> void:
	if not ok:
		failures.append(message)
		printerr(message)

func rigid(basis: Basis) -> bool:
	return absf(basis.determinant() - 1.0) < 0.00001 and (basis.transposed() * basis).is_equal_approx(Basis.IDENTITY)

func run() -> void:
	var geometry := SuspensionGeometry.from_json_path(CONFIG)
	var maximum := {"pushrod": 0.0, "trackrod": 0.0, "hub": 0.0, "tri_lu": 0.0}
	var clamped_rockers := 0
	var clamped_steering := 0
	for wheel in range(4):
		var c := geometry.get_corner(wheel)
		var rest: float = c["spring_len"] * c["resting_ratio"]
		print("Wheel ", wheel, " travel interval (m): ", c["travel_min"], " .. ", c["travel_max"])
		var rest_pose := geometry.solve(wheel, rest, 0.0, 0.0, 0.0)
		for role in ["upper", "lower", "pushrod", "trackrod"]:
			var pose: Transform3D = rest_pose[role + "_pose"]
			check(pose.origin.length() < 0.00001 and pose.basis.is_equal_approx(Basis.IDENTITY), "Authored component moved from Blender rest pose")
		check(rest_pose["pushrod"][0].distance_to(c["pushrod_outer_rest"]) < 0.00001, "Authored pushrod attachment moved at rest")
		check(rest_pose["pushrod"][1].distance_to(c["rocker_arm_rest"]) < 0.00001, "Authored rocker attachment moved at rest")
		for requested in [0.0, float(c["spring_len"])]:
			var limited := geometry.solve(wheel, requested, 0.0, 0.0, 0.0)
			check(not limited["rocker_clamped"] and not limited["steering_clamped"], "Travel stop leaves an unreachable linkage")
			check(limited["solved_compression"] >= c["travel_min"] and limited["solved_compression"] <= c["travel_max"], "Visual travel escapes mechanical stops")
		for step in range(25):
			var compression := rest - 0.02 + step * 0.005
			for degrees in [-25.0, 0.0, 25.0]:
				var pose := geometry.solve(wheel, compression, deg_to_rad(degrees) if wheel < 2 else 0.0, -0.05, 0.7)
				check(not pose["travel_limited"], "Operating travel was clamped")
				check(rigid(pose["upright_basis"]) and rigid(pose["wheel_basis"]) and rigid(pose["rocker"]["basis"]), "Non-rigid component orientation")
				var upright := Transform3D(pose["upright_basis"], pose["hub"])
				check((upright * (c["lbj_rest"] - c["hub_center"])).distance_to(pose["lower"][2]) < 0.001, "Lower upright joint detached")
				check((upright * (c["ubj_rest"] - c["hub_center"])).distance_to(pose["upper"][2]) < 0.001, "Upper upright joint detached")
				check((upright * (c["trackrod_outer"] - c["hub_center"])).distance_to(pose["trackrod"][1]) < 0.001, "Steering arm detached")
				var damper_rest: float = c["damper_chassis"].distance_to(c["damper_arm_rest"])
				var damper_length: float = pose["damper"][0].distance_to(pose["damper"][1])
				check(damper_length > damper_rest * 0.65 and damper_length < damper_rest * 1.25, "Damper piston exits its housing")
				var rocker := Transform3D(pose["rocker"]["basis"], c["rocker_pivot"])
				check((rocker * (c["rocker_arm_rest"] - c["rocker_pivot"])).distance_to(pose["pushrod"][1]) < 0.00001, "Rocker pivot or pushrod detached")
				check((rocker * (c["damper_arm_rest"] - c["rocker_pivot"])).distance_to(pose["damper"][1]) < 0.00001, "Rocker damper detached")
				for key in maximum:
					maximum[key] = maxf(maximum[key], pose["residuals"][key])
				clamped_rockers += int(pose["rocker_clamped"])
				clamped_steering += int(pose["steering_clamped"])
	check(maximum["pushrod"] < 0.001, "Pushrod length changes inside operating envelope")
	check(maximum["trackrod"] < 0.001, "Trackrod length changes inside operating envelope")
	print("Maximum residuals (m): ", maximum, "; limited rocker/steering poses: ", clamped_rockers, "/", clamped_steering)

	# Exercise the actual controller and builder, with deterministic input instead
	# of native DLLs or global car motion accidentally satisfying the assertions.
	var car := Node3D.new()
	root.add_child(car)
	var chassis := MeshInstance3D.new()
	chassis.name = "ChassisVisual"
	var body := BoxMesh.new()
	body.size = Vector3(0.6, 0.7, 4.0)
	chassis.mesh = body
	car.add_child(chassis)
	var controller := F1WheelVisualController.new()
	controller.vehicle = car
	controller.physics_config_path = CONFIG
	var names := ["FrontLeftWheel", "FrontRightWheel", "RearLeftWheel", "RearRightWheel"]
	for i in range(4):
		var hub := Node3D.new()
		hub.name = names[i]
		hub.position = geometry.get_corner(i)["hub_center"]
		car.add_child(hub)
		var parent := hub
		for label in ["SteerPivot", "CamberPivot", "Spinner"]:
			var pivot := Node3D.new()
			pivot.name = label
			parent.add_child(pivot)
			parent = pivot
	car.add_child(controller)
	controller.set_physics_process(false)
	controller.set_process(false)
	await process_frame
	for parts in controller._suspension_links._source_parts:
		check(parts.has("upper") and parts.has("lower"), "Original wishbone profiles missing")
		check(parts["upper"].mesh.get_aabb().size.x > 0.35, "Wishbone shortened to the old outboard chassis anchors")
	controller._has_physics_sample = true
	for frame in range(12):
		for i in range(4):
			var c := geometry.get_corner(i)
			controller._compression_m[i] = float(c["spring_len"]) * float(c["resting_ratio"]) + (0.06 if frame > 0 else 0.0)
			controller._target_steer[i] = (0.3 if frame > 0 and i < 2 else 0.0) + controller._toe(i) * (1.0 if i % 2 == 0 else -1.0)
		controller._process(1.0 / 60.0)
		for i in range(4):
			var pose: Dictionary = controller._suspension_solved[i]
			var hub: Node3D = controller._hubs[i]
			check(hub.position.distance_to(pose["hub"]) < 0.000001, "Wheel lags behind suspension")
			check(hub.get_node("SteerPivot").basis.is_equal_approx(pose["wheel_basis"]), "Wheel orientation differs from bearing pose")
			var links := car.get_node("SuspensionLinkVisual/Susp_%s" % ["FL", "FR", "RL", "RR"][i])
			check(links.get_node("UPRIGHT").position.distance_to(hub.position) < 0.000001, "Rendered upright detached from wheel")
			var rod: MeshInstance3D = links.get_node("TRACKROD")
			check((rod.transform * Vector3(0, -0.5, 0)).distance_to(pose["trackrod"][0]) < 0.00001, "Rendered rod start detached")
			check((rod.transform * Vector3(0, 0.5, 0)).distance_to(pose["trackrod"][1]) < 0.00001, "Rendered rod end detached")
	var rack: MeshInstance3D = car.get_node("SuspensionLinkVisual/SteeringRack")
	check((rack.transform * Vector3(0, -0.5, 0)).distance_to(controller._suspension_solved[0]["trackrod"][0]) < 0.00001, "Steering rack detached from left tie rod")
	check((rack.transform * Vector3(0, 0.5, 0)).distance_to(controller._suspension_solved[1]["trackrod"][0]) < 0.00001, "Steering rack detached from right tie rod")
	car.free()
	print("[RESULT] Suspension visual pose: %d failure(s)" % failures.size())
	quit(0 if failures.is_empty() else 1)
