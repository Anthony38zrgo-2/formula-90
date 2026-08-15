class_name SuspensionSpec
extends Resource

@export_group("Front Axle")
@export var front_spring_length: float = 0.25
@export var front_resting_ratio: float = 0.4
@export var front_damping_ratio: float = 0.8
@export var front_bump_damp_multiplier: float = 1.3
@export var front_rebound_damp_multiplier: float = 1.1
@export var front_arb_ratio: float = 0.0
@export var front_camber: float = -0.0174533
@export var front_toe: float = 0.0017453
@export var front_bump_stop_multiplier: float = 2.2

@export_group("Rear Axle")
@export var rear_spring_length: float = 0.18
@export var rear_resting_ratio: float = 0.35
@export var rear_damping_ratio: float = 0.85
@export var rear_bump_damp_multiplier: float = 1.3
@export var rear_rebound_damp_multiplier: float = 1.1
@export var rear_arb_ratio: float = 0.05
@export var rear_camber: float = -0.0174533
@export var rear_toe: float = 0.0017453
@export var rear_bump_stop_multiplier: float = 3.5

func apply_to(vehicle: Vehicle) -> void:
	if not vehicle:
		return
	vehicle.front_spring_length = front_spring_length
	vehicle.front_resting_ratio = front_resting_ratio
	vehicle.front_damping_ratio = front_damping_ratio
	vehicle.front_bump_damp_multiplier = front_bump_damp_multiplier
	vehicle.front_rebound_damp_multiplier = front_rebound_damp_multiplier
	vehicle.front_arb_ratio = front_arb_ratio
	vehicle.front_camber = front_camber
	vehicle.front_toe = front_toe
	vehicle.front_bump_stop_multiplier = front_bump_stop_multiplier

	vehicle.rear_spring_length = rear_spring_length
	vehicle.rear_resting_ratio = rear_resting_ratio
	vehicle.rear_damping_ratio = rear_damping_ratio
	vehicle.rear_bump_damp_multiplier = rear_bump_damp_multiplier
	vehicle.rear_rebound_damp_multiplier = rear_rebound_damp_multiplier
	vehicle.rear_arb_ratio = rear_arb_ratio
	vehicle.rear_camber = rear_camber
	vehicle.rear_toe = rear_toe
	vehicle.rear_bump_stop_multiplier = rear_bump_stop_multiplier
