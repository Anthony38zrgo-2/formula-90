class_name AeroSpec
extends Resource

@export var coefficient_of_drag: float = 0.15
@export var frontal_area: float = 0.45

func apply_to(vehicle: Vehicle) -> void:
	if not vehicle:
		return
	vehicle.coefficient_of_drag = coefficient_of_drag
	vehicle.frontal_area = frontal_area
