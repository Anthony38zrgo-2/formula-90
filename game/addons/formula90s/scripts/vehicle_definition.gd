class_name VehicleDefinition
extends Resource

@export var id: StringName
@export var display_name := ""
@export var vehicle_scene: PackedScene

func is_valid_definition() -> bool:
	return not id.is_empty() and vehicle_scene != null
