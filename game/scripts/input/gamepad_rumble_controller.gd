class_name GamepadRumbleController
extends Node

const PIANO_SURFACE_CODE := 1
const OFF_ASPHALT_SURFACE_CODES := [2, 3, 4, 5]
const BAND_PIANO := "piano"
const BAND_OFF_ASPHALT := "off_asphalt"
const BAND_FRONT_GRIP_LIMIT := "front_grip_limit"
const BAND_REAR_GRIP_LIMIT := "rear_grip_limit"
const GRIP_BANDS := [BAND_FRONT_GRIP_LIMIT, BAND_REAR_GRIP_LIMIT]
const FRONT_WHEEL_INDEX := 0
const REAR_WHEEL_INDEX := 2

var vehicle_node: Node

var _active_device_id := -1
var _grip_band_engaged := { BAND_FRONT_GRIP_LIMIT: false, BAND_REAR_GRIP_LIMIT: false }
var _current_band := ""
var _pulse_clock_seconds := 0.0
var _rumbling := false


func _exit_tree() -> void:
	_stop_rumble()


func _input(event: InputEvent) -> void:
	if event is InputEventJoypadButton and event.pressed:
		_active_device_id = event.device
	elif event is InputEventJoypadMotion and absf(event.axis_value) > 0.5:
		_active_device_id = event.device


func _physics_process(delta: float) -> void:
	var connected_devices := Input.get_connected_joypads()
	if connected_devices.is_empty():
		_active_device_id = -1
		_stop_rumble()
		return
	if _active_device_id < 0 or not connected_devices.has(_active_device_id):
		var previous_device := _active_device_id
		_active_device_id = connected_devices[0]
		if previous_device >= 0 and previous_device != _active_device_id:
			_stop_rumble()
	if not Input.has_joy_vibration(_active_device_id):
		_stop_rumble()
		return
	var profile := _active_profile()
	if profile == null:
		_stop_rumble()
		return
	var band := _dominant_band(profile)
	if band == "":
		_stop_rumble()
		return
	if band != _current_band:
		_current_band = band
		_pulse_clock_seconds = 0.0
	var band_settings: Dictionary = profile.vibration_settings[band]
	var interval_seconds := 1.0 / float(band_settings["pulses_per_second"])
	_pulse_clock_seconds += delta
	if _pulse_clock_seconds < interval_seconds:
		return
	_pulse_clock_seconds = fmod(_pulse_clock_seconds, interval_seconds)
	Input.start_joy_vibration(
		_active_device_id,
		float(band_settings["weak_motor_magnitude"]),
		float(band_settings["strong_motor_magnitude"]),
		interval_seconds)
	_rumbling = true


func _dominant_band(profile: InputProfile) -> String:
	if vehicle_node == null or not is_instance_valid(vehicle_node):
		return ""
	if "enable_player_input" in vehicle_node and not vehicle_node.enable_player_input:
		return ""
	if not vehicle_node.has_method("get_speed_kmh") or not vehicle_node.has_method("get_wheel_surface_types"):
		return ""
	if float(vehicle_node.get_speed_kmh()) < profile.vibration_minimum_speed_kmh:
		return ""
	_update_grip_band_engagement(profile)
	for grip_band in GRIP_BANDS:
		if bool(_grip_band_engaged[grip_band]):
			return grip_band
	var surface_band := InputProfile.surface_band_for_wheel_codes(
		vehicle_node.get_wheel_surface_types(),
		PIANO_SURFACE_CODE,
		OFF_ASPHALT_SURFACE_CODES)
	return surface_band


func _update_grip_band_engagement(profile: InputProfile) -> void:
	if not profile.vibration_settings.has(BAND_FRONT_GRIP_LIMIT) or not profile.vibration_settings.has(BAND_REAR_GRIP_LIMIT):
		return
	var estimates := _slip_angle_estimates()
	var front_angle: float = estimates[0]
	var rear_angle: float = estimates[1]
	var front_settings: Dictionary = profile.vibration_settings[BAND_FRONT_GRIP_LIMIT]
	var rear_settings: Dictionary = profile.vibration_settings[BAND_REAR_GRIP_LIMIT]
	_grip_band_engaged[BAND_FRONT_GRIP_LIMIT] = _apply_hysteresis(
		bool(_grip_band_engaged[BAND_FRONT_GRIP_LIMIT]),
		front_angle,
		float(front_settings.get("engage_slip_angle_rad", 0.0)),
		float(front_settings.get("release_slip_angle_rad", 0.0)))
	_grip_band_engaged[BAND_REAR_GRIP_LIMIT] = _apply_hysteresis(
		bool(_grip_band_engaged[BAND_REAR_GRIP_LIMIT]),
		rear_angle,
		float(rear_settings.get("engage_slip_angle_rad", 0.0)),
		float(rear_settings.get("release_slip_angle_rad", 0.0)))


static func _apply_hysteresis(engaged: bool, magnitude: float, engage_threshold: float, release_threshold: float) -> bool:
	if engaged:
		return magnitude >= release_threshold
	return magnitude >= engage_threshold


func _slip_angle_estimates() -> Array[float]:
	if not (vehicle_node.has_method("get_linear_velocity") and vehicle_node.has_method("get_angular_velocity")
			and vehicle_node.has_method("get_steer_angle_rad") and vehicle_node.has_method("get_wheel_anchor_local")):
		return [0.0, 0.0]
	var velocity_local := GamepadRumbleController.local_velocity_from_vehicle(vehicle_node)
	var forward_speed := -velocity_local.z
	if forward_speed <= 0.0:
		return [0.0, 0.0]
	var yaw_rate := GamepadRumbleController.yaw_rate_from_vehicle(vehicle_node)
	var front_anchor: Vector3 = vehicle_node.get_wheel_anchor_local(FRONT_WHEEL_INDEX)
	var rear_anchor: Vector3 = vehicle_node.get_wheel_anchor_local(REAR_WHEEL_INDEX)
	var steer_angle: float = vehicle_node.get_steer_angle_rad()
	var front_angle := GamepadRumbleController.front_slip_angle_rad(velocity_local, yaw_rate, absf(front_anchor.z), absf(rear_anchor.z), steer_angle)
	var rear_angle := GamepadRumbleController.rear_slip_angle_rad(velocity_local, yaw_rate, absf(rear_anchor.z))
	return [absf(front_angle), absf(rear_angle)]


static func local_velocity_from_vehicle(vehicle: Node) -> Vector3:
	var velocity_world: Vector3 = vehicle.get_linear_velocity()
	var body_to_local_basis: Basis = vehicle.global_transform.basis.inverse()
	return body_to_local_basis * velocity_world


static func yaw_rate_from_vehicle(vehicle: Node) -> float:
	var angular_velocity_world: Vector3 = vehicle.get_angular_velocity()
	var body_up: Vector3 = vehicle.global_transform.basis.y
	return angular_velocity_world.dot(body_up)


static func front_slip_angle_rad(velocity_local: Vector3, yaw_rate_rad_per_second: float, front_axle_distance_m: float, rear_axle_distance_m: float, steer_angle_rad: float) -> float:
	var forward_speed := -velocity_local.z
	if forward_speed <= 0.0:
		return 0.0
	var leftward_speed_at_front := -velocity_local.x + yaw_rate_rad_per_second * front_axle_distance_m
	return atan2(leftward_speed_at_front, forward_speed) - steer_angle_rad


static func rear_slip_angle_rad(velocity_local: Vector3, yaw_rate_rad_per_second: float, rear_axle_distance_m: float) -> float:
	var forward_speed := -velocity_local.z
	if forward_speed <= 0.0:
		return 0.0
	var leftward_speed_at_rear := -velocity_local.x - yaw_rate_rad_per_second * rear_axle_distance_m
	return atan2(leftward_speed_at_rear, forward_speed)


func _stop_rumble() -> void:
	_current_band = ""
	_pulse_clock_seconds = 0.0
	for grip_band in GRIP_BANDS:
		_grip_band_engaged[grip_band] = false
	if _rumbling and _active_device_id >= 0:
		Input.stop_joy_vibration(_active_device_id)
	_rumbling = false


func _active_profile() -> InputProfile:
	var bindings := get_node_or_null("/root/InputBindings")
	if bindings == null:
		return null
	var profile: InputProfile = bindings.input_profile
	if profile == null or not profile.is_valid():
		return null
	return profile
