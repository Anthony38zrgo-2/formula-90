extends Node

var enabled := true
var vehicle : Vehicle

var _file : FileAccess
var _buffer := PackedStringArray()
var _prev_velocity := Vector3.ZERO
var _prev_velocity_time := 0
var _prev_sample_time := 0
var _file_opened := false
var _search_timer := 0.0
var _setup_signature := ""
var _session_sequence := 0

const LOG_MS := 50
const BUFFER_SIZE := 100
const VEHICLE_SEARCH_INTERVAL := 1.0
const SETUP_SCHEMA_VERSION := 1
var _setup_families := {
	"chassis": [
		"vehicle_mass", "front_weight_distribution", "center_of_gravity_height_offset", "inertia_multiplier"
	],
	"steering": [
		"steering_speed", "countersteer_speed", "steering_speed_decay", "steering_slip_assist",
		"countersteer_assist", "steering_exponent", "front_steering_ratio", "rear_steering_ratio"
	],
	"brakes": [
		"braking_speed", "brake_force_multiplier", "front_brake_bias", "front_abs_pulse_time",
		"front_abs_spin_difference_threshold", "rear_abs_pulse_time", "rear_abs_spin_difference_threshold"
	],
	"differential": [
		"front_torque_split", "variable_torque_split", "front_variable_split", "variable_split_speed",
		"front_locking_differential_engage_torque", "front_torque_vectoring",
		"rear_locking_differential_engage_torque", "rear_torque_vectoring"
	],
	"suspension": [
		"front_spring_length", "front_resting_ratio", "front_damping_ratio", "front_bump_damp_multiplier",
		"front_rebound_damp_multiplier", "front_arb_ratio", "front_camber", "front_toe",
		"front_bump_stop_multiplier", "front_beam_axle", "rear_spring_length", "rear_resting_ratio",
		"rear_damping_ratio", "rear_bump_damp_multiplier", "rear_rebound_damp_multiplier", "rear_arb_ratio",
		"rear_camber", "rear_toe", "rear_bump_stop_multiplier", "rear_beam_axle"
	],
	"tires_and_surfaces": [
		"front_tire_radius", "front_tire_width", "front_wheel_mass", "rear_tire_radius", "rear_tire_width",
		"rear_wheel_mass", "contact_patch", "braking_grip_multiplier", "wheel_to_body_torque_multiplier",
		"tire_stiffnesses", "coefficient_of_friction", "rolling_resistance", "lateral_grip_assist",
		"longitudinal_grip_ratio"
	],
	"engine": [
		"max_torque", "max_rpm", "idle_rpm", "motor_drag", "motor_brake", "motor_moment",
		"clutch_out_rpm", "max_clutch_torque_ratio"
	],
	"transmission": [
		"gear_ratios", "final_drive", "reverse_ratio", "shift_time", "automatic_transmission",
		"automatic_time_between_shifts", "gear_inertia"
	],
	"aerodynamics": [
		"coefficient_of_drag", "air_density", "frontal_area"
	]
}

func _ready():
	_prev_sample_time = Time.get_ticks_msec()
	_prev_velocity_time = _prev_sample_time

func _physics_process(delta):
	if not enabled:
		return

	if not vehicle:
		_search_timer += delta
		if _search_timer >= VEHICLE_SEARCH_INTERVAL:
			_search_timer = 0.0
			_try_find_vehicle()
		return
	_search_timer = 0.0

	var current_velocity = vehicle.linear_velocity

	var setup := _build_setup_snapshot()
	var signature := JSON.stringify(setup, "", true)
	if not _file_opened:
		_open_session(setup, signature)
		_prev_velocity = current_velocity
		_prev_velocity_time = Time.get_ticks_msec()
	elif signature != _setup_signature:
		_close_session()
		_open_session(setup, signature)
		_prev_velocity = current_velocity
		_prev_velocity_time = Time.get_ticks_msec()

	var now = Time.get_ticks_msec()
	if now - _prev_sample_time < LOG_MS:
		return
	_prev_sample_time = now

	var line = _format_line(now, current_velocity)
	_buffer.append(line)
	if _buffer.size() >= BUFFER_SIZE:
		_flush()

func _exit_tree():
	_close_session()

func _try_find_vehicle():
	var nodes = get_tree().root.find_children("*", "Vehicle", true, false)
	if nodes.size() > 0:
		vehicle = nodes[0] as Vehicle
		_prev_velocity_time = 0
		_setup_signature = ""

func _next_session_base() -> String:
	var dir = "res://telemetry/"
	DirAccess.make_dir_recursive_absolute(dir)
	var dt = Time.get_datetime_dict_from_system()
	var msec = Time.get_ticks_msec() % 1000
	var base_stem = "telemetry_%04d%02d%02d_%02d%02d%02d_%03d" % [
		dt.year, dt.month, dt.day,
		dt.hour, dt.minute, dt.second, msec
	]
	var stem = base_stem
	while FileAccess.file_exists(dir + stem + ".csv") or FileAccess.file_exists(dir + stem + "_setup.json"):
		_session_sequence += 1
		stem = base_stem + "_%02d" % _session_sequence
	return stem

func _open_session(setup: Dictionary, signature: String):
	var dir = "res://telemetry/"
	var stem = _next_session_base()
	var setup_path = dir + stem + "_setup.json"
	var setup_file = FileAccess.open(setup_path, FileAccess.WRITE)
	if not setup_file:
		push_error("[TelemetryManager] Cannot open setup snapshot: ", setup_path)
		return
	var persisted_setup = setup.duplicate(true)
	persisted_setup["captured_at_utc"] = Time.get_datetime_string_from_system(true, true)
	persisted_setup["csv_file"] = stem + ".csv"
	setup_file.store_string(JSON.stringify(persisted_setup, "\t", true) + "\n")
	setup_file.close()

	var path = dir + stem + ".csv"
	_file = FileAccess.open(path, FileAccess.WRITE)
	if not _file:
		push_error("[TelemetryManager] Cannot open file: ", path)
		return
	_file_opened = true
	_setup_signature = signature
	_file.store_csv_line(PackedStringArray([
		"Time_ms", "Speed_kmh", "RPM", "Gear",
		"Throttle", "Brake", "Steering",
		"Lat_G", "Long_G",
		"FL_Comp", "FR_Comp", "RL_Comp", "RR_Comp",
		"Front_Slip", "Rear_Slip"
	]))
	print("[TelemetryManager] Started capture: ", path, " with ", setup_path)

func _close_session():
	if _file_opened:
		_flush()
		_file.close()
		_file_opened = false
	_file = null
	_setup_signature = ""

func _build_setup_snapshot() -> Dictionary:
	var families := {}
	for family_name in _setup_families:
		families[family_name] = _read_vehicle_properties(_setup_families[family_name])
	return {
		"schema_version": SETUP_SCHEMA_VERSION,
		"provenance": {
			"active_scene_path": _active_scene_path(),
			"vehicle_node_path": str(vehicle.get_path()),
			"vehicle_scene_path": vehicle.scene_file_path,
			"engine_config_path": _resource_path(vehicle.get("engine_config")),
			"torque_curve_path": _resource_path(vehicle.get("torque_curve"))
		},
		"setup": families,
		"assists": _read_assists()
	}

func _read_vehicle_properties(property_names: Array) -> Dictionary:
	var values := {}
	for property_name in property_names:
		values[property_name] = vehicle.get(property_name)
	return values

func _resource_path(value) -> Variant:
	if value is Resource:
		return value.resource_path if not value.resource_path.is_empty() else null
	return null

func _active_scene_path() -> String:
	var scene = get_tree().current_scene
	return scene.scene_file_path if scene else ""

func _read_assists() -> Dictionary:
	for candidate in get_tree().root.find_children("*", "Node", true, false):
		if not candidate.has_method("is_aid_enabled") or not candidate.has_method("get_aid_label"):
			continue
		if candidate.get("vehicle_node") != vehicle:
			continue
		var states := {}
		var enabled_values = candidate.get("aids")
		if enabled_values is Array:
			for index in enabled_values.size():
				states[str(candidate.call("get_aid_label", index))] = candidate.call("is_aid_enabled", index)
		return {"controller_present": true, "states": states}
	return {"controller_present": false, "states": {}}


func _format_line(now_msec: int, current_velocity: Vector3) -> String:
	var speed_kmh = abs(vehicle.speed) * 3.6
	var rpm = vehicle.motor_rpm
	var gear = vehicle.current_gear
	var throttle = vehicle.throttle_amount
	var brake_amt = vehicle.brake_amount
	var steering = vehicle.steering_input

	var lat_g := 0.0
	var long_g := 0.0
	if _prev_velocity_time > 0:
		var elapsed = max((now_msec - _prev_velocity_time) / 1000.0, 0.001)
		var accel = (current_velocity - _prev_velocity) / elapsed
		var local_accel = vehicle.global_transform.basis.inverse() * accel
		lat_g = local_accel.x / 9.81
		long_g = -local_accel.z / 9.81
	_prev_velocity = current_velocity
	_prev_velocity_time = now_msec

	var fl_comp = vehicle.front_axle.suspension_compression_left if vehicle.front_axle else 0.0
	var fr_comp = vehicle.front_axle.suspension_compression_right if vehicle.front_axle else 0.0
	var rl_comp = vehicle.rear_axle.suspension_compression_left if vehicle.rear_axle else 0.0
	var rr_comp = vehicle.rear_axle.suspension_compression_right if vehicle.rear_axle else 0.0

	var front_slip = vehicle.front_axle.get_max_wheel_slip_y() if vehicle.front_axle else 0.0
	var rear_slip = vehicle.rear_axle.get_max_wheel_slip_y() if vehicle.rear_axle else 0.0

	return "%d,%.1f,%d,%d,%.3f,%.3f,%.3f,%.3f,%.3f,%.1f,%.1f,%.1f,%.1f,%.3f,%.3f" % [
		now_msec, speed_kmh, rpm, gear,
		throttle, brake_amt, steering,
		lat_g, long_g,
		fl_comp, fr_comp, rl_comp, rr_comp,
		front_slip, rear_slip
	]

func _flush():
	if _file and _buffer.size() > 0:
		_file.store_string("\n".join(_buffer) + "\n")
		_buffer.clear()
