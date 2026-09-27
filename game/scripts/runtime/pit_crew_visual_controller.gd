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

var pit_stop: PitStopController
var vehicle: Node3D
var crew_members: Dictionary = {}
var wheel_exchanges: Array[Dictionary] = []
var crew_root: Node3D
var tire_service_duration := 0.0


func configure(next_pit_stop: PitStopController, next_vehicle: Node3D) -> void:
	pit_stop = next_pit_stop
	vehicle = next_vehicle
	if pit_stop == null or vehicle == null or not pit_stop.is_configured:
		return
	pit_stop.crew_visibility_changed.connect(_on_crew_visibility_changed)
	pit_stop.service_started.connect(_on_service_started)
	pit_stop.service_progress.connect(_on_service_progress)
	pit_stop.service_completed.connect(_on_service_completed)
	_on_crew_visibility_changed(pit_stop.crew_visible)


func _on_crew_visibility_changed(should_be_visible: bool) -> void:
	if should_be_visible:
		if crew_root == null:
			_create_crew()
		return
	if pit_stop != null and pit_stop.is_servicing():
		return
	_clear_crew()


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


func _on_service_started(plan: Dictionary) -> void:
	if crew_root == null:
		_create_crew()
	if crew_root != null:
		var box_center: Vector3 = pit_stop.get_assigned_box().get("center", Vector3.ZERO)
		var vehicle_offset := vehicle.global_position - box_center
		crew_root.global_position = Vector3(vehicle_offset.x, 0.0, vehicle_offset.z)
	tire_service_duration = maxf(float(plan.get("tire_seconds", 0.0)), 0.001)
	_start_wheel_exchanges()


func _start_wheel_exchanges() -> void:
	_clear_wheel_exchanges()
	for wheel_index in WHEEL_NAMES.size():
		var wheel_name: String = WHEEL_NAMES[wheel_index]
		var original_wheel := vehicle.get_node_or_null(VEHICLE_WHEEL_PATHS[wheel_index]) as Node3D
		var carrier := crew_members.get(wheel_name + "_wheel_carrier") as Node3D
		if original_wheel == null or carrier == null:
			continue
		var replacement_wheel := carrier.find_child("CarriedWheel", true, false) as Node3D
		var wheel_scene := load(WHEEL_MODEL_DIRECTORY + WHEEL_MODEL_FILES[wheel_index]) as PackedScene
		if replacement_wheel == null or wheel_scene == null:
			continue
		var removed_wheel := wheel_scene.instantiate() as Node3D
		crew_root.add_child(removed_wheel)
		removed_wheel.global_transform = original_wheel.global_transform
		var starting_replacement_transform := replacement_wheel.global_transform
		replacement_wheel.reparent(crew_root, true)
		original_wheel.visible = false
		wheel_exchanges.append({
			"original": original_wheel,
			"removed": removed_wheel,
			"replacement": replacement_wheel,
			"starting_removed_transform": removed_wheel.global_transform,
			"starting_replacement_transform": starting_replacement_transform,
			"target_transform": original_wheel.global_transform,
			"outward_direction": pit_stop.get_pit_right() * (-1.0 if wheel_index % 2 == 0 else 1.0),
		})


func _on_service_progress(status: Dictionary) -> void:
	if wheel_exchanges.is_empty():
		return
	if int(status.get("phase", 0)) != PitStopController.SERVICE_PHASE_TIRES:
		_clear_wheel_exchanges()
		return
	var progress := clampf(1.0 - float(status.get("tire_seconds_remaining", 0.0)) / tire_service_duration, 0.0, 1.0)
	for exchange in wheel_exchanges:
		var removed_wheel: Node3D = exchange["removed"]
		var replacement_wheel: Node3D = exchange["replacement"]
		var starting_removed_transform: Transform3D = exchange["starting_removed_transform"]
		var starting_replacement_transform: Transform3D = exchange["starting_replacement_transform"]
		var target_transform: Transform3D = exchange["target_transform"]
		var outward_direction: Vector3 = exchange["outward_direction"]
		removed_wheel.global_transform = starting_removed_transform.translated(outward_direction * 0.85 * clampf(progress / 0.42, 0.0, 1.0))
		replacement_wheel.global_transform = starting_replacement_transform.interpolate_with(
			target_transform,
			clampf((progress - 0.42) / 0.58, 0.0, 1.0))


func _on_service_completed() -> void:
	_clear_wheel_exchanges()


func _clear_wheel_exchanges() -> void:
	for exchange in wheel_exchanges:
		var original_wheel: Node3D = exchange["original"]
		if is_instance_valid(original_wheel):
			original_wheel.visible = true
		var removed_wheel: Node3D = exchange["removed"]
		if is_instance_valid(removed_wheel):
			removed_wheel.queue_free()
		var replacement_wheel: Node3D = exchange["replacement"]
		if is_instance_valid(replacement_wheel):
			replacement_wheel.queue_free()
	wheel_exchanges.clear()


func _clear_crew() -> void:
	_clear_wheel_exchanges()
	crew_members.clear()
	if crew_root != null:
		crew_root.queue_free()
		crew_root = null
