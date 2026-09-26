class_name VehicleDefinition
extends Resource

@export var id: StringName
@export var display_name := ""
@export var vehicle_scene: PackedScene
@export_file("*.json") var physics_config_path := ""
@export_file("*.json") var manifest_path := ""
@export_file("*.json") var tcam_config_path := ""

func is_valid_definition() -> bool:
	return not id.is_empty() and vehicle_scene != null \
		and not physics_config_path.is_empty() and not manifest_path.is_empty()


func load_fuel_plan() -> Dictionary:
	if physics_config_path.is_empty():
		return {}
	var file := FileAccess.open(physics_config_path, FileAccess.READ)
	if file == null:
		return {}
	var parsed: Variant = JSON.parse_string(file.get_as_text())
	if not (parsed is Dictionary):
		return {}
	var fuel_data: Variant = parsed.get("fuel", {})
	if not (fuel_data is Dictionary):
		return {}
	return {
		"capacity_kg": maxf(float(fuel_data.get("capacity_kg", 0.0)), 0.0),
		"initial_kg": maxf(float(fuel_data.get("initial_kg", 0.0)), 0.0),
		"estimated_lap_consumption_kg": maxf(
			float(fuel_data.get("estimated_lap_consumption_kg", 0.0)), 0.0),
		"reference_lap_time_s": maxf(
			float(fuel_data.get("reference_lap_time_s", 0.0)), 0.0),
	}
