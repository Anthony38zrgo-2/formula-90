class_name EngineSpec
extends Resource

@export var max_torque: float = 455.0
@export var max_rpm: float = 15000.0
@export var idle_rpm: float = 4500.0
@export var torque_curve: Curve
@export var motor_drag: float = 0.006
@export var motor_brake: float = 15.0
@export var motor_moment: float = 0.08
@export var clutch_out_rpm: float = 5000.0
@export var max_clutch_torque_ratio: float = 1.4

func apply_to(vehicle: Vehicle) -> void:
	if not vehicle:
		return
	vehicle.max_torque = max_torque
	vehicle.max_rpm = max_rpm
	vehicle.idle_rpm = idle_rpm
	if torque_curve:
		vehicle.torque_curve = torque_curve
	vehicle.motor_drag = motor_drag
	vehicle.motor_brake = motor_brake
	vehicle.motor_moment = motor_moment
	vehicle.clutch_out_rpm = clutch_out_rpm
	vehicle.max_clutch_torque_ratio = max_clutch_torque_ratio
