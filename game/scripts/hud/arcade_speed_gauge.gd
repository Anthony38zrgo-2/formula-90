class_name ArcadeSpeedGauge
extends Control

const SPEED_LIMIT_KPH := 360.0
const ARC_START := deg_to_rad(180.0)
const ARC_END := deg_to_rad(348.0)
const ARC_SHADOW := Color(0.03, 0.02, 0.08, 0.92)
const ARC_YELLOW := Color(1.0, 0.82, 0.08, 1.0)
const ARC_ORANGE := Color(1.0, 0.46, 0.05, 1.0)
const ARC_RED := Color(1.0, 0.14, 0.20, 1.0)
const ARC_PURPLE := Color(0.41, 0.07, 0.78, 1.0)

var speed_kph := 0.0
var gear_label := "N"

@onready var speed_value: Label = $SpeedValue
@onready var gear_value: Label = $GearPlate/GearValue


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	_refresh_labels()
	queue_redraw()


func set_readout(next_speed_kph: float, next_gear_label: String) -> void:
	speed_kph = maxf(next_speed_kph, 0.0)
	gear_label = next_gear_label
	_refresh_labels()
	queue_redraw()


func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		queue_redraw()


func _draw() -> void:
	if size.x <= 1.0 or size.y <= 1.0:
		return

	var center := Vector2(size.x * 0.42, size.y * 0.89)
	var radius := minf(size.x * 0.40, size.y * 0.82)
	draw_arc(center, radius, ARC_START, ARC_END, 44, ARC_SHADOW, 8.0, true)
	_draw_segment(center, radius, 0.00, 0.28, ARC_YELLOW)
	_draw_segment(center, radius, 0.28, 0.54, ARC_ORANGE)
	_draw_segment(center, radius, 0.54, 0.78, ARC_RED)
	_draw_segment(center, radius, 0.78, 1.00, ARC_PURPLE)

	var normalized_speed := clampf(speed_kph / SPEED_LIMIT_KPH, 0.0, 1.0)
	var needle_angle := lerpf(ARC_START, ARC_END, normalized_speed)
	var needle_direction := Vector2(cos(needle_angle), sin(needle_angle))
	draw_line(center + needle_direction * (radius - 17.0), center + needle_direction * (radius + 3.0), Color.WHITE, 2.0, true)


func _draw_segment(center: Vector2, radius: float, start_ratio: float, end_ratio: float, color: Color) -> void:
	draw_arc(
		center,
		radius,
		lerpf(ARC_START, ARC_END, start_ratio),
		lerpf(ARC_START, ARC_END, end_ratio),
		14,
		color,
		5.0,
		true
	)


func _refresh_labels() -> void:
	if speed_value != null:
		speed_value.text = str(int(roundi(speed_kph)))
	if gear_value != null:
		gear_value.text = gear_label
