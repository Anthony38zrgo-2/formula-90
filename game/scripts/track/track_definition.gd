class_name TrackDefinition
extends Resource

@export var id: StringName
@export var display_name := ""
@export var track_scene: PackedScene
@export var map_data: TrackMapData
@export var background_preset: BackgroundPreset
@export var background_preset_path: String = ""

func is_valid_definition() -> bool:
	return not id.is_empty() and track_scene != null

func get_effective_background_preset() -> BackgroundPreset:
	if background_preset != null:
		return background_preset
	if not background_preset_path.is_empty():
		return BackgroundPreset.load_from_json_file(background_preset_path)
	return null
