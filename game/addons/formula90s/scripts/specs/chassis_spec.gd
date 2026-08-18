class_name ChassisSpec
extends Resource

@export var vehicle_mass: float = 575.0
@export var front_weight_distribution: float = 0.45
@export var inertia_multiplier: float = 1.1

func apply_to(vehicle: Vehicle) -> void:
	if not vehicle:
		return
	vehicle.vehicle_mass = vehicle_mass
	vehicle.front_weight_distribution = front_weight_distribution
	vehicle.inertia_multiplier = inertia_multiplier
