class_name TrackMinimapController
extends Control

const TRACK_OUTLINE := Color(0.97, 0.98, 0.94, 0.98)
const TRACK_SHADOW := Color(0.01, 0.02, 0.03, 0.92)
const START_COLOR := Color(0.28, 1.0, 0.34, 1.0)
const PLAYER_COLOR := Color(0.20, 0.50, 1.0, 1.0)
const PLAYER_OUTLINE := Color(0.01, 0.02, 0.08, 0.98)

@export var target_path: NodePath
@export var map_data: TrackMapData
@export_range(0.02, 0.24, 0.01) var padding_ratio := 0.10

var _target: Node3D


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	queue_redraw()


func set_target(target: Node3D) -> void:
	_target = target
	queue_redraw()


func _process(_delta: float) -> void:
	if not is_instance_valid(_target) and not target_path.is_empty():
		_target = get_node_or_null(target_path) as Node3D
	queue_redraw()


func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		queue_redraw()


func get_player_map_position() -> Vector2:
	if not is_instance_valid(_target):
		return Vector2.ZERO
	return _world_to_map(Vector2(_target.global_position.x, _target.global_position.z))


func _draw() -> void:
	if map_data == null or not map_data.is_valid_map():
		return

	var outline := PackedVector2Array()
	for world_point in map_data.centerline:
		outline.append(_world_to_map(world_point))
	outline.append(outline[0])

	draw_polyline(outline, TRACK_SHADOW, 4.0, true)
	draw_polyline(outline, TRACK_OUTLINE, 1.8, true)
	_draw_start_marker()
	_draw_player_marker()


func _draw_start_marker() -> void:
	var start := _world_to_map(map_data.start_position)
	var heading := map_data.start_forward.normalized()
	if heading.length_squared() < 0.0001:
		heading = Vector2.UP
	var side := Vector2(-heading.y, heading.x)
	draw_line(start - side * 6.0, start + side * 6.0, TRACK_SHADOW, 5.0, true)
	draw_line(start - side * 6.0, start + side * 6.0, START_COLOR, 2.6, true)
	draw_circle(start, 3.7, START_COLOR, true, -1.0, true)


func _draw_player_marker() -> void:
	if not is_instance_valid(_target):
		return

	var map_rect := _get_map_rect()
	var center := get_player_map_position()
	center.x = clampf(center.x, map_rect.position.x, map_rect.end.x)
	center.y = clampf(center.y, map_rect.position.y, map_rect.end.y)

	var world_forward := -_target.global_basis.z
	var heading := Vector2(world_forward.x, world_forward.z)
	if heading.length_squared() < 0.0001:
		heading = Vector2.UP
	else:
		heading = heading.normalized()
	var side := Vector2(-heading.y, heading.x)

	var marker := PackedVector2Array([
		center + heading * 6.0,
		center - heading * 4.0 + side * 3.5,
		center - heading * 2.0,
		center - heading * 4.0 - side * 3.5,
	])
	var marker_outline := PackedVector2Array([marker[0], marker[1], marker[2], marker[3], marker[0]])
	draw_colored_polygon(marker, PLAYER_COLOR)
	draw_polyline(marker_outline, PLAYER_OUTLINE, 1.8, true)


func _world_to_map(world_point: Vector2) -> Vector2:
	if map_data == null or not map_data.is_valid_map():
		return Vector2.ZERO

	var bounds := map_data.get_bounds()
	var available := _get_map_rect()
	var source_size := Vector2(maxf(bounds.size.x, 0.001), maxf(bounds.size.y, 0.001))
	var scale := minf(available.size.x / source_size.x, available.size.y / source_size.y)
	var rendered_size := source_size * scale
	var origin := available.get_center() - rendered_size * 0.5
	return origin + (world_point - bounds.position) * scale


func _get_map_rect() -> Rect2:
	var inset := minf(size.x, size.y) * padding_ratio
	return Rect2(
		Vector2(inset, inset),
		Vector2(maxf(size.x - inset * 2.0, 1.0), maxf(size.y - inset * 2.0, 1.0))
	)
