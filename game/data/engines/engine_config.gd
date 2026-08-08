extends Resource
class_name EngineConfig

@export var max_rpm : float = 7000.0
@export var max_torque : float = 300.0
@export var idle_rpm : float = 1000.0
@export var motor_drag : float = 0.005
@export var motor_brake : float = 10.0
@export var motor_moment : float = 0.5
@export var clutch_out_rpm : float = 3000.0
@export var max_clutch_torque_ratio : float = 1.6
@export var torque_curve : Curve

func apply_to(vehicle) -> void:
	vehicle.max_rpm = max_rpm
	vehicle.max_torque = max_torque
	vehicle.idle_rpm = idle_rpm
	vehicle.motor_drag = motor_drag
	vehicle.motor_brake = motor_brake
	vehicle.motor_moment = motor_moment
	vehicle.clutch_out_rpm = clutch_out_rpm
	vehicle.max_clutch_torque_ratio = max_clutch_torque_ratio
	if torque_curve:
		vehicle.torque_curve = torque_curve
