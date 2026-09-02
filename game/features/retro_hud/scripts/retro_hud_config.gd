class_name RetroHudConfig
extends RefCounted

const DEFAULT_PATH := "res://features/retro_hud/config/retro_hud.json"

var speed_unit := "km/h"
var rpm_min := 6000.0
var rpm_max := 20000.0
var rpm_redline := 18000.0
var speed_max := 360.0
var speed_segments := 20
var peak_hold_seconds := 2.0
var peak_activation_rpm := 12000.0
var peak_return_rpm_per_second := 7500.0
var background_opacity := 0.82
var display_scale := 1.0
var display_position := Vector2.ZERO
var visible := true

var dial_color := Color("f4f4f4")
var dial_shadow_color := Color("080808d9")
var peak_color := Color("dc1f27")
var throttle_color := Color("19ed39")
var brake_color := Color("f02a2a")
var speed_green := Color("00ec2e")
var speed_yellow := Color("ffd702")
var speed_orange := Color("fe9a0d")
var speed_red := Color("ea0000")
var inactive_multiplier := 0.55


static func load_from_json(path: String = DEFAULT_PATH) -> RetroHudConfig:
	var config := RetroHudConfig.new()
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		push_warning("Retro HUD config missing: %s. Using defaults." % path)
		return config
	var parsed: Variant = JSON.parse_string(file.get_as_text())
	if not (parsed is Dictionary):
		push_warning("Retro HUD config is invalid: %s. Using defaults." % path)
		return config
	config._apply(parsed)
	return config


func apply(data: Dictionary) -> void:
	_apply(data)


func _apply(data: Dictionary) -> void:
	speed_unit = str(data.get("speed_unit", speed_unit))
	rpm_min = maxf(float(data.get("rpm_min", rpm_min)), 0.0)
	rpm_max = maxf(float(data.get("rpm_max", rpm_max)), rpm_min + 1.0)
	rpm_redline = clampf(float(data.get("rpm_redline", rpm_redline)), rpm_min, rpm_max)
	speed_max = maxf(float(data.get("speed_max", speed_max)), 1.0)
	speed_segments = clampi(int(data.get("speed_segments", speed_segments)), 4, 19)
	peak_hold_seconds = maxf(float(data.get("peak_hold_seconds", peak_hold_seconds)), 0.0)
	peak_activation_rpm = clampf(float(data.get("peak_activation_rpm", peak_activation_rpm)), rpm_min, rpm_max)
	peak_return_rpm_per_second = maxf(float(data.get("peak_return_rpm_per_second", peak_return_rpm_per_second)), 1.0)
	background_opacity = clampf(float(data.get("background_opacity", background_opacity)), 0.0, 1.0)
	display_scale = maxf(float(data.get("scale", display_scale)), 0.1)
	inactive_multiplier = clampf(float(data.get("inactive_multiplier", inactive_multiplier)), 0.0, 1.0)
	visible = bool(data.get("visible", visible))

	dial_color = _color(data.get("dial_color", "#f4f4f4"), dial_color)
	dial_shadow_color = _color(data.get("dial_shadow_color", "#080808d9"), dial_shadow_color)
	peak_color = _color(data.get("peak_color", "#dc1f27"), peak_color)
	throttle_color = _color(data.get("throttle_color", "#19ed39"), throttle_color)
	brake_color = _color(data.get("brake_color", "#f02a2a"), brake_color)
	speed_green = _color(data.get("speed_green", "#00ec2e"), speed_green)
	speed_yellow = _color(data.get("speed_yellow", "#ffd702"), speed_yellow)
	speed_orange = _color(data.get("speed_orange", "#fe9a0d"), speed_orange)
	speed_red = _color(data.get("speed_red", "#ea0000"), speed_red)

	var position_value: Variant = data.get("position", [0.0, 0.0])
	if position_value is Array and position_value.size() >= 2:
		display_position = Vector2(float(position_value[0]), float(position_value[1]))


func _color(value: Variant, fallback: Color) -> Color:
	if value is String and Color.html_is_valid(value):
		return Color(value)
	return fallback
