class_name TrackDefinition
extends Resource

@export var id: StringName
@export var display_name := ""
@export var track_scene: PackedScene
@export var map_data: TrackMapData
@export var background_preset: BackgroundPreset
@export var background_preset_path: String = ""
@export_file("*.json") var metadata_path: String = ""

func is_valid_definition() -> bool:
	return not id.is_empty() and track_scene != null

func get_effective_background_preset() -> BackgroundPreset:
	if background_preset != null:
		return background_preset
	if not background_preset_path.is_empty():
		return BackgroundPreset.load_from_json_file(background_preset_path)
	return null

func load_metadata() -> Dictionary:
	if metadata_path.is_empty():
		return {}
	var file := FileAccess.open(metadata_path, FileAccess.READ)
	if file == null:
		push_warning("TrackDefinition: no se pudo abrir %s" % metadata_path)
		return {}
	var parsed: Variant = JSON.parse_string(file.get_as_text())
	if not (parsed is Dictionary):
		push_warning("TrackDefinition: metadata invalida en %s" % metadata_path)
		return {}
	return parsed

func load_start_finish_data() -> Dictionary:
	var start_finish: Variant = load_metadata().get("start_finish", {})
	if not (start_finish is Dictionary):
		return {}
	return start_finish

func load_lap_length_m() -> float:
	return float(load_metadata().get("lap_length_m", 0.0))


func load_pit_lane_data() -> Dictionary:
	var pit_lane: Variant = load_metadata().get("pit_lane", {})
	if not (pit_lane is Dictionary):
		return {}
	return pit_lane


func has_pit_lane() -> bool:
	var pit_lane := load_pit_lane_data()
	if pit_lane.is_empty():
		return false
	if not (pit_lane.get("boxes") is Array) or pit_lane.get("boxes", []).is_empty():
		return false
	return pit_lane.get("strip_center_m") is Array and pit_lane.get("strip_half_extents_m") is Array
