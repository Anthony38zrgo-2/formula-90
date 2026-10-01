extends SkeletonModifier3D

var vehicle: Node3D
var configuration: CockpitCameraConfiguration
var head_bone_index: int = -1
var neck_bone_index: int = -1
var angular_displacement := Vector2.ZERO
var angular_velocity := Vector2.ZERO
var previous_vehicle_position := Vector3.ZERO
var has_previous_vehicle_position := false

func _process_modification_with_delta(elapsed_seconds: float) -> void:
	var skeleton := get_skeleton()
	if skeleton == null or not is_instance_valid(vehicle) or configuration == null:
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
	var position_discontinuity := has_previous_vehicle_position and vehicle.global_position.distance_to(previous_vehicle_position) > configuration.teleport_reset_distance_meters
	previous_vehicle_position = vehicle.global_position
	has_previous_vehicle_position = true
	if position_discontinuity or elapsed_seconds > configuration.maximum_elapsed_seconds or configuration.force_response_strength == 0.0:
		reset_motion()
	else:
		update_inertial_motion(read_vehicle_acceleration(), maxf(elapsed_seconds, 0.0))
	apply_head_and_neck_pose(skeleton)

func read_vehicle_acceleration() -> Vector3:
	if not vehicle.has_method("get_telemetry_snapshot"):
		return Vector3.ZERO
	var telemetry: Variant = vehicle.call("get_telemetry_snapshot")
	if not telemetry is Dictionary:
		return Vector3.ZERO
	var acceleration := Vector3(float(telemetry.get("lat_g", 0.0)), float(telemetry.get("vert_g", 0.0)), float(telemetry.get("long_g", 0.0)))
	if not acceleration.is_finite():
		reset_motion()
		return Vector3.ZERO
	return acceleration.clamp(Vector3.ONE * -configuration.maximum_telemetry_gravity, Vector3.ONE * configuration.maximum_telemetry_gravity)

func reset_motion() -> void:
	angular_displacement = Vector2.ZERO
	angular_velocity = Vector2.ZERO

func update_inertial_motion(acceleration: Vector3, elapsed_seconds: float) -> void:
	var desired_displacement := Vector2(
		deg_to_rad(-acceleration.z * configuration.longitudinal_response_degrees_per_gravity + acceleration.y * configuration.vertical_response_degrees_per_gravity),
		deg_to_rad(-acceleration.x * configuration.lateral_response_degrees_per_gravity)
	) * configuration.force_response_strength
	var displacement_limits := Vector2(deg_to_rad(configuration.maximum_pitch_degrees), deg_to_rad(configuration.maximum_roll_degrees))
	desired_displacement = desired_displacement.clamp(-displacement_limits, displacement_limits)
	var angular_frequency := TAU * configuration.response_frequency_hertz
	var remaining_seconds := elapsed_seconds
	while remaining_seconds > 0.0:
		var integration_seconds := minf(remaining_seconds, configuration.maximum_integration_step_seconds)
		var angular_acceleration := angular_frequency * angular_frequency * (desired_displacement - angular_displacement) - 2.0 * configuration.damping_ratio * angular_frequency * angular_velocity
		angular_velocity += angular_acceleration * integration_seconds
		angular_displacement += angular_velocity * integration_seconds
		for axis_index in range(2):
			if absf(angular_displacement[axis_index]) > displacement_limits[axis_index]:
				angular_displacement[axis_index] = clampf(angular_displacement[axis_index], -displacement_limits[axis_index], displacement_limits[axis_index])
				angular_velocity[axis_index] = 0.0
		remaining_seconds = maxf(0.0, remaining_seconds - integration_seconds)

func apply_head_and_neck_pose(skeleton: Skeleton3D) -> void:
	var head_pose := skeleton.get_bone_global_pose(head_bone_index)
	var neck_pose := skeleton.get_bone_global_pose(neck_bone_index)
	var neutral_head_world_basis := (skeleton.global_basis * head_pose.basis).orthonormalized()
	var forward_direction := -vehicle.global_basis.z
	forward_direction.y = 0.0
	if forward_direction.length_squared() < 0.0001:
		forward_direction = neutral_head_world_basis.z
		forward_direction.y = 0.0
	if forward_direction.length_squared() < 0.0001:
		forward_direction = Vector3.FORWARD
	forward_direction = forward_direction.normalized()
	var upright_head_world_basis := Basis(Vector3.UP.cross(forward_direction).normalized(), Vector3.UP, forward_direction)
	var neutral_rotation := neutral_head_world_basis.get_rotation_quaternion()
	var upright_rotation := upright_head_world_basis.get_rotation_quaternion()
	var correction_angle := neutral_rotation.angle_to(upright_rotation)
	var correction_fraction := minf(configuration.horizon_stabilization_strength, deg_to_rad(configuration.maximum_horizon_correction_degrees) / maxf(correction_angle, 0.0001))
	var stabilized_basis := Basis(neutral_rotation.slerp(upright_rotation, correction_fraction))
	var desired_head_world_basis := stabilized_basis * Basis(Vector3.BACK, angular_displacement.y) * Basis(Vector3.RIGHT, angular_displacement.x)
	var total_rotation := (desired_head_world_basis * neutral_head_world_basis.inverse()).get_rotation_quaternion()
	var neck_rotation := Quaternion.IDENTITY.slerp(total_rotation, configuration.neck_rotation_fraction)
	neck_pose.basis = skeleton.global_basis.inverse() * Basis(neck_rotation) * skeleton.global_basis * neck_pose.basis
	skeleton.set_bone_global_pose(neck_bone_index, neck_pose)
	head_pose = skeleton.get_bone_global_pose(head_bone_index)
	head_pose.basis = skeleton.global_basis.inverse() * desired_head_world_basis
	skeleton.set_bone_global_pose(head_bone_index, head_pose)
