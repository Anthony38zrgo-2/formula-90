class_name CockpitCameraConfiguration
extends RefCounted

var field_of_view_degrees: float
var viewpoint_elevation_meters: float
var near_clip_distance_meters: float
var far_clip_distance_meters: float
var force_response_strength: float
var longitudinal_force_response_strength: float
var vertical_bump_response_strength: float
var lateral_bump_response_strength: float
var lateral_response_degrees_per_gravity: float
var longitudinal_response_degrees_per_gravity: float
var maximum_longitudinal_acceleration_gravity: float
var longitudinal_acceleration_filter_time_seconds: float
var gear_shift_motion_reduction_strength: float
var gear_shift_motion_recovery_time_seconds: float
var suspension_pitch_response_degrees_per_gravity: float
var vertical_bump_response_meters_per_gravity: float
var maximum_suspension_pitch_degrees: float
var maximum_pitch_degrees: float
var maximum_roll_degrees: float
var maximum_telemetry_gravity: float
var response_frequency_hertz: float
var damping_ratio: float
var longitudinal_compensation_time_seconds: float
var longitudinal_release_time_seconds: float
var longitudinal_compensation_strength: float
var suspension_bump_separation_time_seconds: float
var minimum_suspension_bump_displacement_meters: float
var road_bump_full_confirmation_displacement_meters: float
var suspension_bump_response_gravity_per_meter: float
var lateral_acceleration_filter_time_seconds: float
var vertical_acceleration_filter_time_seconds: float
var lateral_response_frequency_hertz: float
var vertical_response_frequency_hertz: float
var lateral_damping_ratio: float
var vertical_damping_ratio: float
var positional_filter_time_seconds: float
var vertical_positional_filter_time_seconds: float
var positional_stabilization_strength: float
var positional_bump_full_response_gravity: float
var bump_strength_filter_time_seconds: float
var seat_acceleration_filter_time_seconds: float
var seat_acceleration_blend_strength: float
var maximum_vertical_correction_meters: float
var maximum_lateral_correction_meters: float
var neck_rotation_fraction: float
var horizon_stabilization_strength: float
var pitch_horizon_stabilization_strength: float
var roll_horizon_stabilization_strength: float
var road_pitch_follow_time_seconds: float
var road_roll_follow_time_seconds: float
var velocity_alignment_strength: float
var maximum_velocity_alignment_degrees: float
var velocity_alignment_filter_time_seconds: float
var minimum_velocity_alignment_speed_meters_per_second: float
var full_velocity_alignment_speed_meters_per_second: float
var maximum_horizon_correction_degrees: float
var teleport_reset_distance_meters: float
var maximum_elapsed_seconds: float
var maximum_integration_step_seconds: float

static func load_from_path(configuration_path: String) -> CockpitCameraConfiguration:
	var configuration_file := FileAccess.open(configuration_path, FileAccess.READ)
	if configuration_file == null:
		push_error("Cockpit configuration is missing: " + configuration_path)
		return null
	var configuration_data: Variant = JSON.parse_string(configuration_file.get_as_text())
	if not configuration_data is Dictionary or configuration_data.get("schema_version") != 1:
		push_error("Cockpit configuration schema is invalid: " + configuration_path)
		return null
	var configuration := CockpitCameraConfiguration.new()
	var permitted_ranges := {
		"field_of_view_degrees": [40.0, 110.0],
		"viewpoint_elevation_meters": [0.0, 0.5],
		"near_clip_distance_meters": [0.001, 0.1],
		"far_clip_distance_meters": [100.0, 8000.0],
		"force_response_strength": [0.0, 2.0],
		"longitudinal_force_response_strength": [0.0, 2.0],
		"vertical_bump_response_strength": [0.0, 2.0],
		"lateral_bump_response_strength": [0.0, 2.0],
		"lateral_response_degrees_per_gravity": [0.0, 20.0],
		"longitudinal_response_degrees_per_gravity": [0.0, 20.0],
		"maximum_longitudinal_acceleration_gravity": [0.1, 8.0],
		"longitudinal_acceleration_filter_time_seconds": [0.0, 0.5],
		"gear_shift_motion_reduction_strength": [0.0, 1.0],
		"gear_shift_motion_recovery_time_seconds": [0.01, 1.0],
		"suspension_pitch_response_degrees_per_gravity": [0.0, 5.0],
		"vertical_bump_response_meters_per_gravity": [0.0, 0.02],
		"maximum_suspension_pitch_degrees": [0.0, 3.0],
		"maximum_pitch_degrees": [0.0, 30.0],
		"maximum_roll_degrees": [0.0, 30.0],
		"maximum_telemetry_gravity": [1.0, 20.0],
		"response_frequency_hertz": [0.1, 10.0],
		"damping_ratio": [0.1, 2.0],
		"longitudinal_compensation_time_seconds": [0.05, 3.0],
		"longitudinal_release_time_seconds": [0.01, 1.0],
		"longitudinal_compensation_strength": [0.0, 1.0],
		"suspension_bump_separation_time_seconds": [0.02, 0.5],
		"minimum_suspension_bump_displacement_meters": [0.0, 0.02],
		"road_bump_full_confirmation_displacement_meters": [0.001, 0.05],
		"suspension_bump_response_gravity_per_meter": [0.0, 500.0],
		"lateral_acceleration_filter_time_seconds": [0.0, 0.5],
		"vertical_acceleration_filter_time_seconds": [0.0, 0.5],
		"lateral_response_frequency_hertz": [0.1, 10.0],
		"vertical_response_frequency_hertz": [0.1, 10.0],
		"lateral_damping_ratio": [0.1, 2.0],
		"vertical_damping_ratio": [0.1, 2.0],
		"positional_filter_time_seconds": [0.005, 0.5],
		"vertical_positional_filter_time_seconds": [0.005, 0.5],
		"positional_stabilization_strength": [0.0, 1.0],
		"positional_bump_full_response_gravity": [0.01, 8.0],
		"bump_strength_filter_time_seconds": [0.005, 0.5],
		"seat_acceleration_filter_time_seconds": [0.02, 1.0],
		"seat_acceleration_blend_strength": [0.0, 1.0],
		"maximum_vertical_correction_meters": [0.0, 0.02],
		"maximum_lateral_correction_meters": [0.0, 0.01],
		"neck_rotation_fraction": [0.05, 0.95],
		"horizon_stabilization_strength": [0.0, 1.0],
		"pitch_horizon_stabilization_strength": [0.0, 1.0],
		"roll_horizon_stabilization_strength": [0.0, 1.0],
		"road_pitch_follow_time_seconds": [0.02, 2.0],
		"road_roll_follow_time_seconds": [0.02, 2.0],
		"velocity_alignment_strength": [0.0, 1.0],
		"maximum_velocity_alignment_degrees": [0.0, 20.0],
		"velocity_alignment_filter_time_seconds": [0.02, 1.0],
		"minimum_velocity_alignment_speed_meters_per_second": [0.1, 30.0],
		"full_velocity_alignment_speed_meters_per_second": [0.2, 80.0],
		"maximum_horizon_correction_degrees": [0.0, 90.0],
		"teleport_reset_distance_meters": [0.5, 100.0],
		"maximum_elapsed_seconds": [0.02, 0.25],
		"maximum_integration_step_seconds": [0.001, 0.01]
	}
	for property_name in permitted_ranges:
		var property_value: Variant = configuration_data.get(property_name)
		var permitted_range: Array = permitted_ranges[property_name]
		if not (property_value is float or property_value is int):
			push_error("Cockpit configuration requires a numeric value: " + property_name)
			return null
		var numeric_value := float(property_value)
		if not is_finite(numeric_value) or numeric_value < permitted_range[0] or numeric_value > permitted_range[1]:
			push_error("Cockpit configuration value is outside its permitted range: " + property_name)
			return null
		configuration.set(property_name, numeric_value)
	if configuration.full_velocity_alignment_speed_meters_per_second <= configuration.minimum_velocity_alignment_speed_meters_per_second:
		push_error("Cockpit velocity alignment requires increasing speed thresholds.")
		return null
	if configuration.road_bump_full_confirmation_displacement_meters <= configuration.minimum_suspension_bump_displacement_meters:
		push_error("Cockpit road bump confirmation requires increasing displacement thresholds.")
		return null
	return configuration

func get_motion_preferences() -> Dictionary:
	return {
		"horizon_stabilization_strength": horizon_stabilization_strength,
		"longitudinal_force_response_strength": longitudinal_force_response_strength,
		"vertical_bump_response_strength": vertical_bump_response_strength,
		"lateral_bump_response_strength": lateral_bump_response_strength,
		"positional_stabilization_strength": positional_stabilization_strength,
		"velocity_alignment_strength": velocity_alignment_strength
	}

func apply_motion_preferences(preferences: Dictionary) -> void:
	for property_name in get_motion_preferences():
		var property_value: Variant = preferences.get(property_name)
		if not (property_value is float or property_value is int) or not is_finite(float(property_value)):
			continue
		var maximum_value := 2.0 if property_name in ["longitudinal_force_response_strength", "vertical_bump_response_strength", "lateral_bump_response_strength"] else 1.0
		set(property_name, clampf(float(property_value), 0.0, maximum_value))
