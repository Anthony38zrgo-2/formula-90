class_name RaceSessionConfig
extends Resource

@export var selected_vehicle: VehicleDefinition
@export var selected_track: TrackDefinition

func is_valid_config() -> bool:
	return selected_vehicle != null and selected_vehicle.is_valid_definition() \
		and selected_track != null and selected_track.is_valid_definition()
