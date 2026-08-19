class_name TrackMapData
extends Resource

## Presentation-only data. Gameplay progress remains owned by a future race
## session system, not by this map resource.

@export var track_id: StringName = &""
@export var centerline: PackedVector2Array = PackedVector2Array()
@export var start_position := Vector2.ZERO
@export var start_forward := Vector2(0.0, -1.0)


func is_valid_map() -> bool:
	return centerline.size() >= 3


func get_bounds() -> Rect2:
	if centerline.is_empty():
		return Rect2()

	var bounds := Rect2(centerline[0], Vector2.ZERO)
	for point in centerline:
		bounds = bounds.expand(point)
	return bounds
