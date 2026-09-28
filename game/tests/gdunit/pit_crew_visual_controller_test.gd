extends GdUnitTestSuite

const PIT_STOP_SCRIPT := preload("res://scripts/runtime/pit_stop_controller.gd")
const PIT_STOP_RULES_SCRIPT := preload("res://scripts/runtime/pit_stop_rules.gd")
const PIT_CREW_VISUAL_SCRIPT := preload("res://scripts/runtime/pit_crew_visual_controller.gd")
const WHEEL_FILES := [
	"f1_2030_v10_wheel_FL.glb",
	"f1_2030_v10_wheel_FR.glb",
	"f1_2030_v10_wheel_RL.glb",
	"f1_2030_v10_wheel_RR.glb",
]
const WHEEL_HUB_NAMES := ["FrontLeftWheel", "FrontRightWheel", "RearLeftWheel", "RearRightWheel"]
const IDLE_CLIP_NAME := &"idle_wait"
const SERVICE_CLIP_NAME := &"service_sequence"
const LEFT_HAND_BONE_NAME := "mixamorig_LeftHand"
const LEFT_KNEE_BONE_NAME := "mixamorig_LeftLeg"
const LEFT_FOOT_BONE_NAME := "mixamorig_LeftFoot"
const RIGHT_FOOT_BONE_NAME := "mixamorig_RightFoot"


class VisualTestVehicle:
	extends Node3D

	var enable_player_input := true

	func get_fuel_state_snapshot() -> Dictionary:
		return {"remaining_kg": 40.0, "capacity_kg": 110.0}

	func get_speed_kmh() -> float:
		return 0.0


func test_fuji_crew_has_eleven_members_and_exchanges_four_wheels() -> void:
	var vehicle := auto_free(VisualTestVehicle.new()) as VisualTestVehicle
	add_child(vehicle)
	for wheel_index in WHEEL_HUB_NAMES.size():
		var hub := Node3D.new()
		hub.name = WHEEL_HUB_NAMES[wheel_index]
		vehicle.add_child(hub)
		var steering_pivot := Node3D.new()
		steering_pivot.name = "SteerPivot"
		hub.add_child(steering_pivot)
		var camber_pivot := Node3D.new()
		camber_pivot.name = "CamberPivot"
		steering_pivot.add_child(camber_pivot)
		var wheel_scene := load("res://assets/models/vehicles/f1-2030/" + WHEEL_FILES[wheel_index]) as PackedScene
		var wheel := wheel_scene.instantiate() as Node3D
		wheel.name = "Visual"
		camber_pivot.add_child(wheel)
	var controller := auto_free(PIT_STOP_SCRIPT.new()) as PitStopController
	add_child(controller)
	assert_bool(controller.configure(
		vehicle,
		load("res://data/tracks/fuji76_77.tres") as TrackDefinition,
		load("res://data/vehicles/f1_2030_v10.tres") as VehicleDefinition,
		PIT_STOP_RULES_SCRIPT.load_from_json())).is_true()
	var visual := auto_free(PIT_CREW_VISUAL_SCRIPT.new()) as PitCrewVisualController
	add_child(visual)
	visual.configure(controller, vehicle)
	var processed_wheel_gun_arm_poses: Dictionary = {}
	for mechanic_name in visual.wheel_gun_rigs:
		var inverse_kinematics_rig: Dictionary = visual.wheel_gun_rigs[mechanic_name]
		var modifier := inverse_kinematics_rig[
			"inverse_kinematics_modifier"] as SkeletonModifier3D
		modifier.modification_processed.connect(
			_record_wheel_gun_arm_poses.bind(
				mechanic_name,
				inverse_kinematics_rig["skeleton"] as Skeleton3D,
				inverse_kinematics_rig["arm_configurations"],
				processed_wheel_gun_arm_poses))
	assert_int(visual.crew_members.size()).is_equal(11)
	assert_bool(visual.crew_root.visible).is_false()
	var initial_crew_root := visual.crew_root
	var initial_carrier := visual.crew_members["front_left_wheel_carrier"] as Node3D
	controller.set_selected_stop_lap(2)
	controller.confirm_selection()
	controller.on_lap_started(2)

	assert_int(visual.crew_members.size()).is_equal(11)
	assert_bool(visual.crew_root.visible).is_true()
	assert_object(visual.crew_root).is_same(initial_crew_root)
	assert_object(visual.crew_members["front_left_wheel_carrier"]).is_same(initial_carrier)
	assert_int(visual.crew_root.get_child_count()).is_equal(11)
	visual._on_crew_visibility_changed(false)
	assert_bool(visual.crew_root.visible).is_false()
	assert_int(visual.crew_members.size()).is_equal(11)
	visual._on_crew_visibility_changed(true)
	assert_bool(visual.crew_root.visible).is_true()
	assert_object(visual.crew_members["front_left_wheel_carrier"]).is_same(initial_carrier)
	for member_name in visual.crew_members:
		var member := visual.crew_members[member_name] as Node3D
		var idle_player := member.find_child("AnimationPlayer", true, false) as AnimationPlayer
		assert_object(idle_player).is_not_null()
		assert_bool(idle_player.has_animation(IDLE_CLIP_NAME)).is_true()
		assert_bool(idle_player.has_animation(SERVICE_CLIP_NAME)).is_true()
		assert_str(idle_player.assigned_animation).is_equal(IDLE_CLIP_NAME)
	for wheel_name in visual.WHEEL_NAMES:
		var carrier := visual.crew_members[wheel_name + "_wheel_carrier"] as Node3D
		assert_object(carrier.find_child("CarriedWheel", true, false)).is_not_null()
	visual._on_service_started({"tire_seconds": 3.0})
	assert_int(visual.wheel_exchanges.size()).is_equal(4)
	assert_int(visual.wheel_gun_aim_targets.size()).is_equal(4)
	_assert_wheel_gun_targets_match_center_nuts(visual, vehicle)
	var fuel_operator := visual.crew_members["fuel_hose_operator"] as Node3D
	var fuel_operator_resting_position := visual.fuel_operator_resting_transform.origin
	var mechanic := visual.crew_members["front_left_wheel_change_mechanic"] as Node3D
	var mechanic_player := mechanic.find_child("AnimationPlayer", true, false) as AnimationPlayer
	assert_str(mechanic_player.assigned_animation).is_equal(SERVICE_CLIP_NAME)
	var mechanic_skeleton := mechanic.find_child("Skeleton3D", true, false) as Skeleton3D
	var left_hand_bone := mechanic_skeleton.find_bone(LEFT_HAND_BONE_NAME)
	var left_knee_bone := mechanic_skeleton.find_bone(LEFT_KNEE_BONE_NAME)
	var left_foot_bone := mechanic_skeleton.find_bone(LEFT_FOOT_BONE_NAME)
	var right_foot_bone := mechanic_skeleton.find_bone(RIGHT_FOOT_BONE_NAME)
	assert_int(left_hand_bone).is_not_equal(-1)
	assert_int(left_knee_bone).is_not_equal(-1)
	assert_int(left_foot_bone).is_not_equal(-1)
	assert_int(right_foot_bone).is_not_equal(-1)
	await await_idle_frame()
	var hand_at_start: Vector3 = mechanic_skeleton.get_bone_global_pose(left_hand_bone).origin
	var knee_at_start: Vector3 = mechanic_skeleton.get_bone_global_pose(left_knee_bone).origin
	visual._on_service_progress({"phase": PitStopController.SERVICE_PHASE_TIRES, "tire_seconds_remaining": 2.4})
	var hand_at_work: Vector3 = mechanic_skeleton.get_bone_global_pose(left_hand_bone).origin
	var knee_at_work: Vector3 = mechanic_skeleton.get_bone_global_pose(left_knee_bone).origin
	assert_float(hand_at_start.distance_to(hand_at_work)).is_greater(0.08)
	assert_float(knee_at_start.distance_to(knee_at_work)).is_greater(0.02)
	for foot_bone in [left_foot_bone, right_foot_bone]:
		var foot_world_position: Vector3 = mechanic_skeleton.global_transform * mechanic_skeleton.get_bone_global_pose(foot_bone).origin
		var foot_height_above_member_origin := foot_world_position.y - mechanic.global_position.y
		assert_bool(foot_height_above_member_origin >= -0.06 and foot_height_above_member_origin <= 0.22).is_true()
	visual._on_service_progress({"phase": PitStopController.SERVICE_PHASE_TIRES, "tire_seconds_remaining": 1.5})
	await await_idle_frame()
	_assert_wheel_gun_tools_face_center_nuts(visual, processed_wheel_gun_arm_poses)
	var mechanic_resting_transform: Transform3D = visual.crew_member_service_transforms[mechanic.name]
	assert_float(mechanic.global_position.distance_to(mechanic_resting_transform.origin)).is_greater(0.2)
	visual._on_service_progress({"phase": PitStopController.SERVICE_PHASE_TIRES, "tire_seconds_remaining": 1.74})
	var front_left_exchange: Dictionary = visual.wheel_exchanges[0]
	var front_left_carrier := front_left_exchange["carrier"] as Node3D
	var carrier_resting_transform: Transform3D = visual.crew_member_service_transforms[front_left_carrier.name]
	assert_float(front_left_carrier.global_position.distance_to(carrier_resting_transform.origin)).is_greater(0.5)
	var carried_replacement := front_left_exchange["replacement"] as Node3D
	var carried_wheel_transform: Transform3D = front_left_exchange["carrier_relative_replacement_transform"]
	assert_float(carried_replacement.global_position.distance_to(
		(front_left_carrier.global_transform * carried_wheel_transform).origin)).is_less(0.05)
	visual._on_service_progress({"phase": PitStopController.SERVICE_PHASE_TIRES, "tire_seconds_remaining": 0.12})
	var fuel_nozzle := fuel_operator.find_child("FuelNozzle", true, false) as Node3D
	var nozzle_to_port := fuel_nozzle.global_position - vehicle.global_transform * visual.FUEL_PORT_LOCAL_POSITION
	nozzle_to_port.y = 0.0
	assert_float(nozzle_to_port.length()).is_less(0.05)
	for exchange in visual.wheel_exchanges:
		var original_parts: Array[Node3D] = exchange["original_parts"]
		for original_part in original_parts:
			assert_bool(original_part.visible).is_false()
		var original_wheel := original_parts[0].get_parent().get_parent() as Node3D
		assert_bool((original_wheel.get_node("BrakeStatic") as Node3D).visible).is_true()
	visual._on_service_progress({"phase": PitStopController.SERVICE_PHASE_FUEL, "fuel_seconds_remaining": 1.0})
	assert_int(visual.wheel_exchanges.size()).is_equal(0)
	assert_int(visual.carried_removed_wheels.size()).is_equal(4)
	visual._on_service_completed()
	visual._process(1.2)
	await await_idle_frame()
	assert_float(fuel_operator.global_position.distance_to(fuel_operator_resting_position)).is_less(0.01)
	assert_str(mechanic_player.assigned_animation).is_equal(IDLE_CLIP_NAME)
	assert_float(mechanic_skeleton.get_bone_global_pose(left_hand_bone).origin.distance_to(hand_at_start)).is_less(0.15)
	for wheel_path in visual.VEHICLE_WHEEL_PATHS:
		for wheel_part in visual._find_transferable_wheel_parts(vehicle.get_node(wheel_path) as Node3D):
			assert_bool(wheel_part.visible).is_true()
	visual._on_service_started({"tire_seconds": 3.0})
	assert_int(visual.carried_removed_wheels.size()).is_equal(0)
	assert_int(visual.wheel_exchanges.size()).is_equal(4)
	visual._on_service_completed()


func _assert_wheel_gun_targets_match_center_nuts(
	visual: PitCrewVisualController, vehicle: VisualTestVehicle
) -> void:
	for wheel_index in visual.WHEEL_NAMES.size():
		var wheel_name: String = visual.WHEEL_NAMES[wheel_index]
		var wheel_model_root := vehicle.get_node(visual.VEHICLE_WHEEL_PATHS[wheel_index]) as Node3D
		var spin_visual := wheel_model_root.get_node("SpinVisual") as Node3D
		var center_nut_mesh := spin_visual.find_child(
			visual.WHEEL_CENTER_NUT_MESH_NAMES[wheel_index], true, false) as MeshInstance3D
		var center_nut_bounds := center_nut_mesh.mesh.get_aabb()
		var outward_axis_sign := 1.0 if wheel_index % 2 == 1 else -1.0
		var expected_face_position := center_nut_bounds.get_center()
		expected_face_position.x = (
			center_nut_bounds.position.x + center_nut_bounds.size.x
			if outward_axis_sign > 0.0
			else center_nut_bounds.position.x)
		var expected_world_position := center_nut_mesh.global_transform * expected_face_position
		var expected_outward_direction := (
			center_nut_mesh.global_basis.x * outward_axis_sign).normalized()
		var wheel_gun_aim_target := visual.wheel_gun_aim_targets[wheel_name] as Node3D
		var actual_outward_direction := wheel_gun_aim_target.global_basis * Vector3.FORWARD
		assert_float(wheel_gun_aim_target.global_position.distance_to(expected_world_position)).is_less(0.002)
		assert_float(actual_outward_direction.dot(expected_outward_direction)).is_greater(0.999)


func _assert_wheel_gun_tools_face_center_nuts(
	visual: PitCrewVisualController,
	processed_wheel_gun_arm_poses: Dictionary
) -> void:
	for wheel_name in visual.WHEEL_NAMES:
		var mechanic_name: String = wheel_name + "_wheel_change_mechanic"
		var inverse_kinematics_rig: Dictionary = visual.wheel_gun_rigs[mechanic_name]
		var modifier := inverse_kinematics_rig[
			"inverse_kinematics_modifier"] as SkeletonModifier3D
		var socket_tip_target := inverse_kinematics_rig["socket_tip_target"] as Node3D
		var wheel_gun_aim_target := visual.wheel_gun_aim_targets[wheel_name] as Node3D
		var expected_tool_forward_direction := -(
			wheel_gun_aim_target.global_basis * Vector3.FORWARD).normalized()
		var actual_tool_forward_direction := (
			socket_tip_target.global_basis * Vector3.FORWARD).normalized()
		var tool_pivot := visual.wheel_change_tools[mechanic_name] as Node3D
		var socket_mesh := tool_pivot.find_child("WheelChangeToolSocket", true, false) as MeshInstance3D
		var handle_mesh := tool_pivot.find_child("WheelChangeToolHandle", true, false) as MeshInstance3D
		var socket_bounds := socket_mesh.mesh.get_aabb()
		var socket_axis_index := socket_bounds.size.max_axis_index()
		var socket_axis_in_mesh_space := Vector3.ZERO
		socket_axis_in_mesh_space[socket_axis_index] = 1.0
		var socket_axis_in_world_space := (socket_mesh.global_basis * socket_axis_in_mesh_space).normalized()
		var handle_to_socket_direction := (socket_mesh.global_position - handle_mesh.global_position).normalized()
		if socket_axis_in_world_space.dot(handle_to_socket_direction) < 0.0:
			socket_axis_in_mesh_space = -socket_axis_in_mesh_space
			socket_axis_in_world_space = -socket_axis_in_world_space
		var socket_tip_in_mesh_space := socket_bounds.get_center() + socket_axis_in_mesh_space * (
			socket_bounds.size[socket_axis_index] * 0.5)
		var physical_socket_tip_position := socket_mesh.global_transform * socket_tip_in_mesh_space
		assert_float(socket_tip_target.global_position.distance_to(
			wheel_gun_aim_target.global_position)).is_less(0.002)
		assert_float(actual_tool_forward_direction.dot(expected_tool_forward_direction)).is_greater(0.995)
		assert_float(physical_socket_tip_position.distance_to(
			wheel_gun_aim_target.global_position)).is_less(0.002)
		assert_float(socket_axis_in_world_space.dot(expected_tool_forward_direction)).is_greater(0.995)
		assert_float(modifier.influence).is_greater(0.99)
		assert_int(modifier.get("completed_modification_count")).is_greater(0)
		var skeleton := inverse_kinematics_rig["skeleton"] as Skeleton3D
		var arm_configurations: Array = inverse_kinematics_rig["arm_configurations"]
		for arm_configuration in arm_configurations:
			assert_int(skeleton.find_bone(arm_configuration["root_bone_name"])).is_greater_equal(0)
			assert_int(skeleton.find_bone(arm_configuration["middle_bone_name"])).is_greater_equal(0)
			assert_int(skeleton.find_bone(arm_configuration["end_bone_name"])).is_greater_equal(0)
		var captured_arm_poses: Array = processed_wheel_gun_arm_poses.get(mechanic_name, [])
		assert_int(captured_arm_poses.size()).is_equal(arm_configurations.size())
		for captured_arm_pose in captured_arm_poses:
			var shoulder_world_position: Vector3 = captured_arm_pose["shoulder_world_position"]
			var hand_world_position: Vector3 = captured_arm_pose["hand_world_position"]
			var hand_target_world_position: Vector3 = captured_arm_pose["hand_target_world_position"]
			var desired_arm_direction := (hand_target_world_position - shoulder_world_position).normalized()
			var actual_arm_direction := (hand_world_position - shoulder_world_position).normalized()
			assert_float(actual_arm_direction.dot(desired_arm_direction)).is_greater(0.995)


func _record_wheel_gun_arm_poses(
	mechanic_name: String,
	skeleton: Skeleton3D,
	arm_configurations: Array,
	processed_wheel_gun_arm_poses: Dictionary
) -> void:
	var captured_arm_poses: Array[Dictionary] = []
	for arm_configuration in arm_configurations:
		var root_bone_index := skeleton.find_bone(arm_configuration["root_bone_name"])
		var end_bone_index := skeleton.find_bone(arm_configuration["end_bone_name"])
		var hand_target := arm_configuration["hand_target"] as Node3D
		captured_arm_poses.append({
			"hand_target_world_position": hand_target.global_position,
			"hand_world_position": (
				skeleton.global_transform * skeleton.get_bone_global_pose(end_bone_index).origin),
			"shoulder_world_position": (
				skeleton.global_transform * skeleton.get_bone_global_pose(root_bone_index).origin),
		})
	processed_wheel_gun_arm_poses[mechanic_name] = captured_arm_poses
