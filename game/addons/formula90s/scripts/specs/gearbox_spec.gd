class_name GearboxSpec
extends Resource

@export var gear_ratios: Array[float] = [2.85, 2.29, 1.89, 1.6, 1.38, 1.2]
@export var final_drive: float = 6.3
@export var reverse_ratio: float = 3.0
@export var shift_time: float = 0.12
@export var automatic_time_between_shifts: float = 700.0
@export var gear_inertia: float = 0.03
@export var front_locking_differential_engage_torque: float = -1.0
@export var rear_locking_differential_engage_torque: float = 170.0

func apply_to(vehicle: Vehicle) -> void:
	if not vehicle:
		return
	vehicle.gear_ratios = gear_ratios.duplicate()
	vehicle.final_drive = final_drive
	vehicle.reverse_ratio = reverse_ratio
	vehicle.shift_time = shift_time
	vehicle.automatic_time_between_shifts = automatic_time_between_shifts
	vehicle.gear_inertia = gear_inertia
	vehicle.front_locking_differential_engage_torque = front_locking_differential_engage_torque
	vehicle.rear_locking_differential_engage_torque = rear_locking_differential_engage_torque
