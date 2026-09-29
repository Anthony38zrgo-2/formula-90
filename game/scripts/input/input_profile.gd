class_name InputProfile
extends RefCounted

const MINIMUM_DEADZONE := 0.0
const MAXIMUM_DEADZONE := 0.9
const MINIMUM_CURVE_EXPONENT := 0.05
const MAXIMUM_CURVE_EXPONENT := 5.0
const MINIMUM_RETURN_SPEED_PER_SECOND := 0.1
const MAXIMUM_RETURN_SPEED_PER_SECOND := 50.0
const MINIMUM_RELEASE_SPEED_PER_SECOND := 0.0
const MAXIMUM_RELEASE_SPEED_PER_SECOND := 50.0
const MINIMUM_PULSES_PER_SECOND := 0.5
const MAXIMUM_PULSES_PER_SECOND := 50.0
const MINIMUM_MAGNITUDE := 0.0
const MAXIMUM_MAGNITUDE := 1.0
const MINIMUM_SLIP_ANGLE_RAD := 0.01
const MAXIMUM_SLIP_ANGLE_RAD := 0.6
const MAXIMUM_AXIS_INDEX := 9
const MAXIMUM_BUTTON_INDEX := 64

var schema_version := 0
var errors: Array[String] = []
var action_definitions: Dictionary = {}
var steering_axis_deadzone := 0.0
var steering_curve_exponent := 1.0
var throttle_curve_exponent := 1.0
var brake_curve_exponent := 1.0
var throttle_release_speed_per_second := 0.0
var brake_release_speed_per_second := 0.0
var steering_return_speed_per_second := 0.0
var vibration_minimum_speed_kmh := 0.0
var vibration_settings: Dictionary = {}


static func load_from_path(path: String) -> InputProfile:
	var profile := InputProfile.new()
	if not FileAccess.file_exists(path):
		profile.errors.append("input profile file is missing: " + path)
		return profile
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		profile.errors.append("input profile file cannot be opened: " + path)
		return profile
	var text := file.get_as_text()
	file.close()
	profile.parse_json_text(text)
	return profile


func parse_json_text(text: String) -> void:
	var parsed: Variant = JSON.parse_string(text)
	if typeof(parsed) != TYPE_DICTIONARY:
		errors.append("input profile is not a JSON object")
		return
	var document: Dictionary = parsed
	schema_version = int(document.get("schema_version", 0))
	if schema_version != 1:
		errors.append("unsupported input profile schema_version: %d (expected 1)" % schema_version)
		schema_version = 0
		return
	_collect_action_definitions(document.get("actions", []))
	_collect_axis_shaping(document.get("axis_shaping", {}))
	_collect_vibration(document.get("vibration", {}))


func is_valid() -> bool:
	return errors.is_empty()


func has_registered_action(action_name: String) -> bool:
	return action_definitions.has(action_name)


func get_action_events(action_name: String) -> Array:
	if not action_definitions.has(action_name):
		return []
	return action_definitions[action_name]["events"]


static func apply_signed_curve(value: float, exponent: float) -> float:
	var clamped := clampf(value, -1.0, 1.0)
	return signf(clamped) * pow(absf(clamped), exponent)


static func apply_unsigned_curve(value: float, exponent: float) -> float:
	var clamped := clampf(value, 0.0, 1.0)
	return pow(clamped, exponent)


static func remap_axis_deadzone(value: float, deadzone: float) -> float:
	if absf(value) <= deadzone:
		return 0.0
	var clamped_deadzone := clampf(deadzone, 0.0, MAXIMUM_DEADZONE)
	var remaining := 1.0 - clamped_deadzone
	if remaining <= 0.0:
		return 0.0
	var rescaled := (absf(value) - clamped_deadzone) / remaining
	return signf(value) * clampf(rescaled, 0.0, 1.0)


static func apply_release_limit(previous_value: float, raw_value: float, release_speed_per_second: float, delta_seconds: float) -> float:
	var clamped_previous := clampf(previous_value, 0.0, 1.0)
	var clamped_raw := clampf(raw_value, 0.0, 1.0)
	if release_speed_per_second <= 0.0 or delta_seconds <= 0.0:
		return clamped_raw
	if clamped_raw >= clamped_previous:
		return clamped_raw
	return maxf(clamped_raw, clamped_previous - release_speed_per_second * delta_seconds)


static func surface_band_for_wheel_codes(wheel_codes: Array, piano_code: int, off_asphalt_codes: Array) -> String:
	var saw_off_asphalt := false
	for raw_code in wheel_codes:
		var code := int(raw_code)
		if code == piano_code:
			return "piano"
		if off_asphalt_codes.has(code):
			saw_off_asphalt = true
	if saw_off_asphalt:
		return "off_asphalt"
	return ""


func _collect_action_definitions(raw_actions: Variant) -> void:
	if typeof(raw_actions) != TYPE_ARRAY:
		errors.append("input profile actions must be an array")
		return
	for raw_action in raw_actions:
		if typeof(raw_action) != TYPE_DICTIONARY:
			errors.append("each action entry must be an object")
			continue
		var action: Dictionary = raw_action
		var action_name := String(action.get("name", ""))
		if action_name.is_empty():
			errors.append("action entry is missing a name")
			continue
		if action_definitions.has(action_name):
			errors.append("duplicate action name: " + action_name)
			continue
		var deadzone := float(action.get("deadzone", -1.0))
		if deadzone < MINIMUM_DEADZONE or deadzone > MAXIMUM_DEADZONE:
			errors.append("action '%s' deadzone %f is outside [%f, %f]" % [action_name, deadzone, MINIMUM_DEADZONE, MAXIMUM_DEADZONE])
			continue
		var events := _build_events(action_name, action.get("events", []))
		if events.is_empty():
			errors.append("action '%s' has no valid events" % action_name)
			continue
		action_definitions[action_name] = { "deadzone": deadzone, "events": events }


func _build_events(action_name: String, raw_events: Variant) -> Array:
	if typeof(raw_events) != TYPE_ARRAY:
		errors.append("action '%s' events must be an array" % action_name)
		return []
	var built: Array = []
	for raw_event in raw_events:
		if typeof(raw_event) != TYPE_DICTIONARY:
			errors.append("action '%s' event entries must be objects" % action_name)
			continue
		var event: Dictionary = raw_event
		var event_type := String(event.get("type", ""))
		match event_type:
			"physical_key":
				var keycode := int(event.get("physical_keycode", -1))
				if keycode < 0:
					errors.append("action '%s' physical_key event needs a non-negative physical_keycode" % action_name)
					continue
				var key := InputEventKey.new()
				key.physical_keycode = keycode
				built.append(key)
			"gamepad_button":
				var button_index := int(event.get("button_index", -1))
				if button_index < 0 or button_index >= MAXIMUM_BUTTON_INDEX:
					errors.append("action '%s' gamepad_button event needs button_index in [0, %d)" % [action_name, MAXIMUM_BUTTON_INDEX])
					continue
				var button := InputEventJoypadButton.new()
				button.button_index = button_index
				built.append(button)
			"gamepad_axis":
				var axis_index := int(event.get("axis", -1))
				var axis_value := float(event.get("axis_value", 0.0))
				if axis_index < 0 or axis_index > MAXIMUM_AXIS_INDEX:
					errors.append("action '%s' gamepad_axis event needs axis in [0, %d]" % [action_name, MAXIMUM_AXIS_INDEX])
					continue
				if axis_value < -1.0 or axis_value > 1.0:
					errors.append("action '%s' gamepad_axis event needs axis_value in [-1, 1]" % action_name)
					continue
				var axis := InputEventJoypadMotion.new()
				axis.axis = axis_index
				axis.axis_value = axis_value
				built.append(axis)
			_:
				errors.append("action '%s' uses unknown event type: %s" % [action_name, event_type])
	return built


func _collect_axis_shaping(raw_shaping: Variant) -> void:
	if typeof(raw_shaping) != TYPE_DICTIONARY:
		errors.append("input profile axis_shaping must be an object")
		return
	var shaping: Dictionary = raw_shaping
	steering_axis_deadzone = float(shaping.get("steering_axis_deadzone", -1.0))
	if steering_axis_deadzone < MINIMUM_DEADZONE or steering_axis_deadzone > MAXIMUM_DEADZONE:
		errors.append("steering_axis_deadzone %f is outside [%f, %f]" % [steering_axis_deadzone, MINIMUM_DEADZONE, MAXIMUM_DEADZONE])
		steering_axis_deadzone = 0.0
	steering_curve_exponent = _read_exponent(shaping, "steering_curve_exponent")
	throttle_curve_exponent = _read_exponent(shaping, "throttle_curve_exponent")
	brake_curve_exponent = _read_exponent(shaping, "brake_curve_exponent")
	throttle_release_speed_per_second = _read_release_speed(shaping, "throttle_release_speed_per_second")
	brake_release_speed_per_second = _read_release_speed(shaping, "brake_release_speed_per_second")
	steering_return_speed_per_second = float(shaping.get("steering_return_speed_per_second", -1.0))
	if steering_return_speed_per_second < MINIMUM_RETURN_SPEED_PER_SECOND or steering_return_speed_per_second > MAXIMUM_RETURN_SPEED_PER_SECOND:
		errors.append("steering_return_speed_per_second %f is outside [%f, %f]" % [steering_return_speed_per_second, MINIMUM_RETURN_SPEED_PER_SECOND, MAXIMUM_RETURN_SPEED_PER_SECOND])
		steering_return_speed_per_second = 0.0


func _read_exponent(shaping: Dictionary, field_name: String) -> float:
	var exponent := float(shaping.get(field_name, -1.0))
	if exponent < MINIMUM_CURVE_EXPONENT or exponent > MAXIMUM_CURVE_EXPONENT:
		errors.append("%s %f is outside [%f, %f]" % [field_name, exponent, MINIMUM_CURVE_EXPONENT, MAXIMUM_CURVE_EXPONENT])
		return 1.0
	return exponent


func _read_release_speed(shaping: Dictionary, field_name: String) -> float:
	var release_speed := float(shaping.get(field_name, -1.0))
	if release_speed < MINIMUM_RELEASE_SPEED_PER_SECOND or release_speed > MAXIMUM_RELEASE_SPEED_PER_SECOND:
		errors.append("%s %f is outside [%f, %f]" % [field_name, release_speed, MINIMUM_RELEASE_SPEED_PER_SECOND, MAXIMUM_RELEASE_SPEED_PER_SECOND])
		return 0.0
	return release_speed


func _collect_vibration(raw_vibration: Variant) -> void:
	if typeof(raw_vibration) != TYPE_DICTIONARY:
		errors.append("input profile vibration must be an object")
		return
	var vibration: Dictionary = raw_vibration
	vibration_minimum_speed_kmh = float(vibration.get("minimum_speed_kmh", -1.0))
	if vibration_minimum_speed_kmh < 0.0 or vibration_minimum_speed_kmh > 100.0:
		errors.append("vibration minimum_speed_kmh %f is outside [0, 100]" % vibration_minimum_speed_kmh)
		vibration_minimum_speed_kmh = 0.0
	for band_name in ["piano", "off_asphalt", "front_grip_limit", "rear_grip_limit"]:
		if not vibration.has(band_name):
			errors.append("vibration band '%s' is missing" % band_name)
			continue
		var band: Variant = vibration[band_name]
		if typeof(band) != TYPE_DICTIONARY:
			errors.append("vibration band '%s' must be an object" % band_name)
			continue
		vibration_settings[band_name] = _read_vibration_band(band_name, band)


func _read_vibration_band(band_name: String, band: Dictionary) -> Dictionary:
	var result: Dictionary = {}
	result["weak_motor_magnitude"] = _read_magnitude(band_name, band, "weak_motor_magnitude")
	result["strong_motor_magnitude"] = _read_magnitude(band_name, band, "strong_motor_magnitude")
	var pulses := float(band.get("pulses_per_second", -1.0))
	if pulses < MINIMUM_PULSES_PER_SECOND or pulses > MAXIMUM_PULSES_PER_SECOND:
		errors.append("vibration band '%s' pulses_per_second %f is outside [%f, %f]" % [band_name, pulses, MINIMUM_PULSES_PER_SECOND, MAXIMUM_PULSES_PER_SECOND])
		pulses = 1.0 / 0.2
	result["pulses_per_second"] = pulses
	if band.has("engage_slip_angle_rad"):
		result["engage_slip_angle_rad"] = _read_slip_angle(band_name, "engage_slip_angle_rad", float(band["engage_slip_angle_rad"]))
		result["release_slip_angle_rad"] = _read_slip_angle(band_name, "release_slip_angle_rad", float(band.get("release_slip_angle_rad", 0.0)))
	return result


func _read_magnitude(band_name: String, band: Dictionary, field_name: String) -> float:
	var magnitude := float(band.get(field_name, -1.0))
	if magnitude < MINIMUM_MAGNITUDE or magnitude > MAXIMUM_MAGNITUDE:
		errors.append("vibration band '%s' %s %f is outside [%f, %f]" % [band_name, field_name, magnitude, MINIMUM_MAGNITUDE, MAXIMUM_MAGNITUDE])
		return 0.0
	return magnitude


func _read_slip_angle(band_name: String, field_name: String, angle: float) -> float:
	if angle < MINIMUM_SLIP_ANGLE_RAD or angle > MAXIMUM_SLIP_ANGLE_RAD:
		errors.append("vibration band '%s' %s %f is outside [%f, %f]" % [band_name, field_name, angle, MINIMUM_SLIP_ANGLE_RAD, MAXIMUM_SLIP_ANGLE_RAD])
		return MINIMUM_SLIP_ANGLE_RAD
	return angle
