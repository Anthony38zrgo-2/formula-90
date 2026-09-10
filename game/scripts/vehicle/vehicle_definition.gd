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
