extends RefCounted

var vehicle: Node3D
var configuration: CockpitCameraConfiguration
var seat_reference_local_position := Vector3.ZERO
var angular_displacement := Vector2.ZERO
var angular_velocity := Vector2.ZERO
var component_displacement := Vector3.ZERO
var component_velocity := Vector3.ZERO
var filtered_bump_acceleration := Vector2.ZERO
var longitudinal_compensation_gravity := 0.0
var filtered_longitudinal_acceleration := 0.0
var gear_shift_motion_envelope := 0.0
var previous_gear := 0
var has_previous_gear := false
var suspension_compression_baseline := Vector4.ZERO
var suspension_compression_trend := Vector4.ZERO
var has_suspension_compression_baseline := false
var suspension_bump_acceleration := Vector2.ZERO
var suspension_pitch_acceleration := 0.0
var vertical_displacement_meters := 0.0
var vertical_velocity_meters_per_second := 0.0
var bump_response_envelope := Vector2.ZERO
var positional_correction_world := Vector3.ZERO
var previous_seat_position := Vector3.ZERO
var has_previous_seat_position := false
var previous_linear_velocity := Vector3.ZERO
var previous_angular_velocity := Vector3.ZERO
var seat_acceleration_baseline := Vector3.ZERO
var has_previous_kinematics := false
var road_bump_confirmation := 0.0
var has_road_bump_confirmation := false
var previous_road_contact_center := Vector3.ZERO
var previous_road_up_direction := Vector3.UP
var has_previous_road_contact := false
var road_reference_angles := Vector2.ZERO
var velocity_alignment_angle := 0.0
var current_vehicle_transform := Transform3D.IDENTITY
var previous_render_state: Dictionary = {}
var current_render_state: Dictionary = {}
var update_count := 0

func update_from_vehicle(elapsed_seconds: float) -> void:
	if not is_instance_valid(vehicle) or configuration == null:
		return
	var telemetry: Variant = vehicle.call("get_telemetry_snapshot") if vehicle.has_method("get_telemetry_snapshot") else {}
	if not telemetry is Dictionary:
		telemetry = {}
	var road_up_direction := read_road_up_direction(elapsed_seconds)
	update_sample(telemetry, vehicle.global_transform, road_up_direction, elapsed_seconds, road_bump_confirmation if has_road_bump_confirmation else -1.0)

func read_road_up_direction(elapsed_seconds: float) -> Vector3:
	if vehicle.has_method("get_raycast_list"):
		var raycasts: Variant = vehicle.call("get_raycast_list")
		if raycasts is Array and raycasts.size() == 12:
			var surface_up_direction := Vector3.ZERO
			var contact_positions: Array[Vector3] = []
			for raycast_index in [1, 4, 7, 10]:
				var raycast := raycasts[raycast_index] as RayCast3D
				if is_instance_valid(raycast) and raycast.is_colliding():
					var surface_normal := raycast.get_collision_normal()
					if surface_normal.is_finite() and surface_normal.dot(Vector3.UP) > 0.0:
						surface_up_direction += surface_normal
						contact_positions.append(raycast.get_collision_point())
			has_road_bump_confirmation = true
			var desired_confirmation := 0.0
			if surface_up_direction.length_squared() > 0.0001:
				surface_up_direction = surface_up_direction.normalized()
				if contact_positions.size() == 4:
					var contact_center := Vector3.ZERO
					for contact_position in contact_positions:
						contact_center += contact_position * 0.25
					var surface_discontinuity := 0.0
					for contact_position in contact_positions:
						surface_discontinuity = maxf(surface_discontinuity, absf((contact_position - contact_center).dot(surface_up_direction)))
					if has_previous_road_contact:
						surface_discontinuity = maxf(surface_discontinuity, absf((contact_center - previous_road_contact_center).dot(previous_road_up_direction)))
					previous_road_contact_center = contact_center
					previous_road_up_direction = surface_up_direction
					has_previous_road_contact = true
					desired_confirmation = smoothstep(configuration.minimum_suspension_bump_displacement_meters, configuration.road_bump_full_confirmation_displacement_meters, surface_discontinuity)
				else:
					has_previous_road_contact = false
				road_bump_confirmation = lerpf(road_bump_confirmation, desired_confirmation, 1.0 - exp(-elapsed_seconds / configuration.bump_strength_filter_time_seconds))
				return surface_up_direction
			has_previous_road_contact = false
			road_bump_confirmation = lerpf(road_bump_confirmation, 0.0, 1.0 - exp(-elapsed_seconds / configuration.bump_strength_filter_time_seconds))
	return vehicle.global_basis.y.normalized()

func update_sample(telemetry: Dictionary, vehicle_transform: Transform3D, road_up_direction: Vector3, elapsed_seconds: float, confirmed_road_bump: float = -1.0) -> void:
	if not vehicle_transform.is_finite() or not road_up_direction.is_finite() or road_up_direction.length_squared() < 0.0001 or not is_finite(elapsed_seconds) or elapsed_seconds <= 0.0:
		return
	var position_discontinuity := not current_render_state.is_empty() and vehicle_transform.origin.distance_to(current_vehicle_transform.origin) > configuration.teleport_reset_distance_meters
	if position_discontinuity or elapsed_seconds > configuration.maximum_elapsed_seconds:
		reset_motion()
		current_render_state.clear()
		previous_render_state.clear()
	current_vehicle_transform = vehicle_transform.orthonormalized()
	previous_render_state = current_render_state.duplicate()
	update_suspension_bump_motion(telemetry.get("wheel_compressions", []), elapsed_seconds)
	if confirmed_road_bump >= 0.0:
		suspension_bump_acceleration *= clampf(confirmed_road_bump, 0.0, 1.0)
		suspension_pitch_acceleration *= clampf(confirmed_road_bump, 0.0, 1.0)
	update_seat_acceleration(telemetry, elapsed_seconds)
	update_road_reference(road_up_direction, elapsed_seconds)
	update_velocity_alignment(telemetry.get("linear_velocity", Vector3.ZERO), elapsed_seconds)
	update_gear_shift_motion(telemetry.get("gear"))
	var longitudinal_acceleration := float(telemetry.get("long_g", 0.0))
	if not is_finite(longitudinal_acceleration):
		longitudinal_acceleration = 0.0
	update_inertial_motion(Vector3(suspension_bump_acceleration.x, suspension_bump_acceleration.y, longitudinal_acceleration), elapsed_seconds)
	update_positional_correction(current_vehicle_transform * seat_reference_local_position, current_vehicle_transform.basis, elapsed_seconds, bump_response_envelope)
	current_render_state = {
		"vehicle_transform": current_vehicle_transform,
		"angular_displacement": angular_displacement,
		"road_reference_angles": road_reference_angles,
		"velocity_alignment_angle": velocity_alignment_angle,
		"positional_correction_world": positional_correction_world,
		"vertical_displacement_meters": vertical_displacement_meters
	}
	if previous_render_state.is_empty():
		previous_render_state = current_render_state.duplicate()
	update_count += 1

func update_gear_shift_motion(gear_value: Variant) -> void:
	if not (gear_value is int or gear_value is float) or not is_finite(float(gear_value)):
		has_previous_gear = false
		return
	var current_gear := int(gear_value)
	if has_previous_gear and current_gear != previous_gear:
		gear_shift_motion_envelope = 1.0
	previous_gear = current_gear
	has_previous_gear = true

func update_seat_acceleration(telemetry: Dictionary, elapsed_seconds: float) -> void:
	var linear_velocity: Variant = telemetry.get("linear_velocity")
	var angular_velocity_sample: Variant = telemetry.get("angular_velocity")
	if not linear_velocity is Vector3 or not angular_velocity_sample is Vector3 or not linear_velocity.is_finite() or not angular_velocity_sample.is_finite():
		has_previous_kinematics = false
		return
	if has_previous_kinematics:
		var angular_acceleration: Vector3 = (angular_velocity_sample - previous_angular_velocity) / elapsed_seconds
		var seat_offset_world := current_vehicle_transform.basis * seat_reference_local_position
		var seat_acceleration: Vector3 = (linear_velocity - previous_linear_velocity) / elapsed_seconds + angular_acceleration.cross(seat_offset_world) + angular_velocity_sample.cross(angular_velocity_sample.cross(seat_offset_world))
		seat_acceleration_baseline = seat_acceleration_baseline.lerp(seat_acceleration, 1.0 - exp(-elapsed_seconds / configuration.seat_acceleration_filter_time_seconds))
		var local_transient := current_vehicle_transform.basis.inverse() * (seat_acceleration - seat_acceleration_baseline) / 9.80665
		var suspension_confirmation := clampf(absf(suspension_bump_acceleration.y) / configuration.positional_bump_full_response_gravity, 0.0, 1.0)
		suspension_bump_acceleration.y = lerpf(suspension_bump_acceleration.y, local_transient.y * suspension_confirmation, configuration.seat_acceleration_blend_strength)
	previous_linear_velocity = linear_velocity
	previous_angular_velocity = angular_velocity_sample
	has_previous_kinematics = true

func read_vehicle_acceleration(elapsed_seconds: float) -> Vector3:
	if not is_instance_valid(vehicle) or not vehicle.has_method("get_telemetry_snapshot"):
		reset_suspension_bump_motion()
		return Vector3.ZERO
	var telemetry: Variant = vehicle.call("get_telemetry_snapshot")
	if not telemetry is Dictionary:
		reset_suspension_bump_motion()
		return Vector3.ZERO
	update_suspension_bump_motion(telemetry.get("wheel_compressions", []), elapsed_seconds)
	var acceleration := Vector3(suspension_bump_acceleration.x, suspension_bump_acceleration.y, float(telemetry.get("long_g", 0.0)))
	return acceleration.clamp(Vector3.ONE * -configuration.maximum_telemetry_gravity, Vector3.ONE * configuration.maximum_telemetry_gravity) if acceleration.is_finite() else Vector3.ZERO

func update_suspension_bump_motion(wheel_compressions: Variant, elapsed_seconds: float) -> void:
	if not (wheel_compressions is Array or wheel_compressions is PackedFloat64Array or wheel_compressions is PackedFloat32Array) or wheel_compressions.size() != 4:
		reset_suspension_bump_motion()
		return
	var compression := Vector4.ZERO
	for wheel_index in range(4):
		var compression_value: Variant = wheel_compressions[wheel_index]
		if not (compression_value is float or compression_value is int) or not is_finite(float(compression_value)):
			reset_suspension_bump_motion()
			return
		compression[wheel_index] = float(compression_value) * 0.001
	if not has_suspension_compression_baseline:
		suspension_compression_baseline = compression
		has_suspension_compression_baseline = true
		return
	var baseline_decay := exp(-elapsed_seconds / configuration.suspension_bump_separation_time_seconds)
	var previous_residual := compression - suspension_compression_baseline
	suspension_compression_baseline = compression - previous_residual * baseline_decay
	suspension_compression_trend = (suspension_compression_trend + previous_residual * elapsed_seconds / configuration.suspension_bump_separation_time_seconds) * baseline_decay
	var bump_displacement := compression - suspension_compression_baseline - suspension_compression_trend
	for wheel_index in range(4):
		bump_displacement[wheel_index] = signf(bump_displacement[wheel_index]) * maxf(0.0, absf(bump_displacement[wheel_index]) - configuration.minimum_suspension_bump_displacement_meters)
	var left_displacement := (bump_displacement.x + bump_displacement.z) * 0.5
	var right_displacement := (bump_displacement.y + bump_displacement.w) * 0.5
	var front_displacement := (bump_displacement.x + bump_displacement.y) * 0.5
	var rear_displacement := (bump_displacement.z + bump_displacement.w) * 0.5
	suspension_bump_acceleration = Vector2(left_displacement - right_displacement, -(left_displacement + right_displacement) * 0.5) * configuration.suspension_bump_response_gravity_per_meter
	suspension_bump_acceleration = suspension_bump_acceleration.clamp(Vector2.ONE * -configuration.maximum_telemetry_gravity, Vector2.ONE * configuration.maximum_telemetry_gravity)
	suspension_pitch_acceleration = clampf((front_displacement - rear_displacement) * configuration.suspension_bump_response_gravity_per_meter, -configuration.maximum_telemetry_gravity, configuration.maximum_telemetry_gravity)

func reset_suspension_bump_motion() -> void:
	suspension_compression_baseline = Vector4.ZERO
	suspension_compression_trend = Vector4.ZERO
	has_suspension_compression_baseline = false
	suspension_bump_acceleration = Vector2.ZERO
	suspension_pitch_acceleration = 0.0

func reset_positional_motion() -> void:
	positional_correction_world = Vector3.ZERO
	has_previous_seat_position = false
	if not current_render_state.is_empty():
		current_render_state["positional_correction_world"] = Vector3.ZERO
		previous_render_state["positional_correction_world"] = Vector3.ZERO

func reset_motion() -> void:
	angular_displacement = Vector2.ZERO
	angular_velocity = Vector2.ZERO
	component_displacement = Vector3.ZERO
	component_velocity = Vector3.ZERO
	filtered_bump_acceleration = Vector2.ZERO
	longitudinal_compensation_gravity = 0.0
	filtered_longitudinal_acceleration = 0.0
	gear_shift_motion_envelope = 0.0
	has_previous_gear = false
	vertical_displacement_meters = 0.0
	vertical_velocity_meters_per_second = 0.0
	bump_response_envelope = Vector2.ZERO
	positional_correction_world = Vector3.ZERO
	has_previous_seat_position = false
	has_previous_kinematics = false
	seat_acceleration_baseline = Vector3.ZERO
	has_previous_road_contact = false
	road_bump_confirmation = 0.0
	velocity_alignment_angle = 0.0
	reset_suspension_bump_motion()
	if not current_render_state.is_empty():
		current_render_state["angular_displacement"] = Vector2.ZERO
		current_render_state["positional_correction_world"] = Vector3.ZERO
		current_render_state["velocity_alignment_angle"] = 0.0
		current_render_state["vertical_displacement_meters"] = 0.0
		previous_render_state = current_render_state.duplicate()

func update_inertial_motion(acceleration: Vector3, elapsed_seconds: float) -> void:
	if configuration.force_response_strength == 0.0:
		reset_motion()
		return
	acceleration = acceleration.clamp(Vector3.ONE * -configuration.maximum_telemetry_gravity, Vector3.ONE * configuration.maximum_telemetry_gravity)
	var component_limits := Vector3(deg_to_rad(configuration.maximum_pitch_degrees), deg_to_rad(configuration.maximum_suspension_pitch_degrees), deg_to_rad(configuration.maximum_roll_degrees))
	var angular_frequencies := Vector3(configuration.response_frequency_hertz, configuration.vertical_response_frequency_hertz, configuration.lateral_response_frequency_hertz) * TAU
	var damping_ratios := Vector3(configuration.damping_ratio, configuration.vertical_damping_ratio, configuration.lateral_damping_ratio)
	var remaining_seconds := elapsed_seconds
	while remaining_seconds > 0.0:
		var integration_seconds := minf(remaining_seconds, configuration.maximum_integration_step_seconds)
		filtered_longitudinal_acceleration = filter_acceleration(filtered_longitudinal_acceleration, acceleration.z, configuration.longitudinal_acceleration_filter_time_seconds, integration_seconds)
		var bounded_longitudinal_acceleration := soft_limit(filtered_longitudinal_acceleration, configuration.maximum_longitudinal_acceleration_gravity)
		var shift_response_strength := 1.0 - configuration.gear_shift_motion_reduction_strength * gear_shift_motion_envelope
		gear_shift_motion_envelope *= exp(-integration_seconds / configuration.gear_shift_motion_recovery_time_seconds)
		var compensation_time := configuration.longitudinal_compensation_time_seconds
		if bounded_longitudinal_acceleration * longitudinal_compensation_gravity < 0.0 or absf(bounded_longitudinal_acceleration) < absf(longitudinal_compensation_gravity):
			compensation_time = configuration.longitudinal_release_time_seconds
		longitudinal_compensation_gravity = lerpf(longitudinal_compensation_gravity, bounded_longitudinal_acceleration, 1.0 - exp(-integration_seconds / compensation_time))
		var compensation_in_force_direction := maxf(0.0, longitudinal_compensation_gravity * signf(bounded_longitudinal_acceleration)) * configuration.longitudinal_compensation_strength
		var longitudinal_acceleration := signf(bounded_longitudinal_acceleration) * maxf(0.0, absf(bounded_longitudinal_acceleration) - compensation_in_force_direction) * shift_response_strength
		filtered_bump_acceleration.x = filter_acceleration(filtered_bump_acceleration.x, acceleration.x, configuration.lateral_acceleration_filter_time_seconds, integration_seconds)
		filtered_bump_acceleration.y = filter_acceleration(filtered_bump_acceleration.y, acceleration.y, configuration.vertical_acceleration_filter_time_seconds, integration_seconds)
		var desired_components := Vector3(
			deg_to_rad(-longitudinal_acceleration * configuration.longitudinal_response_degrees_per_gravity) * configuration.longitudinal_force_response_strength,
			deg_to_rad(suspension_pitch_acceleration * configuration.suspension_pitch_response_degrees_per_gravity) * configuration.vertical_bump_response_strength,
			deg_to_rad(-filtered_bump_acceleration.x * configuration.lateral_response_degrees_per_gravity) * configuration.lateral_bump_response_strength
		) * configuration.force_response_strength
		for axis_index in range(3):
			desired_components[axis_index] = soft_limit(desired_components[axis_index], component_limits[axis_index])
		var component_acceleration := angular_frequencies * angular_frequencies * (desired_components - component_displacement) - 2.0 * damping_ratios * angular_frequencies * component_velocity
		component_velocity += component_acceleration * integration_seconds
		component_displacement += component_velocity * integration_seconds
		var desired_vertical_displacement := soft_limit(-filtered_bump_acceleration.y * configuration.vertical_bump_response_meters_per_gravity * configuration.vertical_bump_response_strength * configuration.force_response_strength, configuration.maximum_vertical_correction_meters)
		var vertical_angular_frequency := configuration.vertical_response_frequency_hertz * TAU
		vertical_velocity_meters_per_second += (vertical_angular_frequency * vertical_angular_frequency * (desired_vertical_displacement - vertical_displacement_meters) - 2.0 * configuration.vertical_damping_ratio * vertical_angular_frequency * vertical_velocity_meters_per_second) * integration_seconds
		vertical_displacement_meters += vertical_velocity_meters_per_second * integration_seconds
		var desired_envelope := Vector2(absf(filtered_bump_acceleration.x) * configuration.lateral_bump_response_strength, absf(filtered_bump_acceleration.y) * configuration.vertical_bump_response_strength) / configuration.positional_bump_full_response_gravity
		bump_response_envelope = bump_response_envelope.lerp(desired_envelope.clamp(Vector2.ZERO, Vector2.ONE), 1.0 - exp(-integration_seconds / configuration.bump_strength_filter_time_seconds))
		remaining_seconds = maxf(0.0, remaining_seconds - integration_seconds)
	angular_displacement = Vector2(soft_limit(component_displacement.x + component_displacement.y, deg_to_rad(configuration.maximum_pitch_degrees)), soft_limit(component_displacement.z, deg_to_rad(configuration.maximum_roll_degrees)))
	angular_velocity = Vector2(component_velocity.x + component_velocity.y, component_velocity.z)

func soft_limit(value: float, maximum_magnitude: float) -> float:
	return maximum_magnitude * tanh(value / maximum_magnitude) if maximum_magnitude > 0.0 else 0.0

func filter_acceleration(previous_acceleration: float, current_acceleration: float, filter_time_seconds: float, elapsed_seconds: float) -> float:
	return current_acceleration if filter_time_seconds <= 0.0 else lerpf(previous_acceleration, current_acceleration, 1.0 - exp(-elapsed_seconds / filter_time_seconds))

func make_heading_basis(vehicle_basis: Basis, up_direction: Vector3 = Vector3.UP) -> Basis:
	var forward_direction := -vehicle_basis.z
	forward_direction -= up_direction * forward_direction.dot(up_direction)
	if forward_direction.length_squared() < 0.0001:
		forward_direction = Vector3.FORWARD
	forward_direction = forward_direction.normalized()
	return Basis(up_direction.cross(forward_direction).normalized(), up_direction, forward_direction).orthonormalized()

func update_road_reference(road_up_direction: Vector3, elapsed_seconds: float) -> void:
	var flat_heading := make_heading_basis(current_vehicle_transform.basis)
	var road_heading := make_heading_basis(current_vehicle_transform.basis, road_up_direction.normalized())
	var road_angles := (flat_heading.inverse() * road_heading).get_euler()
	if current_render_state.is_empty():
		road_reference_angles = Vector2(road_angles.x, road_angles.z)
		return
	road_reference_angles.x = lerp_angle(road_reference_angles.x, road_angles.x, 1.0 - exp(-elapsed_seconds / configuration.road_pitch_follow_time_seconds))
	road_reference_angles.y = lerp_angle(road_reference_angles.y, road_angles.z, 1.0 - exp(-elapsed_seconds / configuration.road_roll_follow_time_seconds))

func update_velocity_alignment(linear_velocity: Variant, elapsed_seconds: float) -> void:
	var desired_alignment := 0.0
	if linear_velocity is Vector3 and linear_velocity.is_finite() and configuration.velocity_alignment_strength > 0.0:
		var local_velocity: Vector3 = current_vehicle_transform.basis.inverse() * linear_velocity
		var forward_speed := -local_velocity.z
		if forward_speed > configuration.minimum_velocity_alignment_speed_meters_per_second:
			var speed_strength := smoothstep(configuration.minimum_velocity_alignment_speed_meters_per_second, configuration.full_velocity_alignment_speed_meters_per_second, forward_speed)
			desired_alignment = soft_limit(-atan2(local_velocity.x, forward_speed) * configuration.velocity_alignment_strength * speed_strength, deg_to_rad(configuration.maximum_velocity_alignment_degrees))
	velocity_alignment_angle = lerp_angle(velocity_alignment_angle, desired_alignment, 1.0 - exp(-elapsed_seconds / configuration.velocity_alignment_filter_time_seconds))

func update_positional_correction(anchor_position: Vector3, vehicle_basis: Basis, elapsed_seconds: float, bump_strength: Vector2) -> void:
	var anchor_movement := anchor_position - previous_seat_position
	var should_reset := not has_previous_seat_position or anchor_movement.length() > configuration.teleport_reset_distance_meters or elapsed_seconds > configuration.maximum_elapsed_seconds or configuration.positional_stabilization_strength == 0.0
	previous_seat_position = anchor_position
	has_previous_seat_position = true
	if should_reset:
		positional_correction_world = Vector3.ZERO
		return
	if elapsed_seconds <= 0.0:
		return
	var road_basis := make_heading_basis(vehicle_basis) * Basis.from_euler(Vector3(road_reference_angles.x, 0.0, road_reference_angles.y))
	anchor_movement -= road_basis.z * anchor_movement.dot(road_basis.z)
	var lateral_direction := Vector3(vehicle_basis.x.x, 0.0, vehicle_basis.x.z).normalized()
	var lateral_decay := exp(-elapsed_seconds / configuration.positional_filter_time_seconds)
	var vertical_decay := exp(-elapsed_seconds / configuration.vertical_positional_filter_time_seconds)
	var lateral_response := configuration.positional_filter_time_seconds * (1.0 - lateral_decay) / elapsed_seconds
	var vertical_response := configuration.vertical_positional_filter_time_seconds * (1.0 - vertical_decay) / elapsed_seconds
	var lateral_correction := positional_correction_world.dot(lateral_direction) * lateral_decay - anchor_movement.dot(lateral_direction) * lateral_response * bump_strength.x
	var vertical_correction := positional_correction_world.y * vertical_decay - anchor_movement.y * vertical_response * bump_strength.y
	positional_correction_world = lateral_direction * clampf(lateral_correction, -configuration.maximum_lateral_correction_meters, configuration.maximum_lateral_correction_meters) + Vector3.UP * clampf(vertical_correction, -configuration.maximum_vertical_correction_meters, configuration.maximum_vertical_correction_meters)

func get_render_state(interpolation_fraction: float) -> Dictionary:
	if current_render_state.is_empty():
		return {"vehicle_transform": current_vehicle_transform, "angular_displacement": angular_displacement, "road_reference_angles": road_reference_angles, "velocity_alignment_angle": velocity_alignment_angle, "positional_correction_world": positional_correction_world, "vertical_displacement_meters": vertical_displacement_meters}
	var fraction := clampf(interpolation_fraction, 0.0, 1.0)
	var rendered_correction: Vector3 = previous_render_state["positional_correction_world"].lerp(current_render_state["positional_correction_world"], fraction)
	return {
		"vehicle_transform": previous_render_state["vehicle_transform"].interpolate_with(current_render_state["vehicle_transform"], fraction),
		"angular_displacement": previous_render_state["angular_displacement"].lerp(current_render_state["angular_displacement"], fraction),
		"road_reference_angles": previous_render_state["road_reference_angles"].lerp(current_render_state["road_reference_angles"], fraction),
		"velocity_alignment_angle": lerp_angle(previous_render_state["velocity_alignment_angle"], current_render_state["velocity_alignment_angle"], fraction),
		"positional_correction_world": rendered_correction,
		"vertical_displacement_meters": lerpf(previous_render_state["vertical_displacement_meters"], current_render_state["vertical_displacement_meters"], fraction)
	}
