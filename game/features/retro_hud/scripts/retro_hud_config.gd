class_name RetroHudConfig
extends RefCounted

const DEFAULT_PATH := "res://features/retro_hud/config/retro_hud.json"

var revolutions_per_minute_minimum := 0.0
var revolutions_per_minute_maximum := 19000.0
var revolutions_per_minute_redline := 18000.0
var ideal_shift_revolutions_per_minute := 17400.0
var urgent_shift_revolutions_per_minute := 17800.0
var revolutions_per_minute_segment_count := 20
var fuel_segment_count := 12
var fuel_alert_fraction := 0.05
var background_color := Color("#06152540")
var dial_color := Color("#f4f5f6")
var digital_color := Color("#f4f5f6")
var inactive_color := Color("#183149")
var redline_color := Color("#f0182d")
var inactive_redline_color := Color("#67202b")
var alert_color := Color("#f0182d")
var divider_color := Color("#ffffff")
var display_scale := 1.0
var visible := true
var rpm_min := 0.0
var rpm_max := 19000.0
var rpm_redline := 18000.0
var speed_segments := 19
var peak_activation_rpm := 12000.0


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
	revolutions_per_minute_minimum = maxf(
		float(data.get("revolutions_per_minute_minimum", revolutions_per_minute_minimum)),
		0.0)
	revolutions_per_minute_maximum = maxf(
		float(data.get("revolutions_per_minute_maximum", revolutions_per_minute_maximum)),
		revolutions_per_minute_minimum + 1.0)
	revolutions_per_minute_redline = clampf(
		float(data.get("revolutions_per_minute_redline", revolutions_per_minute_redline)),
		revolutions_per_minute_minimum,
		revolutions_per_minute_maximum)
	ideal_shift_revolutions_per_minute = clampf(
		float(data.get("ideal_shift_revolutions_per_minute", ideal_shift_revolutions_per_minute)),
		revolutions_per_minute_minimum,
		revolutions_per_minute_redline)
	urgent_shift_revolutions_per_minute = clampf(
		float(data.get("urgent_shift_revolutions_per_minute", urgent_shift_revolutions_per_minute)),
		ideal_shift_revolutions_per_minute,
		revolutions_per_minute_redline)
	rpm_min = revolutions_per_minute_minimum
	rpm_max = revolutions_per_minute_maximum
	rpm_redline = revolutions_per_minute_redline
	peak_activation_rpm = clampf(peak_activation_rpm, rpm_min, rpm_max)
	speed_segments = clampi(speed_segments, 4, 19)
	revolutions_per_minute_segment_count = clampi(
		int(data.get("revolutions_per_minute_segment_count", revolutions_per_minute_segment_count)),
		2,
		40)
	fuel_segment_count = clampi(int(data.get("fuel_segment_count", fuel_segment_count)), 4, 24)
	fuel_alert_fraction = clampf(float(data.get("fuel_alert_fraction", fuel_alert_fraction)), 0.0, 1.0)
	display_scale = maxf(float(data.get("scale", display_scale)), 0.1)
	visible = bool(data.get("visible", visible))

	background_color = _color(data.get("background_color", "#06152540"), background_color)
	dial_color = _color(data.get("dial_color", "#f4f5f6"), dial_color)
	digital_color = _color(data.get("digital_color", "#f4f5f6"), digital_color)
	inactive_color = _color(data.get("inactive_color", "#183149"), inactive_color)
	redline_color = _color(data.get("redline_color", "#f0182d"), redline_color)
	inactive_redline_color = _color(data.get("inactive_redline_color", "#67202b"), inactive_redline_color)
	alert_color = _color(data.get("alert_color", "#f0182d"), alert_color)
	divider_color = _color(data.get("divider_color", "#ffffff"), divider_color)


func _color(value: Variant, fallback: Color) -> Color:
	if value is String and Color.html_is_valid(value):
		return Color(value)
	return fallback
