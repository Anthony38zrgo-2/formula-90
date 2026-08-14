class_name RetroHudTachometer
extends Control

var rpm := 0.0
var rpm_max := 15000.0
var rpm_redline := 12200.0
var segment_count := 14
var start_degrees := 201.0
var end_degrees := 339.0
var normal_color := Color("65ffe0")
var high_rpm_color := Color("f4d35e")
var redline_color := Color("ff5a5f")


func configure(config: RefCounted) -> void:
	rpm_max = config.rpm_max
	rpm_redline = config.rpm_redline
	segment_count = config.tach_segments
	start_degrees = config.tach_start_degrees
	end_degrees = config.tach_end_degrees
	normal_color = config.normal_color
	high_rpm_color = config.high_rpm_color
	redline_color = config.redline_color
	queue_redraw()


func set_rpm(next_rpm: float) -> void:
	rpm = maxf(next_rpm, 0.0)
	queue_redraw()


func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		queue_redraw()


func _draw() -> void:
	if size.x <= 1.0 or size.y <= 1.0:
		return
	var center := Vector2(size.x * 0.5, size.y * 0.89)
	var outer_radius := size.x * 0.465
	var inner_radius := outer_radius * 0.81
	var active_segments := int(ceil(clampf(rpm / rpm_max, 0.0, 1.0) * float(segment_count)))
	for index in segment_count:
		if index >= active_segments:
			continue
		var ratio := (float(index) + 0.5) / float(segment_count)
		var color := normal_color
		if ratio >= rpm_redline / rpm_max:
			color = redline_color
		elif ratio >= 0.72:
			color = high_rpm_color
		_draw_wedge(center, inner_radius, outer_radius, ratio, color)


func _draw_wedge(center: Vector2, inner_radius: float, outer_radius: float, ratio: float, color: Color) -> void:
	var segment_span := (end_degrees - start_degrees) / float(segment_count)
	var angle_center := start_degrees + ratio * (end_degrees - start_degrees)
	var angle_start := deg_to_rad(angle_center - segment_span * 0.40)
	var angle_end := deg_to_rad(angle_center + segment_span * 0.40)
	var points := PackedVector2Array([
		center + Vector2(cos(angle_start), sin(angle_start)) * inner_radius,
		center + Vector2(cos(angle_start), sin(angle_start)) * outer_radius,
		center + Vector2(cos(angle_end), sin(angle_end)) * outer_radius,
		center + Vector2(cos(angle_end), sin(angle_end)) * inner_radius
	])
	draw_colored_polygon(points, color)
