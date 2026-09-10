class_name RetroHudDisplay
extends Control

## Procedural reinterpretation of the 2004-2008 Formula broadcast telemetry HUD.
## No texture frames from the Assetto Corsa mod are required at runtime.

const STATE_SCRIPT := preload("res://features/retro_hud/scripts/retro_hud_state.gd")
const CONFIG_SCRIPT := preload("res://features/retro_hud/scripts/retro_hud_config.gd")

const DESIGN_SIZE := Vector2(353.0, 500.0)
const RPM_DIAL_CENTER := Vector2(179.0, 178.0)
const RPM_DIAL_RADIUS := 108.0
const RPM_START_DEG := 91.0
const RPM_END_DEG := 357.0

@export_file("*.json") var config_path := "res://features/retro_hud/config/retro_hud.json"
@export var apply_layout_from_config := false

var state = STATE_SCRIPT.new()
var config = CONFIG_SCRIPT.new()
var _peak_rpm := 0.0
var _peak_hold_remaining := 0.0


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	state.changed.connect(_on_state_changed)
	reload_config()
	_peak_rpm = state.rpm
	set_process(true)
	queue_redraw()


func set_readout(
		speed_kph: float,
		rpm: float,
		gear_label: String,
		throttle: float = 0.0,
		brake: float = 0.0) -> void:
	state.set_readout(speed_kph, rpm, gear_label, throttle, brake)


func set_state(next_state: RefCounted) -> void:
	if next_state == null or state == next_state:
		return
	if state.changed.is_connected(_on_state_changed):
		state.changed.disconnect(_on_state_changed)
	state = next_state
	state.changed.connect(_on_state_changed)
	_peak_rpm = state.rpm
	_peak_hold_remaining = 0.0
	queue_redraw()


func reload_config() -> void:
	config = CONFIG_SCRIPT.load_from_json(config_path)
	if apply_layout_from_config:
		position = config.display_position
		scale = Vector2.ONE * config.display_scale
	queue_redraw()


func _process(delta: float) -> void:
	var changed := false
	if state.rpm >= _peak_rpm:
		if not is_equal_approx(_peak_rpm, state.rpm):
			_peak_rpm = state.rpm
			changed = true
		_peak_hold_remaining = config.peak_hold_seconds if state.rpm >= config.peak_activation_rpm else 0.0
	elif _peak_hold_remaining > 0.0:
		_peak_hold_remaining = maxf(_peak_hold_remaining - delta, 0.0)
	elif _peak_rpm > state.rpm:
		_peak_rpm = maxf(state.rpm, _peak_rpm - config.peak_return_rpm_per_second * delta)
		changed = true
	if state.rpm < config.peak_activation_rpm and _peak_hold_remaining <= 0.0 and not is_equal_approx(_peak_rpm, state.rpm):
		_peak_rpm = state.rpm
		changed = true
	if changed:
		queue_redraw()


func _on_state_changed() -> void:
	if state.rpm > _peak_rpm:
		_peak_rpm = state.rpm
		_peak_hold_remaining = config.peak_hold_seconds if state.rpm >= config.peak_activation_rpm else 0.0
	queue_redraw()


func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		queue_redraw()


func _draw() -> void:
	if size.x <= 1.0 or size.y <= 1.0:
		return

	var scale_factor := minf(size.x / DESIGN_SIZE.x, size.y / DESIGN_SIZE.y)
	var origin := (size - DESIGN_SIZE * scale_factor) * 0.5
	draw_set_transform(origin, 0.0, Vector2.ONE * scale_factor)

	_draw_rpm_cluster()
	_draw_gear_and_pedals()
	_draw_speed_cluster()

	draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)


func _draw_rpm_cluster() -> void:
	var shadow := config.dial_shadow_color
	shadow.a *= config.background_opacity
	draw_circle(RPM_DIAL_CENTER, RPM_DIAL_RADIUS + 24.0, shadow)

	# The original TV graphic uses several close concentric white rings.
	for radius_offset in [-16.0, 0.0, 7.0]:
		draw_arc(
			RPM_DIAL_CENTER,
			RPM_DIAL_RADIUS + radius_offset,
			deg_to_rad(RPM_START_DEG),
			deg_to_rad(RPM_END_DEG),
			72,
			config.dial_color,
			2.2,
			true)

	var font: Font = ThemeDB.fallback_font
	var major_count := int(round((config.rpm_max - config.rpm_min) / 1000.0))
	major_count = maxi(major_count, 1)
	for index in range(major_count + 1):
		var ratio := float(index) / float(major_count)
		var angle := deg_to_rad(lerpf(RPM_START_DEG, RPM_END_DEG, ratio))
		var direction := Vector2(cos(angle), sin(angle))
		var tick_outer := RPM_DIAL_RADIUS + 22.0
		var tick_inner := RPM_DIAL_RADIUS + 9.0
		draw_line(
			RPM_DIAL_CENTER + direction * tick_inner,
			RPM_DIAL_CENTER + direction * tick_outer,
			config.dial_color,
			2.2,
			true)

		var rpm_number := int(round((config.rpm_min + 1000.0 * index) / 1000.0))
		var label := str(rpm_number)
		var label_pos := RPM_DIAL_CENTER + direction * (RPM_DIAL_RADIUS + 44.0)
		_draw_centered_text(font, label_pos, label, 15, config.dial_color)

		if index < major_count:
			var minor_angle := deg_to_rad(lerpf(RPM_START_DEG, RPM_END_DEG, ratio + 0.5 / float(major_count)))
			var minor_direction := Vector2(cos(minor_angle), sin(minor_angle))
			draw_line(
				RPM_DIAL_CENTER + minor_direction * (RPM_DIAL_RADIUS + 11.0),
				RPM_DIAL_CENTER + minor_direction * (RPM_DIAL_RADIUS + 19.0),
				config.dial_color,
				1.4,
				true)

	# Peak-hold needle first, current RPM needle on top.
	_draw_rpm_needle(_peak_rpm, config.peak_color, 3.6, RPM_DIAL_RADIUS - 8.0)
	_draw_rpm_needle(state.rpm, config.dial_color, 4.8, RPM_DIAL_RADIUS - 17.0)
	draw_circle(RPM_DIAL_CENTER, 13.0, config.dial_color)
	draw_circle(RPM_DIAL_CENTER, 5.4, Color(0.82, 0.82, 0.82, 1.0))


func _draw_rpm_needle(value: float, color: Color, width: float, length: float) -> void:
	var ratio := inverse_lerp(config.rpm_min, config.rpm_max, clampf(value, config.rpm_min, config.rpm_max))
	var angle := deg_to_rad(lerpf(RPM_START_DEG, RPM_END_DEG, ratio))
	var direction := Vector2(cos(angle), sin(angle))
	var tail := RPM_DIAL_CENTER - direction * 7.0
	var tip := RPM_DIAL_CENTER + direction * length
	draw_line(tail, tip, color, width, true)


func _draw_gear_and_pedals() -> void:
	var font: Font = ThemeDB.fallback_font
	var panel_x := 208.0
	var panel_width := 145.0
	var row_height := 40.0

	# Gear plate.
	var gear_rect := Rect2(panel_x, 183.0, panel_width, 40.0)
	draw_rect(gear_rect, Color.WHITE)
	draw_string(font, Vector2(panel_x + 23.0, 211.0), "Gear", HORIZONTAL_ALIGNMENT_LEFT, -1.0, 17, Color.BLACK)
	draw_string(font, Vector2(panel_x + 99.0, 211.0), state.gear_label, HORIZONTAL_ALIGNMENT_CENTER, 38.0, 21, Color.BLACK)

	# Pedal bars. They preserve the horizontal TV layout but now use true analog values.
	var throttle_rect := Rect2(panel_x, 223.0, panel_width, row_height)
	_draw_input_bar(throttle_rect, state.throttle, "Throttle", config.throttle_color, font)
	var brake_rect := Rect2(panel_x, 263.0, panel_width, row_height)
	_draw_input_bar(brake_rect, state.brake, "Brake", config.brake_color, font)


func _draw_input_bar(rect: Rect2, amount: float, label: String, color: Color, font: Font) -> void:
	# Original broadcast graphic leaves the row transparent and reveals a colored fill.
	if amount > 0.0:
		draw_rect(Rect2(rect.position, Vector2(rect.size.x * clampf(amount, 0.0, 1.0), rect.size.y)), color)
	_draw_text_with_shadow(font, rect.position + Vector2(17.0, 28.0), label, 18, Color.WHITE)


func _draw_speed_cluster() -> void:
	var font: Font = ThemeDB.fallback_font
	# The source graphic has 19 visual segments: 10 straight green bars followed
	# by 3 yellow, 3 orange and 3 red curved/stacked pieces.
	var segments := 19
	var active_segments := int(round(clampf(state.speed_kph / config.speed_max, 0.0, 1.0) * float(segments)))
	for index in range(segments):
		var base_color := _speed_color_for_index(index)
		if index >= active_segments:
			base_color = Color(
				base_color.r * config.inactive_multiplier,
				base_color.g * config.inactive_multiplier,
				base_color.b * config.inactive_multiplier,
				base_color.a)
		_draw_speed_segment(index, base_color)

	# Numeric scale follows the source layout closely, but the current value is live.
	_draw_text_with_shadow(font, Vector2(15.0, 483.0), "%d %s" % [int(round(state.speed_kph)), config.speed_unit], 16, config.dial_color)
	_draw_centered_text(font, Vector2(202.0, 472.0), "200", 14, config.dial_color)
	_draw_centered_text(font, Vector2(284.0, 434.0), "260", 14, config.dial_color)
	_draw_centered_text(font, Vector2(320.0, 378.0), "320", 14, config.dial_color)
	_draw_centered_text(font, Vector2(327.0, 319.0), "340", 14, config.dial_color)


func _draw_speed_segment(index: int, color: Color) -> void:
	if index < 10:
		# Exact straight section spacing measured from the 353x500 reference sprites.
		draw_rect(Rect2(16.0 + 17.0 * float(index), 403.0, 12.0, 45.0), color)
		return

	var curved_segments := [
		# Yellow
		[[200, 400], [187, 403], [189, 446], [207, 444]],
		[[213, 395], [205, 402], [215, 442], [232, 434]],
		[[227, 386], [219, 392], [239, 432], [255, 422]],
		# Orange
		[[236, 378], [231, 383], [260, 417], [273, 405]],
		[[242, 369], [240, 374], [278, 399], [287, 385], [247, 368]],
		[[250, 354], [247, 362], [290, 379], [296, 362]],
		# Red
		[[254, 337], [253, 348], [299, 355], [299, 339]],
		[[255, 322], [255, 331], [299, 333], [299, 322]],
		[[255, 309], [255, 318], [299, 318], [299, 309]]
	]
	var source_points: Array = curved_segments[index - 10]
	var points := PackedVector2Array()
	for point in source_points:
		points.append(Vector2(float(point[0]), float(point[1])))
	draw_colored_polygon(points, color)


func _speed_color_for_index(index: int) -> Color:
	if index < 10:
		return config.speed_green
	if index < 13:
		return config.speed_yellow
	if index < 16:
		return config.speed_orange
	return config.speed_red


func _draw_centered_text(font: Font, center: Vector2, text: String, font_size: int, color: Color) -> void:
	var text_size := font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1.0, font_size)
	var baseline := center - Vector2(text_size.x * 0.5, -text_size.y * 0.32)
	_draw_text_with_shadow(font, baseline, text, font_size, color)


func _draw_text_with_shadow(font: Font, baseline: Vector2, text: String, font_size: int, color: Color) -> void:
	draw_string(font, baseline + Vector2(1.5, 1.5), text, HORIZONTAL_ALIGNMENT_LEFT, -1.0, font_size, Color(0.0, 0.0, 0.0, 0.88))
	draw_string(font, baseline, text, HORIZONTAL_ALIGNMENT_LEFT, -1.0, font_size, color)
