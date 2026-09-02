class_name RetroHudState
extends RefCounted

signal changed

var speed_kph := 0.0
var rpm := 0.0
var gear_label := "N"
var throttle := 0.0
var brake := 0.0


func set_readout(
		next_speed_kph: float,
		next_rpm: float,
		next_gear_label: String,
		next_throttle: float = 0.0,
		next_brake: float = 0.0) -> void:
	var normalized_speed := maxf(next_speed_kph, 0.0)
	var normalized_rpm := maxf(next_rpm, 0.0)
	var normalized_gear := next_gear_label if not next_gear_label.is_empty() else "N"
	var normalized_throttle := clampf(next_throttle, 0.0, 1.0)
	var normalized_brake := clampf(next_brake, 0.0, 1.0)
	if (
			is_equal_approx(speed_kph, normalized_speed)
			and is_equal_approx(rpm, normalized_rpm)
			and gear_label == normalized_gear
			and is_equal_approx(throttle, normalized_throttle)
			and is_equal_approx(brake, normalized_brake)):
		return
	speed_kph = normalized_speed
	rpm = normalized_rpm
	gear_label = normalized_gear
	throttle = normalized_throttle
	brake = normalized_brake
	changed.emit()
