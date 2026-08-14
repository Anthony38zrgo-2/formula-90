class_name RaceSession
extends Node3D

signal composition_ready(vehicle: Vehicle, track: Node3D, aids: DrivingAidsController)

const CAMERA_SCENE := preload("res://scenes/runtime/arcade_chase_camera_rig.tscn")
const AIDS_SCRIPT := preload("res://addons/formula90s/scripts/driving_aids.gd")

@export var config: RaceSessionConfig

var active_track: Node3D
var active_vehicle_root: Node3D
var active_vehicle: Vehicle
var driving_aids: DrivingAidsController

func _ready() -> void:
	compose(config)

func compose(next_config: RaceSessionConfig) -> bool:
	if next_config == null or not next_config.is_valid_config():
		push_error("RaceSession requires valid vehicle and track definitions.")
		return false
	_clear_composition()
	config = next_config
	active_track = config.selected_track.track_scene.instantiate() as Node3D
	active_track.name = "ActiveTrack"
	$TrackContainer.add_child(active_track)
	var spawn := _find_vehicle_spawn(active_track)
	if spawn == null:
		push_error("Track '%s' has no VehicleSpawn Marker3D." % config.selected_track.id)
		_clear_composition()
		return false
	active_vehicle_root = config.selected_vehicle.vehicle_scene.instantiate() as Node3D
	active_vehicle_root.name = "ActiveVehicle"
	$VehicleContainer.add_child(active_vehicle_root)
	active_vehicle_root.global_transform = spawn.global_transform
	active_vehicle = active_vehicle_root.get_node_or_null("VehicleRigidBody") as Vehicle
	if active_vehicle == null:
		push_error("Vehicle '%s' does not expose VehicleRigidBody." % config.selected_vehicle.id)
		_clear_composition()
		return false
	_add_runtime_systems()
	composition_ready.emit(active_vehicle, active_track, driving_aids)
	return true

func _find_vehicle_spawn(track: Node) -> Marker3D:
	var named := track.find_child("VehicleSpawn", true, false) as Marker3D
	if named != null:
		return named
	for candidate in get_tree().get_nodes_in_group("vehicle_spawn"):
		if candidate is Marker3D and track.is_ancestor_of(candidate):
			return candidate as Marker3D
	return null

func _add_runtime_systems() -> void:
	var camera := CAMERA_SCENE.instantiate() as Node3D
	camera.name = "CameraRig"
	camera.set("car_path", NodePath("../VehicleContainer/ActiveVehicle/VehicleRigidBody"))
	add_child(camera)
	driving_aids = AIDS_SCRIPT.new() as DrivingAidsController
	driving_aids.name = "DrivingAids"
	driving_aids.vehicle_node = active_vehicle
	add_child(driving_aids)

func _clear_composition() -> void:
	for container in [$TrackContainer, $VehicleContainer]:
		for child in container.get_children():
			container.remove_child(child)
			child.queue_free()
	for child_name in [&"CameraRig", &"DrivingAids"]:
		var child := get_node_or_null(NodePath(String(child_name)))
		if child != null:
			remove_child(child)
			child.queue_free()
	active_track = null
	active_vehicle_root = null
	active_vehicle = null
	driving_aids = null
