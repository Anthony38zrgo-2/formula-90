class_name PitCrewVisualController
extends Node3D

const CREW_MODEL_DIRECTORY := "res://assets/models/pit_crew/racer/poses/"
const WHEEL_MODEL_DIRECTORY := "res://assets/models/vehicles/f1-2030/"
const WHEEL_NAMES := ["front_left", "front_right", "rear_left", "rear_right"]
const WHEEL_MODEL_FILES := [
	"f1_2030_v10_wheel_FL.glb",
	"f1_2030_v10_wheel_FR.glb",
	"f1_2030_v10_wheel_RL.glb",
	"f1_2030_v10_wheel_RR.glb",
]
const VEHICLE_WHEEL_PATHS := [
	"FrontLeftWheel/SteerPivot/CamberPivot/Visual",
	"FrontRightWheel/SteerPivot/CamberPivot/Visual",
	"RearLeftWheel/SteerPivot/CamberPivot/Visual",
	"RearRightWheel/SteerPivot/CamberPivot/Visual",
]
const FUEL_PORT_LOCAL_POSITION := Vector3(-0.78, 0.72, 0.35)
const FUEL_OPERATOR_RETREAT_SECONDS := 1.1

var pit_stop: PitStopController
var vehicle: Node3D
var crew_members: Dictionary = {}
var crew_member_resting_transforms: Dictionary = {}
var crew_member_service_transforms: Dictionary = {}
var wheel_change_tools: Dictionary = {}
var wheel_change_tool_resting_transforms: Dictionary = {}
var wheel_exchanges: Array[Dictionary] = []
var carried_removed_wheels: Array[Node3D] = []
var crew_root: Node3D
var tire_service_duration := 0.0
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
	fuel_operator.global_position += Vector3.UP * sin(retreat_progress * PI * 4.0) * 0.025 * sin(retreat_progress * PI)
	if retreat_progress >= 1.0:
		fuel_operator_retreat_elapsed = -1.0


func _on_crew_visibility_changed(should_be_visible: bool) -> void:
	if crew_root == null:
		return
	if not should_be_visible and pit_stop != null and pit_stop.is_servicing():
		return
	crew_root.visible = should_be_visible


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
	crew_root.add_child(member)
	member.global_position = world_position
	member.rotation.y = atan2(pit_stop.get_pit_forward().x, -pit_stop.get_pit_forward().z)
	if side_sign != 0.0:
		member.rotation.y += -side_sign * PI * 0.5
	crew_members[member_name] = member
	crew_member_resting_transforms[member_name] = member.transform
	if member_name.ends_with("_wheel_change_mechanic"):
		_create_wheel_change_tool_pivot(member_name, member)


func _create_wheel_change_tool_pivot(member_name: String, member: Node3D) -> void:
	var tool_pivot := Node3D.new()
	tool_pivot.name = "WheelChangeToolPivot"
	member.add_child(tool_pivot)
	tool_pivot.position = Vector3(0.10, 0.71, 0.635)
	for tool_name in ["WheelChangeToolGrip", "WheelChangeToolHandle", "WheelChangeToolSocket"]:
		var tool_part := member.find_child(tool_name, true, false) as Node3D
		if tool_part != null:
			tool_part.reparent(tool_pivot, true)
	wheel_change_tools[member_name] = tool_pivot
	wheel_change_tool_resting_transforms[member_name] = tool_pivot.transform


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
	_start_wheel_exchanges()


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
			"starting_removed_transform": removed_wheel.global_transform,
			"target_transform": original_wheel.global_transform,
			"outward_direction": pit_stop.get_pit_right() * (-1.0 if wheel_index % 2 == 0 else 1.0),
		})


func _on_service_progress(status: Dictionary) -> void:
	if int(status.get("phase", 0)) == PitStopController.SERVICE_PHASE_FUEL:
		_clear_wheel_exchanges(true)
		_animate_tire_team(1.0)
		_animate_fuel_operator(1.0)
		return
	if int(status.get("phase", 0)) != PitStopController.SERVICE_PHASE_TIRES:
		return
	var progress := clampf(1.0 - float(status.get("tire_seconds_remaining", 0.0)) / tire_service_duration, 0.0, 1.0)
	_animate_tire_team(progress)
	_animate_fuel_operator(smoothstep(0.62, 0.96, progress))
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
		var outward_direction := pit_stop.get_pit_right() * (-1.0 if wheel_index % 2 == 0 else 1.0)
		var mechanic_name := wheel_name + "_wheel_change_mechanic"
		var mechanic := crew_members.get(mechanic_name) as Node3D
		if mechanic != null and crew_member_service_transforms.has(mechanic_name):
			var mechanic_approach := smoothstep(0.02, 0.18, progress) * (1.0 - smoothstep(0.86, 1.0, progress))
			var release_motion := smoothstep(0.15, 0.24, progress) * (1.0 - smoothstep(0.30, 0.40, progress))
			var installation_motion := smoothstep(0.52, 0.62, progress) * (1.0 - smoothstep(0.70, 0.84, progress))
			var work_motion := maxf(release_motion, installation_motion)
			var mechanic_transform: Transform3D = crew_member_service_transforms[mechanic_name]
			mechanic_transform.origin -= outward_direction * (0.30 * mechanic_approach + 0.07 * work_motion)
			mechanic_transform.origin += Vector3.UP * sin(progress * PI * 8.0) * 0.025 * mechanic_approach
			mechanic_transform.basis *= Basis(Vector3.RIGHT, 0.10 * work_motion)
			mechanic.global_transform = mechanic_transform
			var tool_pivot := wheel_change_tools.get(mechanic_name) as Node3D
			if tool_pivot != null:
				tool_pivot.transform = wheel_change_tool_resting_transforms[mechanic_name]
				tool_pivot.position += Vector3(0.0, 0.0, 0.07 * work_motion)
				tool_pivot.rotation.z += sin(progress * PI * 12.0) * 0.16 * work_motion
		var carrier_name := wheel_name + "_wheel_carrier"
		var carrier := crew_members.get(carrier_name) as Node3D
		if carrier != null and crew_member_service_transforms.has(carrier_name):
			var carrier_approach := smoothstep(0.12, 0.42, progress) * (1.0 - smoothstep(0.82, 1.0, progress))
			var carrier_transform: Transform3D = crew_member_service_transforms[carrier_name]
			carrier_transform.origin -= outward_direction * 0.62 * carrier_approach
			carrier_transform.origin += Vector3.UP * sin(progress * PI * 8.0) * 0.03 * carrier_approach
			carrier.global_transform = carrier_transform


func _animate_fuel_operator(approach_fraction: float) -> void:
	var fuel_operator := crew_members.get("fuel_hose_operator") as Node3D
	if fuel_operator == null:
		return
	fuel_operator.global_transform = fuel_operator_resting_transform.interpolate_with(
		fuel_operator_filling_transform, approach_fraction)
	fuel_operator.global_position += Vector3.UP * sin(approach_fraction * PI * 4.0) * 0.025 * sin(approach_fraction * PI)


func _on_service_completed() -> void:
	_clear_wheel_exchanges(true)
	_animate_tire_team(1.0)
	_animate_fuel_operator(1.0)
	fuel_operator_retreat_elapsed = 0.0


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
	wheel_exchanges.clear()
