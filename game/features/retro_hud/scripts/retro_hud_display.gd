class_name RetroHudDisplay
extends Control

const STATE_SCRIPT := preload("res://features/retro_hud/scripts/retro_hud_state.gd")
const CONFIG_SCRIPT := preload("res://features/retro_hud/scripts/retro_hud_config.gd")
const DISPLAY_FONT := preload("res://assets/fonts/BarlowCondensed-Medium.ttf")
const REVOLUTIONS_PER_MINUTE_SEGMENT_TEXTURE := preload("res://features/retro_hud/assets/rpm_segment.svg")
const FUEL_SEGMENT_TEXTURE := preload("res://features/retro_hud/assets/fuel_segment.svg")
const FUEL_ICON_TEXTURE := preload("res://features/retro_hud/assets/fuel_icon.svg")
const DIVIDER_TEXTURE := preload("res://features/retro_hud/assets/divider.svg")

const DESIGN_SIZE := Vector2(1448.0, 1086.0)
const REVOLUTIONS_PER_MINUTE_CENTER := Vector2(650.0, 760.0)
const REVOLUTIONS_PER_MINUTE_RADIUS := 470.0
const REVOLUTIONS_PER_MINUTE_START_DEGREES := 180.0
const REVOLUTIONS_PER_MINUTE_END_DEGREES := 328.0
const REVOLUTIONS_PER_MINUTE_LABEL_RADIUS := 540.0
const INDICATOR_FONT_SIZE := 72
const GEAR_FONT_SIZE := 400
const SPEED_FONT_SIZE := 120

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
		has_fuel_delta: bool,
		oil_temperature_color: Color = Color.WHITE,
		water_temperature_color: Color = Color.WHITE) -> void:
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
		has_fuel_delta,
		oil_temperature_color,
		water_temperature_color)


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
	_draw_shift_lights()
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
		if revolutions_per_minute < config.urgent_shift_revolutions_per_minute:
			var segment_color := (
				config.dial_color
				if state.engine_revolutions_per_minute >= revolutions_per_minute
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
			_draw_competition_text(
				str(displayed_revolutions_per_minute),
				label_center,
				INDICATOR_FONT_SIZE,
				config.dial_color)
	_draw_redline_band()


func _draw_redline_band() -> void:
	var revolutions_per_minute_range := config.revolutions_per_minute_maximum - config.revolutions_per_minute_minimum
	var redline_range := config.revolutions_per_minute_maximum - config.urgent_shift_revolutions_per_minute
	if redline_range <= 0.0:
		return
	var redline_midpoint := (config.urgent_shift_revolutions_per_minute + config.revolutions_per_minute_maximum) * 0.5
	var midpoint_ratio := (redline_midpoint - config.revolutions_per_minute_minimum) / revolutions_per_minute_range
	var midpoint_angle := deg_to_rad(lerpf(
		REVOLUTIONS_PER_MINUTE_START_DEGREES,
		REVOLUTIONS_PER_MINUTE_END_DEGREES,
		midpoint_ratio))
	var arc_width := REVOLUTIONS_PER_MINUTE_RADIUS * deg_to_rad(
		(REVOLUTIONS_PER_MINUTE_END_DEGREES - REVOLUTIONS_PER_MINUTE_START_DEGREES)
		* redline_range / revolutions_per_minute_range)
	var center := REVOLUTIONS_PER_MINUTE_CENTER + Vector2(cos(midpoint_angle), sin(midpoint_angle)) * REVOLUTIONS_PER_MINUTE_RADIUS
	var redline_color := (
		config.redline_color
		if state.engine_revolutions_per_minute >= config.urgent_shift_revolutions_per_minute
		else config.inactive_redline_color)
	_draw_rotated_texture(
		REVOLUTIONS_PER_MINUTE_SEGMENT_TEXTURE,
		center,
		Vector2(arc_width, 46.0),
		midpoint_angle + PI * 0.5,
		redline_color)


func _draw_shift_lights() -> void:
	if state.engine_revolutions_per_minute >= config.ideal_shift_revolutions_per_minute:
		draw_circle(Vector2(130.0, 130.0), 57.0, config.inactive_color)
		draw_circle(Vector2(130.0, 130.0), 48.0, config.dial_color)
	if state.engine_revolutions_per_minute >= config.urgent_shift_revolutions_per_minute:
		draw_circle(Vector2(285.0, 130.0), 57.0, config.inactive_color)
		draw_circle(Vector2(285.0, 130.0), 48.0, config.alert_color)


func _draw_gear_and_speed() -> void:
	var gear_color := (
		config.alert_color
		if state.engine_revolutions_per_minute >= config.urgent_shift_revolutions_per_minute
		else config.digital_color)
	_draw_competition_text(
		state.gear_label,
		Vector2(617.0, 540.0),
		GEAR_FONT_SIZE,
		gear_color)
	_draw_competition_text(
		str(int(round(state.speed_kilometers_per_hour))),
		Vector2(846.0, 594.0),
		SPEED_FONT_SIZE,
		config.digital_color)
	_draw_competition_text("KM/H", Vector2(844.0, 694.0), INDICATOR_FONT_SIZE, config.dial_color)


func _draw_temperature_panel() -> void:
	_draw_competition_text("OIL TEMP", Vector2(1257.0, 123.0), INDICATOR_FONT_SIZE, config.dial_color)
	_draw_temperature_value(state.oil_temperature_celsius, state.oil_temperature_color, Vector2(1268.0, 203.0))
	_draw_competition_text("°C", Vector2(1370.0, 204.0), INDICATOR_FONT_SIZE, config.dial_color)

	_draw_horizontal_divider(Vector2(1164.0, 264.0), 224.0)

	_draw_competition_text("WATER TEMP", Vector2(1257.0, 304.0), INDICATOR_FONT_SIZE, config.dial_color)
	_draw_temperature_value(state.water_temperature_celsius, state.water_temperature_color, Vector2(1268.0, 385.0))
	_draw_competition_text("°C", Vector2(1370.0, 386.0), INDICATOR_FONT_SIZE, config.dial_color)


func _draw_temperature_value(temperature_celsius: float, temperature_color: Color, center: Vector2) -> void:
	if temperature_celsius < 0.0:
		_draw_competition_text("--", center, INDICATOR_FONT_SIZE, config.inactive_color)
		return
	_draw_competition_text(
		str(int(round(temperature_celsius))),
		center,
		INDICATOR_FONT_SIZE,
		temperature_color)


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
	var fuel_color := (
		config.alert_color
		if fuel_fraction <= config.fuel_alert_fraction
		else config.dial_color)

	for segment_index in range(segment_count):
		var segment_center := Vector2(
			first_segment_x + (segment_width + segment_gap) * float(segment_index) + segment_width * 0.5,
			fuel_track.position.y + fuel_track.size.y * 0.5)
		var segment_fill_fraction := clampf(fuel_fraction * float(segment_count) - float(segment_index), 0.0, 1.0)
		_draw_fuel_segment(
			segment_center,
			Vector2(segment_width, fuel_track.size.y - 16.0),
			segment_fill_fraction,
			fuel_color)


func _draw_fuel_segment(center: Vector2, dimensions: Vector2, fill_fraction: float, active_color: Color) -> void:
	_draw_rotated_texture(FUEL_SEGMENT_TEXTURE, center, dimensions, 0.0, config.inactive_color)
	if fill_fraction <= 0.0:
		return
	draw_set_transform(_design_origin + center * _design_scale_factor, 0.0, Vector2.ONE * _design_scale_factor)
	draw_texture_rect_region(
		FUEL_SEGMENT_TEXTURE,
		Rect2(-dimensions * 0.5, Vector2(dimensions.x * fill_fraction, dimensions.y)),
		Rect2(Vector2.ZERO, Vector2(FUEL_SEGMENT_TEXTURE.get_width() * fill_fraction, FUEL_SEGMENT_TEXTURE.get_height())),
		active_color)
	_restore_design_transform()


func _draw_fuel_summary() -> void:
	_draw_competition_text("FUEL", Vector2(290.0, 887.0), INDICATOR_FONT_SIZE, config.dial_color)
	_draw_competition_text("AVG", Vector2(728.0, 887.0), INDICATOR_FONT_SIZE, config.dial_color)
	_draw_competition_text("DELTA", Vector2(1154.0, 887.0), INDICATOR_FONT_SIZE, config.dial_color)

	_draw_vertical_divider(Vector2(509.0, 879.0), 123.0)
	_draw_vertical_divider(Vector2(943.0, 879.0), 123.0)

	if state.fuel_remaining_kg < 0.0:
		_draw_competition_text("--.-", Vector2(291.0, 963.0), INDICATOR_FONT_SIZE, config.inactive_color)
	else:
		var fuel_color := config.dial_color
		if state.fuel_capacity_kg > 0.0:
			var fuel_fraction := state.fuel_remaining_kg / state.fuel_capacity_kg
			if fuel_fraction <= config.fuel_alert_fraction:
				fuel_color = config.alert_color
		_draw_competition_text(
			"%.1f" % state.fuel_remaining_kg,
			Vector2(292.0, 960.0),
			INDICATOR_FONT_SIZE,
			fuel_color)
	_draw_competition_text("KG", Vector2(408.0, 968.0), INDICATOR_FONT_SIZE, config.dial_color)

	if state.has_average_consumption:
		_draw_competition_text(
			"%.2f" % state.average_consumption_kg_per_lap,
			Vector2(725.0, 960.0),
			INDICATOR_FONT_SIZE,
			config.digital_color)
	else:
		_draw_competition_text("--.--", Vector2(724.0, 963.0), INDICATOR_FONT_SIZE, config.inactive_color)
	_draw_competition_text("KG/LAP", Vector2(866.0, 968.0), INDICATOR_FONT_SIZE, config.dial_color)

	if state.has_fuel_delta:
		var fuel_delta_color := (
			config.alert_color
			if state.fuel_delta_laps < 0.0
			else config.digital_color)
		var fuel_delta_text := (
			"-%.2f" % absf(state.fuel_delta_laps)
			if state.fuel_delta_laps < 0.0
			else "%.2f" % state.fuel_delta_laps)
		_draw_competition_text(
			fuel_delta_text,
			Vector2(1078.0, 960.0),
			INDICATOR_FONT_SIZE,
			fuel_delta_color)
	else:
		_draw_competition_text("--.--", Vector2(1078.0, 963.0), INDICATOR_FONT_SIZE, config.inactive_color)
	_draw_competition_text("LAPS", Vector2(1305.0, 968.0), INDICATOR_FONT_SIZE, config.dial_color)


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


func _draw_competition_text(
	text: String,
	center: Vector2,
	font_size: int,
	color: Color) -> void:
	var text_width := DISPLAY_FONT.get_string_size(
		text,
		HORIZONTAL_ALIGNMENT_LEFT,
		-1.0,
		font_size).x
	var baseline := Vector2(
		center.x - text_width * 0.5,
		center.y + (DISPLAY_FONT.get_ascent(font_size) - DISPLAY_FONT.get_descent(font_size)) * 0.5)
	draw_set_transform(
		_design_origin,
		0.0,
		Vector2.ONE * _design_scale_factor)
	draw_string(DISPLAY_FONT, baseline, text, HORIZONTAL_ALIGNMENT_LEFT, -1.0, font_size, color)
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
