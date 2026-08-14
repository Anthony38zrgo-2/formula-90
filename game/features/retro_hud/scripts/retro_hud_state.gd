class_name RetroHudState
extends RefCounted

signal changed

var speed_kph := 0.0
var rpm := 0.0
var gear_label := "N"


func set_readout(next_speed_kph: float, next_rpm: float, next_gear_label: String) -> void:
	var normalized_speed := maxf(next_speed_kph, 0.0)
	var normalized_rpm := maxf(next_rpm, 0.0)
	var normalized_gear := next_gear_label if not next_gear_label.is_empty() else "N"
	if is_equal_approx(speed_kph, normalized_speed) and is_equal_approx(rpm, normalized_rpm) and gear_label == normalized_gear:
		return
	speed_kph = normalized_speed
	rpm = normalized_rpm
	gear_label = normalized_gear
	changed.emit()
