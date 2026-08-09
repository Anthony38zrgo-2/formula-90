extends Node

## Places the test vehicle from the PlayerSpawn marker authored by the Blender
## racetrack pipeline. This keeps spawn position/orientation owned by the track
## instead of duplicating coordinates in the Godot test scene.

@export var generated_track_path: NodePath = NodePath("Track/GeneratedTrack")
@export var vehicle_path: NodePath = NodePath("VehicleController/VehicleRigidBody")
@export var spawn_marker_name: StringName = &"PlayerSpawn"
@export var vehicle_vertical_offset_m := 0.55

func _ready() -> void:
	var track_root := get_node_or_null(generated_track_path)
	var vehicle := get_node_or_null(vehicle_path) as Node3D
	if track_root == null:
		push_error("Generated track root missing: %s" % generated_track_path)
		return
	if vehicle == null:
		push_error("Generated track spawn binder vehicle missing: %s" % vehicle_path)
		return

	var marker := track_root.find_child(String(spawn_marker_name), true, false) as Node3D
	if marker == null:
		push_error("Generated track has no %s marker." % spawn_marker_name)
		return

	var spawn_transform := marker.global_transform
	spawn_transform.origin.y += vehicle_vertical_offset_m
	vehicle.global_transform = spawn_transform

	if vehicle is RigidBody3D:
		vehicle.linear_velocity = Vector3.ZERO
		vehicle.angular_velocity = Vector3.ZERO
		vehicle.sleeping = false
