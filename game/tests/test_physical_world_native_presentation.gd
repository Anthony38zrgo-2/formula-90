extends SceneTree

const ControllerScript = preload("res://scripts/runtime/physical_vehicle_world_controller.gd")
const VisualControllerScript = preload("res://scripts/vehicle/f1_wheel_visual_controller.gd")

var contract_failed: bool = false

func require_condition(condition: bool, message: String) -> void:
	if not condition:
		contract_failed = true
		push_error(message)

func _initialize() -> void:
	call_deferred("run_native_contract")

func run_native_contract() -> void:
	var arguments := OS.get_cmdline_user_args()
	if arguments.size() != 3:
		push_error("Expected repository root, physical package and vehicle profile paths")
		quit(1)
		return
	for class_identifier in ["F194RustVehicle", "F90Core", "PhysicalVehicleWorldInterface"]:
		if not ClassDB.class_exists(class_identifier):
			push_error("Missing native class: " + class_identifier)
			quit(1)
			return
	var session := Node3D.new()
	session.name = "NativeWorldSession"
	root.add_child(session)
	var controller := ControllerScript.new()
	controller.repository_root_path = arguments[0]
	controller.physical_package_path = arguments[1]
	for entity_index in range(2):
		var vehicle: Node3D = ClassDB.instantiate("F194RustVehicle")
		vehicle.name = "PhysicalVehicle" + str(entity_index)
		vehicle.physics_config_path = arguments[2]
		vehicle.position = Vector3(float(entity_index) * 5.0, 10.0, 0.0)
		session.add_child(vehicle)
		var presentation_core: Node = ClassDB.instantiate("F90Core")
		presentation_core.name = "PresentationCore" + str(entity_index)
		presentation_core.config_json_path = arguments[2]
		presentation_core.enable_audio = false
		presentation_core.target_vehicle_path = NodePath("../" + str(vehicle.name))
		session.add_child(presentation_core)
		controller.vehicle_paths.append(NodePath("../" + str(vehicle.name)))
		controller.presentation_core_paths.append(NodePath("../" + str(presentation_core.name)))
		controller.vehicle_profile_paths.append(arguments[2])
	session.add_child(controller)
	if not controller.initialize_physical_world():
		quit(1)
		return
	controller.set_physics_process(false)
	controller._physics_process(0.002)
	require_condition(controller.time_seconds > 0.0 and not controller.failed, "Native world did not advance")
	var vehicle: Node3D = controller.registered_vehicles[1]["vehicle"]
	var snapshot: Dictionary = vehicle.get_physical_world_snapshot()
	require_condition(snapshot["solved_suspension_corners"].size() == 4, "Native snapshot omitted solved suspension corners")
	require_condition(vehicle.freeze and vehicle.collision_layer == 0 and vehicle.collision_mask == 0 and vehicle.gravity_scale == 0.0, "Godot retained physical response ownership")
	var pose: Dictionary = snapshot["state"]["suspension"]["body_origin_world_metres"]
	require_condition(vehicle.global_position.distance_to(Vector3(pose["x"], pose["y"], pose["z"])) < 0.00001, "Presentation body disagrees with Rust pose")
	vehicle.set_fuel_kg(20.0)
	snapshot = vehicle.get_physical_world_snapshot()
	require_condition(is_equal_approx(float(snapshot["fuel_mass_kilograms"]), 20.0) and is_equal_approx(float(snapshot["operating_mass_kilograms"]), 718.0), "Native refuel did not update the coupled mass")
	vehicle.replace_tires()
	vehicle.reset_vehicle(Vector3(1.0, 10.0, 2.0), 0.2)
	require_condition(vehicle.global_position.distance_to(Vector3(1.0, 10.0, 2.0)) < 0.00001, "Native reset lost the requested pose")
	for wheel_name in ["FrontLeftWheel", "FrontRightWheel", "RearLeftWheel", "RearRightWheel"]:
		var hub := Node3D.new()
		hub.name = wheel_name
		vehicle.add_child(hub)
		var steering := Node3D.new()
		steering.name = "SteerPivot"
		hub.add_child(steering)
		var camber := Node3D.new()
		camber.name = "CamberPivot"
		steering.add_child(camber)
		var spinner := Node3D.new()
		spinner.name = "Spinner"
		camber.add_child(spinner)
	var visuals := VisualControllerScript.new()
	visuals.vehicle = vehicle
	visuals.physics_config_path = arguments[2]
	vehicle.add_child(visuals)
	await process_frame
	visuals._physics_process(0.002)
	visuals._process(0.002)
	snapshot = vehicle.get_physical_world_snapshot()
	for wheel_index in range(4):
		var corner: Dictionary = snapshot["solved_suspension_corners"][wheel_index]
		var rendered: Dictionary = visuals._suspension_solved[wheel_index]
		require_condition(rendered["hub"].distance_to(visuals.physical_vector(corner["hub"])) < 0.00001, "Rendered hub differs from Rust solution")
		require_condition(rendered["damper"][1].distance_to(visuals.physical_vector(corner["damper_end"])) < 0.00001, "Rendered damper differs from Rust solution")
		require_condition(rendered["authoritative_physical_pose"], "Renderer selected cosmetic suspension geometry")
	require_condition(visuals.linkage_solve_count == 0, "Godot performed a second suspension solve")
	var time_before_pause: float = controller.time_seconds
	paused = true
	controller._physics_process(0.1)
	require_condition(controller.time_seconds == time_before_pause, "Paused native world consumed time")
	paused = false
	controller._physics_process(0.002)
	require_condition(not controller.failed, "Native world failed after reset and pause")
	var removed_vehicle: Node = controller.registered_vehicles[2]["vehicle"]
	removed_vehicle.free()
	require_condition(not controller.registered_vehicles.has(2), "Native removal retained a stale vehicle binding")
	controller._physics_process(0.002)
	require_condition(not controller.failed, "Remaining vehicle failed after native removal")
	if not contract_failed:
		print("PHYSICAL_WORLD_NATIVE_PRESENTATION_PASS")
	quit(1 if contract_failed else 0)
