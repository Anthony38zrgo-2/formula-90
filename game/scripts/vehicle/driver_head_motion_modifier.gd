extends SkeletonModifier3D

const DRIVER_MOTION_STATE_SCRIPT := preload("res://scripts/camera/cockpit_driver_motion_state.gd")

var vehicle: Node3D
var configuration: CockpitCameraConfiguration
var driver_eye_point: Node3D
var motion_state: RefCounted
var head_bone_index: int = -1
var neck_bone_index: int = -1
var angular_displacement := Vector2.ZERO
var angular_velocity := Vector2.ZERO
var previous_physics_frame: int = -1
var has_seat_reference := false

func _ready() -> void:
	motion_state = DRIVER_MOTION_STATE_SCRIPT.new()
	motion_state.set("vehicle", vehicle)
	motion_state.set("configuration", configuration)
	motion_state.set("current_vehicle_transform", vehicle.global_transform)
	process_physics_priority = 20

func _physics_process(elapsed_seconds: float) -> void:
	var current_physics_frame := Engine.get_physics_frames()
	if current_physics_frame == previous_physics_frame or not is_instance_valid(vehicle) or configuration == null:
		return
	previous_physics_frame = current_physics_frame
	if not has_seat_reference and is_instance_valid(driver_eye_point):
		motion_state.set("seat_reference_local_position", vehicle.global_transform.affine_inverse() * driver_eye_point.global_position)
		has_seat_reference = true
	motion_state.call("update_from_vehicle", elapsed_seconds)

func _process_modification_with_delta(_elapsed_seconds: float) -> void:
	var skeleton := get_skeleton()
	if skeleton == null or not is_instance_valid(vehicle) or configuration == null or motion_state == null:
		return
	if head_bone_index < 0 or neck_bone_index < 0:
		head_bone_index = skeleton.find_bone("mixamorig_Head")
		neck_bone_index = skeleton.find_bone("mixamorig_Neck")
		if head_bone_index < 0:
			head_bone_index = skeleton.find_bone("mixamorig:Head")
		if neck_bone_index < 0:
			neck_bone_index = skeleton.find_bone("mixamorig:Neck")
		if head_bone_index < 0 or neck_bone_index < 0:
			push_error("Cockpit head motion requires head and neck bones.")
			active = false
			return
	var rendered_state: Dictionary = motion_state.call("get_render_state", Engine.get_physics_interpolation_fraction())
	angular_displacement = rendered_state["angular_displacement"]
	angular_velocity = motion_state.get("angular_velocity")
	apply_head_and_neck_pose(skeleton, rendered_state)

func reset_motion() -> void:
	if motion_state != null:
		motion_state.call("reset_motion")
	angular_displacement = Vector2.ZERO
	angular_velocity = Vector2.ZERO

func apply_head_and_neck_pose(skeleton: Skeleton3D, rendered_state: Dictionary) -> void:
	var head_pose := skeleton.get_bone_global_pose(head_bone_index)
	var neck_pose := skeleton.get_bone_global_pose(neck_bone_index)
	var neutral_head_world_basis := (skeleton.global_basis * head_pose.basis).orthonormalized()
	var flat_heading: Basis = motion_state.call("make_heading_basis", vehicle.global_basis)
	var neutral_angles := (flat_heading.inverse() * neutral_head_world_basis).get_euler()
	var reference_angles: Vector2 = rendered_state["road_reference_angles"]
	var stabilized_pitch := lerp_angle(neutral_angles.x, reference_angles.x, configuration.horizon_stabilization_strength * configuration.pitch_horizon_stabilization_strength)
	var stabilized_roll := lerp_angle(neutral_angles.z, reference_angles.y, configuration.horizon_stabilization_strength * configuration.roll_horizon_stabilization_strength)
	var stabilized_basis := flat_heading * Basis.from_euler(Vector3(stabilized_pitch, 0.0, stabilized_roll))
	var neutral_rotation := neutral_head_world_basis.get_rotation_quaternion()
	var stabilized_rotation := stabilized_basis.get_rotation_quaternion()
	var correction_angle := neutral_rotation.angle_to(stabilized_rotation)
	var correction_fraction := minf(1.0, deg_to_rad(configuration.maximum_horizon_correction_degrees) / maxf(correction_angle, 0.0001))
	stabilized_basis = Basis(neutral_rotation.slerp(stabilized_rotation, correction_fraction))
	var desired_head_world_basis := Basis(Vector3.UP, float(rendered_state["velocity_alignment_angle"])) * stabilized_basis * Basis(Vector3.BACK, angular_displacement.y) * Basis(Vector3.RIGHT, angular_displacement.x)
	var total_rotation := (desired_head_world_basis * neutral_head_world_basis.inverse()).get_rotation_quaternion()
	var neck_rotation := Quaternion.IDENTITY.slerp(total_rotation, configuration.neck_rotation_fraction)
	neck_pose.basis = skeleton.global_basis.inverse() * Basis(neck_rotation) * skeleton.global_basis * neck_pose.basis
	skeleton.set_bone_global_pose(neck_bone_index, neck_pose)
	head_pose = skeleton.get_bone_global_pose(head_bone_index)
	head_pose.basis = skeleton.global_basis.inverse() * desired_head_world_basis
	skeleton.set_bone_global_pose(head_bone_index, head_pose)
