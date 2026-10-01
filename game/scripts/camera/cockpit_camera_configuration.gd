class_name CockpitCameraConfiguration
extends RefCounted

var field_of_view_degrees: float
var viewpoint_elevation_meters: float
var near_clip_distance_meters: float
var far_clip_distance_meters: float
var force_response_strength: float
var lateral_response_degrees_per_gravity: float
var longitudinal_response_degrees_per_gravity: float
var vertical_response_degrees_per_gravity: float
var maximum_pitch_degrees: float
var maximum_roll_degrees: float
var maximum_telemetry_gravity: float
var response_frequency_hertz: float
var damping_ratio: float
var neck_rotation_fraction: float
var horizon_stabilization_strength: float
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
		"field_of_view_degrees": Vector2(40.0, 110.0),
		"viewpoint_elevation_meters": Vector2(0.0, 0.5),
		"near_clip_distance_meters": Vector2(0.001, 0.1),
		"far_clip_distance_meters": Vector2(100.0, 8000.0),
		"force_response_strength": Vector2(0.0, 2.0),
		"lateral_response_degrees_per_gravity": Vector2(0.0, 20.0),
		"longitudinal_response_degrees_per_gravity": Vector2(0.0, 20.0),
		"vertical_response_degrees_per_gravity": Vector2(0.0, 20.0),
		"maximum_pitch_degrees": Vector2(0.0, 30.0),
		"maximum_roll_degrees": Vector2(0.0, 30.0),
		"maximum_telemetry_gravity": Vector2(1.0, 20.0),
		"response_frequency_hertz": Vector2(0.1, 10.0),
		"damping_ratio": Vector2(0.1, 2.0),
		"neck_rotation_fraction": Vector2(0.05, 0.95),
		"horizon_stabilization_strength": Vector2(0.0, 1.0),
		"maximum_horizon_correction_degrees": Vector2(0.0, 90.0),
		"teleport_reset_distance_meters": Vector2(0.5, 100.0),
		"maximum_elapsed_seconds": Vector2(0.02, 0.25),
		"maximum_integration_step_seconds": Vector2(0.001, 0.01)
	}
	for property_name in permitted_ranges:
		var property_value: Variant = configuration_data.get(property_name)
		var permitted_range: Vector2 = permitted_ranges[property_name]
		if not (property_value is float or property_value is int):
			push_error("Cockpit configuration requires a numeric value: " + property_name)
			return null
		var numeric_value := float(property_value)
		if not is_finite(numeric_value) or numeric_value < permitted_range.x or numeric_value > permitted_range.y:
			push_error("Cockpit configuration value is outside its permitted range: " + property_name)
			return null
		configuration.set(property_name, numeric_value)
	return configuration
