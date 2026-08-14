class_name TrackDefinition
extends Resource

@export var id: StringName
@export var display_name := ""
@export var track_scene: PackedScene
@export var map_data: TrackMapData

func is_valid_definition() -> bool:
	return not id.is_empty() and track_scene != null
