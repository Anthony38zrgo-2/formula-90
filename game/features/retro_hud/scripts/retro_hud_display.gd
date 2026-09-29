class_name RetroHudDisplay
extends Control

const STATE_SCRIPT := preload("res://features/retro_hud/scripts/retro_hud_state.gd")
const CONFIG_SCRIPT := preload("res://features/retro_hud/scripts/retro_hud_config.gd")
const REVOLUTIONS_PER_MINUTE_SEGMENT_TEXTURE := preload("res://features/retro_hud/assets/rpm_segment.svg")
const FUEL_SEGMENT_TEXTURE := preload("res://features/retro_hud/assets/fuel_segment.svg")
const FUEL_ICON_TEXTURE := preload("res://features/retro_hud/assets/fuel_icon.svg")
const DIVIDER_TEXTURE := preload("res://features/retro_hud/assets/divider.svg")
const DIGITAL_SEGMENT_TEXTURE := preload("res://features/retro_hud/assets/digital_segment.svg")

const DESIGN_SIZE := Vector2(1448.0, 1086.0)
const REVOLUTIONS_PER_MINUTE_CENTER := Vector2(700.0, 760.0)
const REVOLUTIONS_PER_MINUTE_RADIUS := 570.0
const REVOLUTIONS_PER_MINUTE_START_DEGREES := 180.0
const REVOLUTIONS_PER_MINUTE_END_DEGREES := 328.0
const REVOLUTIONS_PER_MINUTE_LABEL_RADIUS := 650.0
const DIGITAL_SEGMENT_SOURCE_SIZE := Vector2(100.0, 24.0)

@export_file("*.json") var config_path := "res://features/retro_hud/config/retro_hud.json"
@export var apply_layout_from_config := false

var state: RetroHudState = STATE_SCRIPT.new()
var config: RetroHudConfig = CONFIG_SCRIPT.new()
var _peak_rpm := 0.0
var _design_origin := Vector2.ZERO
var _design_scale_factor := 1.0


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	state.changed.connect(_on_state_changed)
	reload_config()
	_peak_rpm = state.rpm
	queue_redraw()


func set_readout(
		speed_kph: float,
		rpm: float,
		gear_label: String,
		throttle: float = 0.0,
		brake: float = 0.0) -> void:
	state.set_readout(speed_kph, rpm, gear_label, throttle, brake)


func set_competition_readout(
		speed_kilometers_per_hour: float,
		engine_revolutions_per_minute: float,
		gear_label: String,
		oil_temperature_celsius: float,
		water_temperature_celsius: float,
		fuel_remaining_kg: float,
		fuel_capacity_kg: float,
		average_consumption_kg_per_lap: float,
		has_average_consumption: bool,
		fuel_delta_laps: float,
		has_fuel_delta: bool) -> void:
	state.set_competition_readout(
		speed_kilometers_per_hour,
		engine_revolutions_per_minute,
		gear_label,
		oil_temperature_celsius,
		water_temperature_celsius,
		fuel_remaining_kg,
		fuel_capacity_kg,
		average_consumption_kg_per_lap,
		has_average_consumption,
		fuel_delta_laps,
		has_fuel_delta)


func set_state(next_state: RetroHudState) -> void:
	if next_state == null or state == next_state:
		return
	if state.changed.is_connected(_on_state_changed):
		state.changed.disconnect(_on_state_changed)
	state = next_state
	state.changed.connect(_on_state_changed)
	queue_redraw()


func reload_config() -> void:
	config = CONFIG_SCRIPT.load_from_json(config_path)
	if apply_layout_from_config:
		scale = Vector2.ONE * config.display_scale
		visible = config.visible
	queue_redraw()


func apply_hud_layout(scale_value: float, visible_value: bool) -> void:
	visible = visible_value
	scale = Vector2.ONE * maxf(scale_value, 0.1)
	queue_redraw()


func _on_state_changed() -> void:
	_peak_rpm = maxf(_peak_rpm, state.rpm)
	queue_redraw()


func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		queue_redraw()


func _draw() -> void:
	if size.x <= 1.0 or size.y <= 1.0:
		return

	draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)
	draw_rect(Rect2(Vector2.ZERO, size), config.background_color)
	_design_scale_factor = minf(size.x / DESIGN_SIZE.x, size.y / DESIGN_SIZE.y)
	_design_origin = (size - DESIGN_SIZE * _design_scale_factor) * 0.5
	draw_set_transform(_design_origin, 0.0, Vector2.ONE * _design_scale_factor)

	_draw_revolutions_per_minute_scale()
	_draw_gear_and_speed()
	_draw_temperature_panel()
	_draw_fuel_bar()
	_draw_fuel_summary()

	draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)


func _draw_revolutions_per_minute_scale() -> void:
	var segment_count := maxi(config.revolutions_per_minute_segment_count, 2)
	for segment_index in range(segment_count):
		var segment_ratio := float(segment_index) / float(segment_count - 1)
		var angle_degrees := lerpf(
			REVOLUTIONS_PER_MINUTE_START_DEGREES,
			REVOLUTIONS_PER_MINUTE_END_DEGREES,
			segment_ratio)
		var angle_radians := deg_to_rad(angle_degrees)
		var radial_direction := Vector2(cos(angle_radians), sin(angle_radians))
		var segment_center := (
			REVOLUTIONS_PER_MINUTE_CENTER
			+ radial_direction * REVOLUTIONS_PER_MINUTE_RADIUS)
		var revolutions_per_minute := lerpf(
			config.revolutions_per_minute_minimum,
			config.revolutions_per_minute_maximum,
			segment_ratio)
		var redline_ratio := inverse_lerp(
			config.revolutions_per_minute_minimum,
			config.revolutions_per_minute_maximum,
			config.revolutions_per_minute_redline)
		var is_redline_segment := segment_ratio >= redline_ratio
		var segment_color := config.dial_color
		if is_redline_segment:
			segment_color = config.redline_color
		if state.engine_revolutions_per_minute < revolutions_per_minute:
			segment_color = (
				config.inactive_redline_color
				if is_redline_segment
				else config.inactive_color)
		_draw_rotated_texture(
			REVOLUTIONS_PER_MINUTE_SEGMENT_TEXTURE,
			segment_center,
			Vector2(92.0, 46.0),
			angle_radians + PI * 0.5,
			segment_color)

		var displayed_revolutions_per_minute := int(round(revolutions_per_minute / 1000.0))
		if displayed_revolutions_per_minute % 2 == 0 or segment_index == segment_count - 1:
			var label_center := (
				REVOLUTIONS_PER_MINUTE_CENTER
				+ radial_direction * REVOLUTIONS_PER_MINUTE_LABEL_RADIUS)
			_draw_condensed_text(
				str(displayed_revolutions_per_minute),
				label_center,
				38,
				config.dial_color)


func _draw_gear_and_speed() -> void:
	_draw_seven_segment_text(
		state.gear_label,
		Vector2(617.0, 540.0),
		Vector2(194.0, 300.0),
		config.digital_color)
	_draw_seven_segment_text(
		str(int(round(state.speed_kilometers_per_hour))),
		Vector2(846.0, 594.0),
		Vector2(57.0, 96.0),
		config.digital_color)
	_draw_condensed_text("KM/H", Vector2(844.0, 694.0), 42, config.dial_color)


func _draw_temperature_panel() -> void:
	_draw_condensed_text("OIL TEMP", Vector2(1297.0, 123.0), 34, config.dial_color)
	_draw_temperature_value(state.oil_temperature_celsius, Vector2(1288.0, 203.0))
	_draw_condensed_text("°C", Vector2(1390.0, 204.0), 32, config.dial_color)

	_draw_horizontal_divider(Vector2(1184.0, 264.0), 224.0)

	_draw_condensed_text("WATER TEMP", Vector2(1297.0, 304.0), 34, config.dial_color)
	_draw_temperature_value(state.water_temperature_celsius, Vector2(1288.0, 385.0))
	_draw_condensed_text("°C", Vector2(1390.0, 386.0), 32, config.dial_color)


func _draw_temperature_value(temperature_celsius: float, center: Vector2) -> void:
	if temperature_celsius < 0.0:
		_draw_condensed_text("--", center, 52, config.inactive_color)
		return
	_draw_seven_segment_text(
		str(int(round(temperature_celsius))),
		center,
		Vector2(54.0, 88.0),
		config.digital_color)


func _draw_fuel_bar() -> void:
	_draw_rotated_texture(
		FUEL_ICON_TEXTURE,
		Vector2(328.0, 783.0),
		Vector2(68.0, 78.0),
		0.0,
		config.dial_color)

	var fuel_track := Rect2(Vector2(383.0, 751.0), Vector2(690.0, 66.0))
	draw_rect(fuel_track, config.background_color, false, 3.0)

	var fuel_fraction := 0.0
	if state.fuel_capacity_kg > 0.0 and state.fuel_remaining_kg >= 0.0:
		fuel_fraction = clampf(state.fuel_remaining_kg / state.fuel_capacity_kg, 0.0, 1.0)
	var segment_count := maxi(config.fuel_segment_count, 1)
	var segment_gap := 7.0
	var segment_width := (
		(fuel_track.size.x - 34.0 - segment_gap * float(segment_count - 1))
		/ float(segment_count))
	var first_segment_x := fuel_track.position.x + 17.0
	var active_segment_count := int(floor(fuel_fraction * float(segment_count)))
	var fuel_color := (
		config.alert_color
		if fuel_fraction <= config.fuel_alert_fraction
		else config.dial_color)

	for segment_index in range(segment_count):
		var segment_center := Vector2(
			first_segment_x + (segment_width + segment_gap) * float(segment_index) + segment_width * 0.5,
			fuel_track.position.y + fuel_track.size.y * 0.5)
		var segment_color := (
			fuel_color
			if segment_index < active_segment_count
			else config.inactive_color)
		_draw_rotated_texture(
			FUEL_SEGMENT_TEXTURE,
			segment_center,
			Vector2(segment_width, fuel_track.size.y - 16.0),
			0.0,
			segment_color)


func _draw_fuel_summary() -> void:
	_draw_condensed_text("FUEL", Vector2(290.0, 887.0), 36, config.dial_color)
	_draw_condensed_text("AVG", Vector2(728.0, 887.0), 36, config.dial_color)
	_draw_condensed_text("DELTA", Vector2(1154.0, 887.0), 36, config.dial_color)

	_draw_vertical_divider(Vector2(509.0, 879.0), 123.0)
	_draw_vertical_divider(Vector2(943.0, 879.0), 123.0)

	if state.fuel_remaining_kg < 0.0:
		_draw_condensed_text("--.-", Vector2(291.0, 963.0), 44, config.inactive_color)
	else:
		var fuel_color := config.dial_color
		if state.fuel_capacity_kg > 0.0:
			var fuel_fraction := state.fuel_remaining_kg / state.fuel_capacity_kg
			if fuel_fraction <= config.fuel_alert_fraction:
				fuel_color = config.alert_color
		_draw_seven_segment_text(
			"%.1f" % state.fuel_remaining_kg,
			Vector2(292.0, 960.0),
			Vector2(52.0, 84.0),
			fuel_color)
	_draw_condensed_text("KG", Vector2(408.0, 968.0), 32, config.dial_color)

	if state.has_average_consumption:
		_draw_seven_segment_text(
			"%.2f" % state.average_consumption_kg_per_lap,
			Vector2(725.0, 960.0),
			Vector2(47.0, 82.0),
			config.digital_color)
	else:
		_draw_condensed_text("--.--", Vector2(724.0, 963.0), 40, config.inactive_color)
	_draw_condensed_text("KG/LAP", Vector2(866.0, 968.0), 29, config.dial_color)

	if state.has_fuel_delta:
		var fuel_delta_color := (
			config.alert_color
			if state.fuel_delta_laps < 0.0
			else config.digital_color)
		var fuel_delta_text := (
			"-%.2f" % absf(state.fuel_delta_laps)
			if state.fuel_delta_laps < 0.0
			else "%.2f" % state.fuel_delta_laps)
		_draw_seven_segment_text(
			fuel_delta_text,
			Vector2(1123.0, 960.0),
			Vector2(43.0, 79.0),
			fuel_delta_color)
	else:
		_draw_condensed_text("--.--", Vector2(1124.0, 963.0), 40, config.inactive_color)
	_draw_condensed_text("LAPS", Vector2(1335.0, 968.0), 30, config.dial_color)


func _draw_rotated_texture(
		texture: Texture2D,
		center: Vector2,
		dimensions: Vector2,
		rotation: float,
		color: Color) -> void:
	draw_set_transform(
		_design_origin + center * _design_scale_factor,
		rotation,
		Vector2.ONE * _design_scale_factor)
	draw_texture_rect(
		texture,
		Rect2(-dimensions * 0.5, dimensions),
		false,
		color)
	_restore_design_transform()


func _draw_seven_segment_text(
	text: String,
	center: Vector2,
	glyph_size: Vector2,
	color: Color) -> void:
	var glyph_gap := glyph_size.x * 0.11
	var text_width := 0.0
	for character_index in range(text.length()):
		var character := text.substr(character_index, 1)
		text_width += glyph_size.x * (0.34 if character == "." else 1.0)
		if character_index < text.length() - 1:
			text_width += glyph_gap

	var cursor_x := center.x - text_width * 0.5
	var glyph_top := center.y - glyph_size.y * 0.5
	for character_index in range(text.length()):
		var character := text.substr(character_index, 1)
		var glyph_width := _draw_seven_segment_glyph(
			character,
			Vector2(cursor_x, glyph_top),
			glyph_size,
			color)
		cursor_x += glyph_width
		if character_index < text.length() - 1:
			cursor_x += glyph_gap


func _draw_seven_segment_glyph(
	character: String,
	top_left: Vector2,
	glyph_size: Vector2,
	color: Color) -> float:
	if character == ".":
		draw_circle(
			top_left + Vector2(glyph_size.x * 0.15, glyph_size.y * 0.91),
			minf(glyph_size.x * 0.09, glyph_size.y * 0.045),
			color)
		return glyph_size.x * 0.34

	var horizontal_segment_length := glyph_size.x * 0.72
	var horizontal_segment_thickness := glyph_size.y * 0.115
	var vertical_segment_length := glyph_size.y * 0.34
	var vertical_segment_thickness := glyph_size.x * 0.14
	var active_segments := _segments_for_character(character)

	for segment_name in active_segments:
		match segment_name:
			"top_horizontal":
				_draw_digital_segment(
					top_left + Vector2(glyph_size.x * 0.5, glyph_size.y * 0.055),
					horizontal_segment_length,
					horizontal_segment_thickness,
					0.0,
					color)
			"upper_right":
				_draw_digital_segment(
					top_left + Vector2(glyph_size.x * 0.92, glyph_size.y * 0.275),
					vertical_segment_length,
					vertical_segment_thickness,
					PI * 0.5,
					color)
			"lower_right":
				_draw_digital_segment(
					top_left + Vector2(glyph_size.x * 0.92, glyph_size.y * 0.725),
					vertical_segment_length,
					vertical_segment_thickness,
					PI * 0.5,
					color)
			"bottom_horizontal":
				_draw_digital_segment(
					top_left + Vector2(glyph_size.x * 0.5, glyph_size.y * 0.945),
					horizontal_segment_length,
					horizontal_segment_thickness,
					0.0,
					color)
			"lower_left":
				_draw_digital_segment(
					top_left + Vector2(glyph_size.x * 0.08, glyph_size.y * 0.725),
					vertical_segment_length,
					vertical_segment_thickness,
					PI * 0.5,
					color)
			"upper_left":
				_draw_digital_segment(
					top_left + Vector2(glyph_size.x * 0.08, glyph_size.y * 0.275),
					vertical_segment_length,
					vertical_segment_thickness,
					PI * 0.5,
					color)
			"middle_horizontal":
				_draw_digital_segment(
					top_left + Vector2(glyph_size.x * 0.5, glyph_size.y * 0.5),
					horizontal_segment_length,
					horizontal_segment_thickness,
					0.0,
					color)

	return glyph_size.x


func _draw_digital_segment(
	center: Vector2,
	segment_length: float,
	segment_thickness: float,
	rotation: float,
	color: Color) -> void:
	draw_set_transform(
		_design_origin + center * _design_scale_factor,
		rotation,
		Vector2(
			segment_length / DIGITAL_SEGMENT_SOURCE_SIZE.x * _design_scale_factor,
			segment_thickness / DIGITAL_SEGMENT_SOURCE_SIZE.y * _design_scale_factor))
	draw_texture_rect(
		DIGITAL_SEGMENT_TEXTURE,
		Rect2(-DIGITAL_SEGMENT_SOURCE_SIZE * 0.5, DIGITAL_SEGMENT_SOURCE_SIZE),
		false,
		color)
	_restore_design_transform()


func _segments_for_character(character: String) -> PackedStringArray:
	match character:
		"0":
			return PackedStringArray([
				"top_horizontal",
				"upper_right",
				"lower_right",
				"bottom_horizontal",
				"lower_left",
				"upper_left"])
		"1":
			return PackedStringArray(["upper_right", "lower_right"])
		"2":
			return PackedStringArray([
				"top_horizontal",
				"upper_right",
				"middle_horizontal",
				"lower_left",
				"bottom_horizontal"])
		"3":
			return PackedStringArray([
				"top_horizontal",
				"upper_right",
				"middle_horizontal",
				"lower_right",
				"bottom_horizontal"])
		"4":
			return PackedStringArray([
				"upper_left",
				"middle_horizontal",
				"upper_right",
				"lower_right"])
		"5":
			return PackedStringArray([
				"top_horizontal",
				"upper_left",
				"middle_horizontal",
				"lower_right",
				"bottom_horizontal"])
		"6":
			return PackedStringArray([
				"top_horizontal",
				"upper_left",
				"middle_horizontal",
				"lower_left",
				"lower_right",
				"bottom_horizontal"])
		"7":
			return PackedStringArray([
				"top_horizontal",
				"upper_right",
				"lower_right"])
		"8":
			return PackedStringArray([
				"top_horizontal",
				"upper_right",
				"lower_right",
				"bottom_horizontal",
				"lower_left",
				"upper_left",
				"middle_horizontal"])
		"9":
			return PackedStringArray([
				"top_horizontal",
				"upper_right",
				"lower_right",
				"bottom_horizontal",
				"upper_left",
				"middle_horizontal"])
		"-":
			return PackedStringArray(["middle_horizontal"])
		"R":
			return PackedStringArray([
				"upper_left",
				"middle_horizontal",
				"lower_left",
				"lower_right"])
		"N":
			return PackedStringArray([
				"middle_horizontal",
				"lower_left",
				"lower_right"])
		_:
			return PackedStringArray()


func _draw_condensed_text(
	text: String,
	center: Vector2,
	font_size: int,
	color: Color) -> void:
	var font: Font = ThemeDB.fallback_font
	var text_width := font.get_string_size(
		text,
		HORIZONTAL_ALIGNMENT_LEFT,
		-1.0,
		font_size).x
	var horizontal_scale := 0.82
	var baseline := Vector2(
		(center.x - text_width * horizontal_scale * 0.5) / horizontal_scale,
		center.y + float(font_size) * 0.34)
	draw_set_transform(
		_design_origin,
		0.0,
		Vector2(
			horizontal_scale * _design_scale_factor,
			_design_scale_factor))
	draw_string(font, baseline, text, HORIZONTAL_ALIGNMENT_LEFT, -1.0, font_size, color)
	_restore_design_transform()


func _draw_vertical_divider(position: Vector2, height: float) -> void:
	draw_texture_rect(
		DIVIDER_TEXTURE,
		Rect2(position, Vector2(4.0, height)),
		false,
		config.divider_color)


func _draw_horizontal_divider(position: Vector2, width: float) -> void:
	draw_set_transform(
		_design_origin + (position + Vector2(width * 0.5, 0.0)) * _design_scale_factor,
		PI * 0.5,
		Vector2.ONE * _design_scale_factor)
	draw_texture_rect(
		DIVIDER_TEXTURE,
		Rect2(Vector2(-2.0, -width * 0.5), Vector2(4.0, width)),
		false,
		config.divider_color)
	_restore_design_transform()


func _restore_design_transform() -> void:
	draw_set_transform(
		_design_origin,
		0.0,
		Vector2.ONE * _design_scale_factor)
