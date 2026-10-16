extends Control

const PANEL_SIZE := Vector2(396.0, 116.0)
const ACCELERATOR_COLOR := Color("48df94")
const BRAKE_COLOR := Color("ff665c")
const STEERING_COLOR := Color("ffd477")

var steering_fraction := 0.0
var accelerator_fraction := 0.0
var brake_fraction := 0.0
var panel_background := StyleBoxFlat.new()

func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	custom_minimum_size = PANEL_SIZE
	size = PANEL_SIZE
	panel_background.bg_color = Color(0.025, 0.035, 0.05, 0.90)
	panel_background.border_color = Color(0.45, 0.52, 0.62, 0.45)
	panel_background.set_border_width_all(1)
	panel_background.set_corner_radius_all(12)
	queue_redraw()

func set_controls(steering: float, accelerator: float, brake: float) -> void:
	steering_fraction = clampf(steering, -1.0, 1.0)
	accelerator_fraction = clampf(accelerator, 0.0, 1.0)
	brake_fraction = clampf(brake, 0.0, 1.0)
	queue_redraw()

func _draw() -> void:
	draw_style_box(panel_background, Rect2(Vector2.ZERO, PANEL_SIZE))
	var font := ThemeDB.fallback_font
	var wheel_center := Vector2(62.0, 64.0)
	var wheel_rotation := -steering_fraction * PI * 0.75
	draw_string(font, Vector2(26.0, 21.0), "VOLANTE", HORIZONTAL_ALIGNMENT_LEFT, -1.0, 13, Color(0.78, 0.83, 0.90))
	draw_arc(wheel_center, 27.0, 0.0, TAU, 64, Color(0.92, 0.95, 1.0), 3.0, true)
	for spoke_angle in [-PI * 0.5, PI / 6.0, PI * 5.0 / 6.0]:
		var direction := Vector2(cos(spoke_angle + wheel_rotation), sin(spoke_angle + wheel_rotation))
		draw_line(wheel_center + direction * 6.0, wheel_center + direction * 25.0, STEERING_COLOR if spoke_angle < 0.0 else Color(0.70, 0.76, 0.85), 3.0, true)
	draw_circle(wheel_center, 6.0, Color(0.45, 0.51, 0.61), true, -1.0, true)
	var steering_text := "%d%%" % roundi(absf(steering_fraction) * 100.0)
	if absf(steering_fraction) > 0.005:
		steering_text = ("IZQ " if steering_fraction > 0.0 else "DER ") + steering_text
	draw_string(font, Vector2(16.0, 106.0), steering_text, HORIZONTAL_ALIGNMENT_CENTER, 92.0, 13, STEERING_COLOR)
	draw_pedal(font, "ACELERADOR", accelerator_fraction, 30.0, ACCELERATOR_COLOR)
	draw_pedal(font, "FRENO", brake_fraction, 77.0, BRAKE_COLOR)

func draw_pedal(font: Font, label: String, fraction: float, baseline: float, color: Color) -> void:
	draw_string(font, Vector2(126.0, baseline), label, HORIZONTAL_ALIGNMENT_LEFT, -1.0, 14, Color(0.88, 0.92, 0.98))
	draw_string(font, Vector2(320.0, baseline), "%d%%" % roundi(fraction * 100.0), HORIZONTAL_ALIGNMENT_RIGHT, 56.0, 15, color)
	var bar := Rect2(126.0, baseline + 9.0, 250.0, 15.0)
	draw_rect(bar, Color(0.16, 0.20, 0.26))
	draw_rect(Rect2(bar.position, Vector2(bar.size.x * fraction, bar.size.y)), color)
