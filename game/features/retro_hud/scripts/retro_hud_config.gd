class_name RetroHudConfig
extends RefCounted

const DEFAULT_PATH := "res://features/retro_hud/config/retro_hud.json"

var base_texture_path := "res://features/retro_hud/assets/retro_lcd_bezel_base.png"
var speed_unit := "KPH"
var rpm_max := 15000.0
var rpm_redline := 12200.0
var tach_segments := 14
var tach_start_degrees := 201.0
var tach_end_degrees := 339.0
var normal_color := Color("65ffe0")
var high_rpm_color := Color("f4d35e")
var redline_color := Color("ff5a5f")
var glyph_color := Color("061414")
var gear_color := Color("7fffe8")
var background_opacity := 1.0
var display_scale := 1.0
var display_position := Vector2.ZERO


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


func _apply(data: Dictionary) -> void:
	base_texture_path = str(data.get("base_texture", base_texture_path))
	speed_unit = str(data.get("speed_unit", speed_unit)).to_upper()
	rpm_max = maxf(float(data.get("rpm_max", rpm_max)), 1.0)
	rpm_redline = clampf(float(data.get("rpm_redline", rpm_redline)), 0.0, rpm_max)
	tach_segments = maxi(int(data.get("tach_segments", tach_segments)), 1)
	tach_start_degrees = float(data.get("tach_start_degrees", tach_start_degrees))
	tach_end_degrees = float(data.get("tach_end_degrees", tach_end_degrees))
	normal_color = _color(data.get("normal_color", "#65ffe0"), normal_color)
	high_rpm_color = _color(data.get("high_rpm_color", "#f4d35e"), high_rpm_color)
	redline_color = _color(data.get("redline_color", "#ff5a5f"), redline_color)
	glyph_color = _color(data.get("glyph_color", "#061414"), glyph_color)
	gear_color = _color(data.get("gear_color", "#7fffe8"), gear_color)
	background_opacity = clampf(float(data.get("background_opacity", background_opacity)), 0.0, 1.0)
	display_scale = maxf(float(data.get("scale", display_scale)), 0.1)
	var position_value: Variant = data.get("position", [0.0, 0.0])
	if position_value is Array and position_value.size() >= 2:
		display_position = Vector2(float(position_value[0]), float(position_value[1]))


func _color(value: Variant, fallback: Color) -> Color:
	if value is String and Color.html_is_valid(value):
		return Color(value)
	return fallback
