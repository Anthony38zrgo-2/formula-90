class_name PitCrewVisualController
extends Node3D

const WHEEL_GUN_ARM_INVERSE_KINEMATICS_MODIFIER_SCRIPT := preload(
	"res://scripts/runtime/wheel_gun_arm_inverse_kinematics_modifier.gd")
const CREW_MODEL_DIRECTORY := "res://assets/models/pit_crew/racer/poses/"
const WHEEL_MODEL_DIRECTORY := "res://assets/models/vehicles/f1-2030/"
const WHEEL_NAMES := ["front_left", "front_right", "rear_left", "rear_right"]
const WHEEL_MODEL_FILES := [
	"f1_2030_v10_wheel_FL.glb",
	"f1_2030_v10_wheel_FR.glb",
	"f1_2030_v10_wheel_RL.glb",
	"f1_2030_v10_wheel_RR.glb",
]
const WHEEL_CENTER_NUT_MESH_NAMES := [
	"GEO_WHEEL_FRONT_L_NUT_02",
	"GEO_WHEEL_FRONT_R_NUT_02",
	"GEO_WHEEL_REAR_L_NUT_02",
	"GEO_WHEEL_REAR_R_NUT_02",
]
const VEHICLE_WHEEL_PATHS := [
	"FrontLeftWheel/SteerPivot/CamberPivot/Visual",
	"FrontRightWheel/SteerPivot/CamberPivot/Visual",
	"RearLeftWheel/SteerPivot/CamberPivot/Visual",
	"RearRightWheel/SteerPivot/CamberPivot/Visual",
]
const FUEL_PORT_LOCAL_POSITION := Vector3(-0.78, 0.72, 0.35)
const FUEL_OPERATOR_RETREAT_SECONDS := 1.1
const WHEEL_GUN_STANDOFF_DISTANCE := 0.52
const WHEEL_GUN_MINIMUM_BODY_STANDOFF_DISTANCE := 0.28
const WHEEL_GUN_HAND_REACH_MARGIN := 0.035
const WHEEL_GUN_POLE_DISTANCE := 0.25
const WHEEL_GUN_TARGET_POSITION_TOLERANCE := 0.015
const WHEEL_GUN_MINIMUM_AXIS_ALIGNMENT_DOT := 0.995
const IDLE_ANIMATION_NAME := &"idle_wait"
const SERVICE_ANIMATION_NAME := &"service_sequence"
const WHEEL_PHASE_OFFSETS := [0.0, 0.06, 0.10, 0.14]
const IDLE_LOOP_DESYNC_MODULUS := 997

@export var show_wheel_gun_target_debug := false

var pit_stop: PitStopController
var vehicle: Node3D
var crew_members: Dictionary = {}
var crew_member_resting_transforms: Dictionary = {}
var crew_member_service_transforms: Dictionary = {}
var crew_member_animation_players: Dictionary = {}
var wheel_change_tools: Dictionary = {}
var wheel_change_tool_resting_transforms: Dictionary = {}
var wheel_gun_aim_targets: Dictionary = {}
var wheel_gun_rigs: Dictionary = {}
var wheel_exchanges: Array[Dictionary] = []
var carried_removed_wheels: Array[Node3D] = []
var crew_root: Node3D
var tire_service_duration := 0.0
var fuel_service_duration := 0.0
var fuel_operator_resting_transform := Transform3D.IDENTITY
var fuel_operator_filling_transform := Transform3D.IDENTITY
var fuel_operator_retreat_elapsed := -1.0


func configure(next_pit_stop: PitStopController, next_vehicle: Node3D) -> void:
	pit_stop = next_pit_stop
	vehicle = next_vehicle
	if pit_stop == null or vehicle == null or not pit_stop.is_configured:
		return
	pit_stop.crew_visibility_changed.connect(_on_crew_visibility_changed)
	pit_stop.service_started.connect(_on_service_started)
	pit_stop.service_progress.connect(_on_service_progress)
	pit_stop.service_completed.connect(_on_service_completed)
	_create_crew()
	_on_crew_visibility_changed(pit_stop.crew_visible)


func _process(delta: float) -> void:
	if fuel_operator_retreat_elapsed < 0.0:
		return
	var fuel_operator := crew_members.get("fuel_hose_operator") as Node3D
	if fuel_operator == null:
		fuel_operator_retreat_elapsed = -1.0
		return
	fuel_operator_retreat_elapsed += delta
	var retreat_progress := clampf(fuel_operator_retreat_elapsed / FUEL_OPERATOR_RETREAT_SECONDS, 0.0, 1.0)
	var retreat_fraction := smoothstep(0.0, 1.0, retreat_progress)
	fuel_operator.global_transform = fuel_operator_filling_transform.interpolate_with(
		fuel_operator_resting_transform, retreat_fraction)
	if retreat_progress >= 1.0:
		fuel_operator_retreat_elapsed = -1.0


func _on_crew_visibility_changed(should_be_visible: bool) -> void:
	if crew_root == null:
		return
	if not should_be_visible and pit_stop != null and pit_stop.is_servicing():
		return
	crew_root.visible = should_be_visible
	if should_be_visible and not pit_stop.is_servicing():
		_start_idle_animations()


func _create_crew() -> void:
	var assigned_box := pit_stop.get_assigned_box()
	if assigned_box.is_empty():
		return
	crew_root = Node3D.new()
	crew_root.name = "PitCrewMembers"
	add_child(crew_root)
	var box_center: Vector3 = assigned_box.get("center", Vector3.ZERO)
	var pit_forward := pit_stop.get_pit_forward()
	var pit_right := pit_stop.get_pit_right()
	for wheel_index in WHEEL_NAMES.size():
		var wheel_name: String = WHEEL_NAMES[wheel_index]
		var side_sign := -1.0 if wheel_index % 2 == 0 else 1.0
		var longitudinal_distance := 1.48 if wheel_index < 2 else -1.48
		var wheel_center := box_center + pit_forward * longitudinal_distance
		_place_member(
			wheel_name + "_wheel_change_mechanic",
			wheel_center + pit_right * side_sign * 1.35,
			side_sign)
		_place_member(
			wheel_name + "_wheel_carrier",
			wheel_center + pit_right * side_sign * 2.15 + pit_forward * -0.15,
			side_sign)
	_place_member("front_jack_operator", box_center + pit_forward * 3.0, 0.0)
	_place_member("pit_signaler", box_center + pit_forward * 5.2, 0.0)
	_place_member("fuel_hose_operator", box_center + pit_right * -2.25 + pit_forward * -0.35, -1.0)


func _place_member(member_name: String, world_position: Vector3, side_sign: float) -> void:
	var member_scene := load(CREW_MODEL_DIRECTORY + member_name + ".glb") as PackedScene
	if member_scene == null:
		push_error("Missing pit crew pose: " + member_name)
		return
	var member := member_scene.instantiate() as Node3D
	member.name = member_name
	var inverse_kinematics_modifier
	if member_name.ends_with("_wheel_change_mechanic"):
		var member_skeleton := member.find_child("Skeleton3D", true, false) as Skeleton3D
		if member_skeleton == null:
			push_error("Wheel-change mechanic has no skeleton: " + member_name)
			return
		inverse_kinematics_modifier = WHEEL_GUN_ARM_INVERSE_KINEMATICS_MODIFIER_SCRIPT.new()
		inverse_kinematics_modifier.name = "WheelGunArmInverseKinematics"
		inverse_kinematics_modifier.active = true
		inverse_kinematics_modifier.influence = 0.0
		member_skeleton.add_child(inverse_kinematics_modifier)
	crew_root.add_child(member)
	member.global_position = world_position
	member.rotation.y = atan2(pit_stop.get_pit_forward().x, -pit_stop.get_pit_forward().z)
	if side_sign != 0.0:
		member.rotation.y += -side_sign * PI * 0.5
	crew_members[member_name] = member
	crew_member_resting_transforms[member_name] = member.transform
	crew_member_animation_players[member_name] = member.find_child("AnimationPlayer", true, false) as AnimationPlayer
	if inverse_kinematics_modifier != null:
		_create_wheel_change_tool_pivot(member_name, member, inverse_kinematics_modifier)


func _create_wheel_change_tool_pivot(
	member_name: String, member: Node3D, inverse_kinematics_modifier
) -> void:
	var tool_pivot := Node3D.new()
	tool_pivot.name = "WheelChangeToolPivot"
	member.add_child(tool_pivot)
	tool_pivot.position = Vector3(0.10, 0.71, 0.635)
	var wheel_tool_meshes: Dictionary = {}
	for tool_name in ["WheelChangeToolGrip", "WheelChangeToolHandle", "WheelChangeToolSocket"]:
		var tool_part := member.find_child(tool_name, true, false) as Node3D
		if tool_part != null:
			tool_part.reparent(tool_pivot, true)
			wheel_tool_meshes[tool_name] = tool_part
	wheel_change_tools[member_name] = tool_pivot
	wheel_change_tool_resting_transforms[member_name] = tool_pivot.transform
	var grip_mesh := wheel_tool_meshes.get("WheelChangeToolGrip") as MeshInstance3D
	var handle_mesh := wheel_tool_meshes.get("WheelChangeToolHandle") as MeshInstance3D
	var socket_mesh := wheel_tool_meshes.get("WheelChangeToolSocket") as MeshInstance3D
	if grip_mesh == null or handle_mesh == null or socket_mesh == null:
		push_error("Wheel-change tool is missing a grip, handle, or socket for " + member_name)
		return
	var left_hand_target := _create_mesh_center_target("WheelGunLeftHandTarget", grip_mesh, tool_pivot)
	var right_hand_target := _create_mesh_center_target("WheelGunRightHandTarget", handle_mesh, tool_pivot)
	var socket_tip_target := _create_socket_tip_target(socket_mesh, handle_mesh, tool_pivot)
	if left_hand_target == null or right_hand_target == null or socket_tip_target == null:
		push_error("Wheel-change tool targets could not be created for " + member_name)
		return
	_create_wheel_gun_inverse_kinematics(
		member_name, member, left_hand_target, right_hand_target, socket_tip_target,
		inverse_kinematics_modifier)


func _create_mesh_center_target(target_name: String, source_mesh: MeshInstance3D, target_parent: Node3D) -> Node3D:
	if source_mesh.mesh == null:
		return null
	var target := Node3D.new()
	target.name = target_name
	target_parent.add_child(target)
	target.global_position = source_mesh.global_transform * source_mesh.mesh.get_aabb().get_center()
	return target


func _create_socket_tip_target(
	socket_mesh: MeshInstance3D, handle_mesh: MeshInstance3D, target_parent: Node3D
) -> Node3D:
	if socket_mesh.mesh == null or handle_mesh.mesh == null:
		return null
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
	var socket_tip_in_world_space := socket_mesh.global_transform * socket_tip_in_mesh_space
	var target := Node3D.new()
	target.name = "WheelGunSocketTipTarget"
	target_parent.add_child(target)
	target.global_transform = Transform3D(
		Basis.looking_at(socket_axis_in_world_space, Vector3.UP), socket_tip_in_world_space)
	return target


func _create_wheel_gun_inverse_kinematics(
	member_name: String,
	member: Node3D,
	left_hand_target: Node3D,
	right_hand_target: Node3D,
	socket_tip_target: Node3D,
	inverse_kinematics_modifier
) -> void:
	var skeleton := member.find_child("Skeleton3D", true, false) as Skeleton3D
	if skeleton == null or inverse_kinematics_modifier == null:
		push_error("Wheel-change mechanic has no skeleton: " + member_name)
		return
	var arm_configurations: Array[Dictionary] = [
		{
			"end_bone_name": "mixamorig_LeftHand",
			"hand_target": left_hand_target,
			"middle_bone_name": "mixamorig_LeftForeArm",
			"pole_name": "WheelGunLeftArmPoleTarget",
			"root_bone_name": "mixamorig_LeftArm",
		},
		{
			"end_bone_name": "mixamorig_RightHand",
			"hand_target": right_hand_target,
			"middle_bone_name": "mixamorig_RightForeArm",
			"pole_name": "WheelGunRightArmPoleTarget",
			"root_bone_name": "mixamorig_RightArm",
		},
	]
	var pole_targets: Array[Node3D] = []
	for arm_configuration in arm_configurations:
		var pole_target := Node3D.new()
		pole_target.name = arm_configuration["pole_name"]
		member.add_child(pole_target)
		pole_targets.append(pole_target)
		arm_configuration["pole_target"] = pole_target
	inverse_kinematics_modifier.arm_configurations = arm_configurations
	wheel_gun_rigs[member_name] = {
		"arm_configurations": arm_configurations,
		"inverse_kinematics_modifier": inverse_kinematics_modifier,
		"pole_targets": pole_targets,
		"skeleton": skeleton,
		"socket_tip_target": socket_tip_target,
	}


func _start_idle_animations() -> void:
	for member_name in crew_member_animation_players:
		var player := crew_member_animation_players[member_name] as AnimationPlayer
		if player == null:
			continue
		player.speed_scale = 1.0
		player.play(IDLE_ANIMATION_NAME, 0.35)
		var desync_fraction := float(posmod(member_name.hash(), IDLE_LOOP_DESYNC_MODULUS)) / float(IDLE_LOOP_DESYNC_MODULUS)
		player.seek(desync_fraction * player.get_current_animation_length())


func _scrub_member_animation(member_name: String, fraction: float) -> void:
	var player := crew_member_animation_players.get(member_name) as AnimationPlayer
	if player == null:
		return
	if player.assigned_animation != SERVICE_ANIMATION_NAME:
		player.play(SERVICE_ANIMATION_NAME)
	player.speed_scale = 0.0
	player.seek(clampf(fraction, 0.0, 1.0) * player.get_current_animation_length(), true)


func _on_service_started(plan: Dictionary) -> void:
	if crew_root == null:
		return
	_restore_carried_wheels()
	for member_name in crew_member_resting_transforms:
		var member := crew_members.get(member_name) as Node3D
		if member != null:
			member.transform = crew_member_resting_transforms[member_name]
	var box_center: Vector3 = pit_stop.get_assigned_box().get("center", Vector3.ZERO)
	var vehicle_offset := vehicle.global_position - box_center
	crew_root.global_position = Vector3(vehicle_offset.x, 0.0, vehicle_offset.z)
	crew_member_service_transforms.clear()
	for member_name in crew_members:
		crew_member_service_transforms[member_name] = (crew_members[member_name] as Node3D).global_transform
	_prepare_fuel_operator()
	fuel_operator_retreat_elapsed = -1.0
	tire_service_duration = maxf(float(plan.get("tire_seconds", 0.0)), 0.001)
	fuel_service_duration = maxf(float(plan.get("fuel_seconds", 0.0)), 0.001)
	_start_wheel_exchanges()
	for member_name in crew_member_animation_players:
		_scrub_member_animation(member_name, 0.0)


func _prepare_fuel_operator() -> void:
	var fuel_operator := crew_members.get("fuel_hose_operator") as Node3D
	if fuel_operator == null:
		return
	fuel_operator_resting_transform = fuel_operator.global_transform
	fuel_operator_filling_transform = fuel_operator_resting_transform
	var fuel_nozzle := fuel_operator.find_child("FuelNozzle", true, false) as Node3D
	if fuel_nozzle == null:
		return
	var fuel_port_position := vehicle.global_transform * FUEL_PORT_LOCAL_POSITION
	var approach_displacement := fuel_port_position - fuel_nozzle.global_position
	approach_displacement.y = 0.0
	fuel_operator_filling_transform = fuel_operator_resting_transform.translated(approach_displacement)


func _restore_carried_wheels() -> void:
	for removed_wheel in carried_removed_wheels:
		if is_instance_valid(removed_wheel):
			removed_wheel.free()
	carried_removed_wheels.clear()
	for wheel_name in WHEEL_NAMES:
		var carrier := crew_members.get(wheel_name + "_wheel_carrier") as Node3D
		if carrier == null or carrier.find_child("CarriedWheel", true, false) != null:
			continue
		var carrier_scene := load(CREW_MODEL_DIRECTORY + wheel_name + "_wheel_carrier.glb") as PackedScene
		if carrier_scene == null:
			continue
		var spare_carrier := carrier_scene.instantiate() as Node3D
		var spare_wheel := spare_carrier.find_child("CarriedWheel", true, false) as Node3D
		if spare_wheel != null:
			spare_wheel.owner = null
			spare_wheel.reparent(carrier, false)
		spare_carrier.free()


func _start_wheel_exchanges() -> void:
	_clear_wheel_exchanges()
	for wheel_index in WHEEL_NAMES.size():
		var wheel_name: String = WHEEL_NAMES[wheel_index]
		var original_wheel := vehicle.get_node_or_null(VEHICLE_WHEEL_PATHS[wheel_index]) as Node3D
		var carrier := crew_members.get(wheel_name + "_wheel_carrier") as Node3D
		if original_wheel == null or carrier == null:
			continue
		var original_wheel_parts := _find_transferable_wheel_parts(original_wheel)
		if original_wheel_parts.size() != 2:
			continue
		var replacement_wheel := carrier.find_child("CarriedWheel", true, false) as Node3D
		var wheel_scene := load(WHEEL_MODEL_DIRECTORY + WHEEL_MODEL_FILES[wheel_index]) as PackedScene
		if replacement_wheel == null or wheel_scene == null:
			continue
		var removed_wheel := wheel_scene.instantiate() as Node3D
		var removed_wheel_parts := _find_transferable_wheel_parts(removed_wheel)
		if removed_wheel_parts.size() != 2:
			removed_wheel.free()
			continue
		var wheel_gun_aim_target := _create_wheel_gun_aim_target(original_wheel, wheel_index)
		if wheel_gun_aim_target == null:
			removed_wheel.free()
			continue
		var removed_brake_parts := removed_wheel.get_node_or_null("BrakeStatic") as Node3D
		if removed_brake_parts != null:
			removed_brake_parts.visible = false
		var removed_spin_parts := removed_wheel.get_node_or_null("SpinVisual") as Node3D
		for wheel_part in removed_spin_parts.get_children():
			if wheel_part is Node3D:
				wheel_part.visible = removed_wheel_parts.has(wheel_part)
		crew_root.add_child(removed_wheel)
		removed_wheel.global_transform = original_wheel.global_transform
		replacement_wheel.reparent(crew_root, true)
		var original_part_visibility: Array[bool] = []
		for wheel_part in original_wheel_parts:
			original_part_visibility.append(wheel_part.visible)
			wheel_part.visible = false
		wheel_exchanges.append({
			"carrier": carrier,
			"carrier_relative_replacement_transform": carrier.global_transform.affine_inverse() * replacement_wheel.global_transform,
			"original_parts": original_wheel_parts,
			"original_part_visibility": original_part_visibility,
			"removed": removed_wheel,
			"replacement": replacement_wheel,
			"wheel_gun_aim_target": wheel_gun_aim_target,
			"starting_removed_transform": removed_wheel.global_transform,
			"target_transform": original_wheel.global_transform,
			"outward_direction": pit_stop.get_pit_right() * (-1.0 if wheel_index % 2 == 0 else 1.0),
		})
		wheel_gun_aim_targets[wheel_name] = wheel_gun_aim_target


func _create_wheel_gun_aim_target(wheel_model_root: Node3D, wheel_index: int) -> Node3D:
	var spin_visual := wheel_model_root.get_node_or_null("SpinVisual") as Node3D
	if spin_visual == null:
		push_error("Wheel model has no SpinVisual: " + WHEEL_NAMES[wheel_index])
		return null
	var central_nut_mesh := spin_visual.find_child(
		WHEEL_CENTER_NUT_MESH_NAMES[wheel_index], true, false) as MeshInstance3D
	if central_nut_mesh == null or central_nut_mesh.mesh == null:
		push_error("Wheel model has no central exterior nut mesh: " + WHEEL_NAMES[wheel_index])
		return null
	var central_nut_bounds := central_nut_mesh.mesh.get_aabb()
	var outward_axis_sign := 1.0 if wheel_index % 2 == 1 else -1.0
	var center_nut_face_position := central_nut_bounds.get_center()
	center_nut_face_position.x = (
		central_nut_bounds.position.x + central_nut_bounds.size.x
		if outward_axis_sign > 0.0
		else central_nut_bounds.position.x)
	var target_world_position := central_nut_mesh.global_transform * center_nut_face_position
	var outward_direction := (central_nut_mesh.global_basis.x * outward_axis_sign).normalized()
	var wheel_gun_aim_target := Node3D.new()
	wheel_gun_aim_target.name = "WheelGunTarget"
	spin_visual.add_child(wheel_gun_aim_target)
	wheel_gun_aim_target.global_transform = Transform3D(
		Basis.looking_at(outward_direction, Vector3.UP), target_world_position)
	if show_wheel_gun_target_debug:
		_add_wheel_gun_target_debug_visuals(wheel_gun_aim_target)
	return wheel_gun_aim_target


func _add_wheel_gun_target_debug_visuals(wheel_gun_aim_target: Node3D) -> void:
	var target_material := StandardMaterial3D.new()
	target_material.albedo_color = Color(0.1, 1.0, 0.25)
	target_material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	var center_marker_mesh := SphereMesh.new()
	center_marker_mesh.radius = 0.025
	center_marker_mesh.height = 0.05
	var center_marker := MeshInstance3D.new()
	center_marker.name = "WheelGunTargetCenterDebugVisual"
	center_marker.mesh = center_marker_mesh
	center_marker.material_override = target_material
	center_marker.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	wheel_gun_aim_target.add_child(center_marker)
	var axis_marker_mesh := BoxMesh.new()
	axis_marker_mesh.size = Vector3(0.01, 0.01, 0.18)
	var axis_marker := MeshInstance3D.new()
	axis_marker.name = "WheelGunTargetAxisDebugVisual"
	axis_marker.mesh = axis_marker_mesh
	axis_marker.position.z = -0.09
	axis_marker.material_override = target_material
	axis_marker.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	wheel_gun_aim_target.add_child(axis_marker)


func _on_service_progress(status: Dictionary) -> void:
	if int(status.get("phase", 0)) == PitStopController.SERVICE_PHASE_FUEL:
		_clear_wheel_exchanges(true)
		_animate_tire_team(1.0)
		var fuel_progress := clampf(1.0 - float(status.get("fuel_seconds_remaining", 0.0)) / fuel_service_duration, 0.0, 1.0)
		_animate_fuel_operator(1.0)
		_scrub_member_animation("fuel_hose_operator", 0.2 + 0.8 * fuel_progress)
		return
	if int(status.get("phase", 0)) != PitStopController.SERVICE_PHASE_TIRES:
		return
	var progress := clampf(1.0 - float(status.get("tire_seconds_remaining", 0.0)) / tire_service_duration, 0.0, 1.0)
	_animate_tire_team(progress)
	var fuel_operator_approach := smoothstep(0.62, 0.96, progress)
	_animate_fuel_operator(fuel_operator_approach)
	_scrub_member_animation("fuel_hose_operator", 0.2 * fuel_operator_approach)
	for exchange in wheel_exchanges:
		var removed_wheel: Node3D = exchange["removed"]
		var replacement_wheel: Node3D = exchange["replacement"]
		var carrier: Node3D = exchange["carrier"]
		var carrier_relative_replacement_transform: Transform3D = exchange["carrier_relative_replacement_transform"]
		var starting_removed_transform: Transform3D = exchange["starting_removed_transform"]
		var target_transform: Transform3D = exchange["target_transform"]
		var outward_direction: Vector3 = exchange["outward_direction"]
		var handoff_carrier_transform: Transform3D = crew_member_service_transforms[carrier.name]
		handoff_carrier_transform.origin -= outward_direction * 0.62
		var handoff_replacement_transform := handoff_carrier_transform * carrier_relative_replacement_transform
		var parked_removed_transform := starting_removed_transform.translated(outward_direction * 0.85)
		removed_wheel.global_transform = starting_removed_transform.interpolate_with(
			parked_removed_transform, smoothstep(0.0, 0.42, progress))
		if progress < 0.42:
			replacement_wheel.global_transform = carrier.global_transform * carrier_relative_replacement_transform
		else:
			replacement_wheel.global_transform = handoff_replacement_transform.interpolate_with(
				target_transform, smoothstep(0.42, 0.82, progress))
		if progress >= 0.82:
			removed_wheel.global_transform = parked_removed_transform.interpolate_with(
				carrier.global_transform * carrier_relative_replacement_transform,
				smoothstep(0.82, 0.96, progress))


func _animate_tire_team(progress: float) -> void:
	for wheel_index in WHEEL_NAMES.size():
		var wheel_name: String = WHEEL_NAMES[wheel_index]
		var phase_offset: float = WHEEL_PHASE_OFFSETS[wheel_index]
		var outward_direction := pit_stop.get_pit_right() * (-1.0 if wheel_index % 2 == 0 else 1.0)
		var wheel_gun_aim_target := wheel_gun_aim_targets.get(wheel_name) as Node3D
		var mechanic_name := wheel_name + "_wheel_change_mechanic"
		_scrub_member_animation(mechanic_name, progress)
		_scrub_member_animation(wheel_name + "_wheel_carrier", progress)
		var mechanic := crew_members.get(mechanic_name) as Node3D
		if mechanic != null and crew_member_service_transforms.has(mechanic_name):
			var mechanic_approach := smoothstep(0.02 + phase_offset * 0.5, 0.20 + phase_offset * 0.5, progress)
			var mechanic_wind_up := _pulse(progress, maxf(0.02 + phase_offset * 0.5 - 0.06, 0.0), 0.02 + phase_offset * 0.5)
			var mechanic_retract := smoothstep(0.86, 1.0, progress)
			var release_motion := smoothstep(0.15 + phase_offset, 0.24 + phase_offset, progress) * (1.0 - smoothstep(0.30 + phase_offset, 0.40 + phase_offset, progress))
			var installation_motion := smoothstep(0.52 + phase_offset, 0.62 + phase_offset, progress) * (1.0 - smoothstep(0.70 + phase_offset, 0.84 + phase_offset, progress))
			var work_motion := maxf(release_motion, installation_motion)
			var mechanic_service_transform: Transform3D = crew_member_service_transforms[mechanic_name]
			var mechanic_transform := mechanic_service_transform
			var tool_engagement := mechanic_approach * (1.0 - mechanic_retract)
			if wheel_gun_aim_target != null:
				var wheel_face_outward_direction := wheel_gun_aim_target.global_basis * Vector3.FORWARD
				var wheel_facing_stance := mechanic_service_transform
				wheel_facing_stance.origin = wheel_gun_aim_target.global_position + (
					wheel_face_outward_direction * WHEEL_GUN_STANDOFF_DISTANCE)
				wheel_facing_stance.origin.y = mechanic_service_transform.origin.y
				wheel_facing_stance.basis = Basis.looking_at(
					-wheel_face_outward_direction, Vector3.UP)
				mechanic_transform = mechanic_service_transform.interpolate_with(
					wheel_facing_stance, tool_engagement)
			mechanic.global_transform = mechanic_transform
			var tool_pivot := wheel_change_tools.get(mechanic_name) as Node3D
			if tool_pivot != null:
				var inverse_kinematics_rig: Dictionary = wheel_gun_rigs.get(mechanic_name, {})
				var socket_tip_target := inverse_kinematics_rig.get("socket_tip_target") as Node3D
				var resting_tool_transform: Transform3D = mechanic.global_transform * (
					wheel_change_tool_resting_transforms[mechanic_name] as Transform3D)
				var aligned_tool_transform := resting_tool_transform
				var tool_roll_angle := sin(progress * PI * 12.0 + phase_offset * 3.0) * 0.12 * work_motion
				if wheel_gun_aim_target != null and socket_tip_target != null:
					aligned_tool_transform = _get_aligned_wheel_change_tool_transform(
						tool_pivot, socket_tip_target, wheel_gun_aim_target, tool_roll_angle)
				tool_pivot.global_transform = resting_tool_transform.interpolate_with(
					aligned_tool_transform, tool_engagement)
				if tool_engagement > 0.75:
					_move_wheel_change_mechanic_into_arm_reach(
						mechanic_name, mechanic, wheel_gun_aim_target)
					resting_tool_transform = mechanic.global_transform * (
						wheel_change_tool_resting_transforms[mechanic_name] as Transform3D)
					if wheel_gun_aim_target != null and socket_tip_target != null:
						aligned_tool_transform = _get_aligned_wheel_change_tool_transform(
							tool_pivot, socket_tip_target, wheel_gun_aim_target, tool_roll_angle)
					tool_pivot.global_transform = resting_tool_transform.interpolate_with(
						aligned_tool_transform, tool_engagement)
					_update_wheel_gun_arm_pole_targets(mechanic_name, mechanic)
					_validate_wheel_gun_alignment(
						mechanic_name, socket_tip_target, wheel_gun_aim_target, tool_engagement)
				var inverse_kinematics_modifier := inverse_kinematics_rig.get(
					"inverse_kinematics_modifier") as SkeletonModifier3D
				if inverse_kinematics_modifier != null:
					inverse_kinematics_modifier.influence = tool_engagement
		var carrier_name := wheel_name + "_wheel_carrier"
		var carrier := crew_members.get(carrier_name) as Node3D
		if carrier != null and crew_member_service_transforms.has(carrier_name):
			var carrier_approach := smoothstep(0.12 + phase_offset * 0.5, 0.42, progress) * (1.0 - smoothstep(0.82, 1.0, progress))
			var carrier_transform: Transform3D = crew_member_service_transforms[carrier_name]
			carrier_transform.origin -= outward_direction * 0.62 * carrier_approach
			carrier.global_transform = carrier_transform
	_scrub_member_animation("front_jack_operator", progress)
	_scrub_member_animation("pit_signaler", progress)


func _get_aligned_wheel_change_tool_transform(
	tool_pivot: Node3D,
	socket_tip_target: Node3D,
	wheel_gun_aim_target: Node3D,
	tool_roll_angle: float
) -> Transform3D:
	var wheel_face_outward_direction := wheel_gun_aim_target.global_basis * Vector3.FORWARD
	var socket_tip_basis := Basis.looking_at(-wheel_face_outward_direction, Vector3.UP)
	socket_tip_basis = socket_tip_basis.rotated(wheel_face_outward_direction, tool_roll_angle)
	var desired_socket_tip_transform := Transform3D(socket_tip_basis, wheel_gun_aim_target.global_position)
	var socket_tip_transform_relative_to_tool := (
		tool_pivot.global_transform.affine_inverse() * socket_tip_target.global_transform)
	return desired_socket_tip_transform * socket_tip_transform_relative_to_tool.affine_inverse()


func _move_wheel_change_mechanic_into_arm_reach(
	mechanic_name: String, mechanic: Node3D, wheel_gun_aim_target: Node3D
) -> void:
	var inverse_kinematics_rig: Dictionary = wheel_gun_rigs.get(mechanic_name, {})
	var skeleton := inverse_kinematics_rig.get("skeleton") as Skeleton3D
	var arm_configurations: Array = inverse_kinematics_rig.get("arm_configurations", [])
	if skeleton == null or arm_configurations.is_empty():
		return
	skeleton.force_update_all_bone_transforms()
	var wheel_face_outward_direction := wheel_gun_aim_target.global_basis * Vector3.FORWARD
	var remaining_standoff_adjustment := maxf(
		(mechanic.global_position - wheel_gun_aim_target.global_position).dot(wheel_face_outward_direction)
		- WHEEL_GUN_MINIMUM_BODY_STANDOFF_DISTANCE,
		0.0)
	for arm_configuration in arm_configurations:
		var root_bone_index := skeleton.find_bone(arm_configuration["root_bone_name"])
		var middle_bone_index := skeleton.find_bone(arm_configuration["middle_bone_name"])
		var end_bone_index := skeleton.find_bone(arm_configuration["end_bone_name"])
		if root_bone_index < 0 or middle_bone_index < 0 or end_bone_index < 0:
			continue
		var root_bone_position := skeleton.global_transform * skeleton.get_bone_global_pose(root_bone_index).origin
		var middle_bone_position := skeleton.global_transform * skeleton.get_bone_global_pose(middle_bone_index).origin
		var end_bone_position := skeleton.global_transform * skeleton.get_bone_global_pose(end_bone_index).origin
		var available_arm_reach := (
			root_bone_position.distance_to(middle_bone_position)
			+ middle_bone_position.distance_to(end_bone_position)
			- WHEEL_GUN_HAND_REACH_MARGIN)
		var hand_target := arm_configuration["hand_target"] as Node3D
		var hand_target_displacement := hand_target.global_position - root_bone_position
		var excess_reach := hand_target_displacement.length() - available_arm_reach
		if excess_reach <= 0.0:
			continue
		var inward_direction := -wheel_face_outward_direction
		var inward_alignment := hand_target_displacement.normalized().dot(inward_direction)
		if inward_alignment <= 0.1 or remaining_standoff_adjustment <= 0.0:
			continue
		var required_standoff_adjustment := excess_reach / inward_alignment
		var actual_standoff_adjustment := minf(required_standoff_adjustment, remaining_standoff_adjustment)
		mechanic.global_position += inward_direction * actual_standoff_adjustment
		remaining_standoff_adjustment -= actual_standoff_adjustment


func _update_wheel_gun_arm_pole_targets(mechanic_name: String, mechanic: Node3D) -> void:
	var inverse_kinematics_rig: Dictionary = wheel_gun_rigs.get(mechanic_name, {})
	var skeleton := inverse_kinematics_rig.get("skeleton") as Skeleton3D
	var arm_configurations: Array = inverse_kinematics_rig.get("arm_configurations", [])
	var pole_targets: Array = inverse_kinematics_rig.get("pole_targets", [])
	if skeleton == null or arm_configurations.size() != pole_targets.size():
		return
	skeleton.force_update_all_bone_transforms()
	for arm_index in arm_configurations.size():
		var arm_configuration: Dictionary = arm_configurations[arm_index]
		var root_bone_index := skeleton.find_bone(arm_configuration["root_bone_name"])
		var middle_bone_index := skeleton.find_bone(arm_configuration["middle_bone_name"])
		var end_bone_index := skeleton.find_bone(arm_configuration["end_bone_name"])
		if root_bone_index < 0 or middle_bone_index < 0 or end_bone_index < 0:
			continue
		var shoulder_position := skeleton.global_transform * skeleton.get_bone_global_pose(root_bone_index).origin
		var elbow_position := skeleton.global_transform * skeleton.get_bone_global_pose(middle_bone_index).origin
		var hand_position := (arm_configuration["hand_target"] as Node3D).global_position
		var shoulder_to_hand_direction := (hand_position - shoulder_position).normalized()
		var elbow_bend_direction := elbow_position - shoulder_position
		elbow_bend_direction -= shoulder_to_hand_direction * elbow_bend_direction.dot(shoulder_to_hand_direction)
		if elbow_bend_direction.length_squared() < 0.0001:
			var side_sign := -1.0 if arm_index == 0 else 1.0
			elbow_bend_direction = mechanic.global_basis.x * side_sign
		var pole_target := pole_targets[arm_index] as Node3D
		pole_target.global_position = elbow_position + elbow_bend_direction.normalized() * WHEEL_GUN_POLE_DISTANCE


func _validate_wheel_gun_alignment(
	mechanic_name: String,
	socket_tip_target: Node3D,
	wheel_gun_aim_target: Node3D,
	tool_engagement: float
) -> void:
	if not OS.is_debug_build() or tool_engagement < 0.999:
		return
	var position_error := socket_tip_target.global_position.distance_to(wheel_gun_aim_target.global_position)
	var tool_forward_direction := (socket_tip_target.global_basis * Vector3.FORWARD).normalized()
	var wheel_face_inward_direction := -(wheel_gun_aim_target.global_basis * Vector3.FORWARD).normalized()
	var axis_alignment := tool_forward_direction.dot(wheel_face_inward_direction)
	if position_error > WHEEL_GUN_TARGET_POSITION_TOLERANCE or axis_alignment < WHEEL_GUN_MINIMUM_AXIS_ALIGNMENT_DOT:
		push_warning(
			"Wheel-change tool missed the central wheel nut for %s: position error %.4f m, axis dot %.4f"
			% [mechanic_name, position_error, axis_alignment])


func _animate_fuel_operator(approach_fraction: float) -> void:
	var fuel_operator := crew_members.get("fuel_hose_operator") as Node3D
	if fuel_operator == null:
		return
	fuel_operator.global_transform = fuel_operator_resting_transform.interpolate_with(
		fuel_operator_filling_transform, approach_fraction)


func _on_service_completed() -> void:
	_clear_wheel_exchanges(true)
	_animate_tire_team(1.0)
	_animate_fuel_operator(1.0)
	_start_idle_animations()
	fuel_operator_retreat_elapsed = 0.0


static func _pulse(progress: float, window_start: float, window_end: float) -> float:
	if window_end <= window_start:
		return 0.0
	var window_middle := (window_start + window_end) * 0.5
	return smoothstep(window_start, window_middle, progress) * (
		1.0 - smoothstep(window_middle, window_end, progress))


func _find_transferable_wheel_parts(wheel_root: Node3D) -> Array[Node3D]:
	var transferable_parts: Array[Node3D] = []
	var spin_visual := wheel_root.get_node_or_null("SpinVisual") as Node3D
	if spin_visual == null:
		return transferable_parts
	for wheel_part in spin_visual.get_children():
		if wheel_part is Node3D and (wheel_part.name.ends_with("_RIM_04") or wheel_part.name.ends_with("_TIRE")):
			transferable_parts.append(wheel_part)
	return transferable_parts


func _clear_wheel_exchanges(retain_removed_wheels: bool = false) -> void:
	for exchange in wheel_exchanges:
		var original_parts: Array[Node3D] = exchange["original_parts"]
		var original_part_visibility: Array[bool] = exchange["original_part_visibility"]
		for part_index in original_parts.size():
			if is_instance_valid(original_parts[part_index]):
				original_parts[part_index].visible = original_part_visibility[part_index]
		var removed_wheel: Node3D = exchange["removed"]
		if is_instance_valid(removed_wheel):
			var carrier: Node3D = exchange["carrier"]
			if retain_removed_wheels and is_instance_valid(carrier):
				removed_wheel.reparent(carrier, true)
				carried_removed_wheels.append(removed_wheel)
			else:
				removed_wheel.queue_free()
		var replacement_wheel: Node3D = exchange["replacement"]
		if is_instance_valid(replacement_wheel):
			replacement_wheel.queue_free()
		var wheel_gun_aim_target: Node3D = exchange.get("wheel_gun_aim_target") as Node3D
		if is_instance_valid(wheel_gun_aim_target):
			wheel_gun_aim_target.free()
	wheel_exchanges.clear()
	wheel_gun_aim_targets.clear()
