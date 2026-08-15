class_name SteeringBrakesSpec
extends Resource

@export_group("Steering")
@export var max_steering_angle: float = 0.436332
@export var steering_speed: float = 4.25
@export var countersteer_speed: float = 11.0
@export var steering_speed_decay: float = 0.20
@export var steering_slip_assist: float = 0.54
@export var countersteer_assist: float = 0.89
@export var steering_exponent: float = 1.5
@export var front_steering_ratio: float = 1.0
@export var rear_steering_ratio: float = 0.0

@export_group("Braking")
@export var front_brake_bias: float = 0.57
@export var braking_speed: float = 10.0
@export var brake_force_multiplier: float = 1.0
@export var braking_grip_multiplier: float = 1.08
@export var traction_control_max_slip: float = 8.0

@export_group("Stability")
@export var enable_stability: bool = true
@export var stability_yaw_engage_angle: float = 0.01
@export var stability_yaw_strength: float = 5.25
@export var stability_yaw_ground_multiplier: float = 2.0
@export var stability_upright_spring: float = 1.0
@export var stability_upright_damping: float = 1000.0

func apply_to(vehicle: Vehicle) -> void:
	if not vehicle:
		return
	vehicle.max_steering_angle = max_steering_angle
	vehicle.steering_speed = steering_speed
	vehicle.countersteer_speed = countersteer_speed
	vehicle.steering_speed_decay = steering_speed_decay
	vehicle.steering_slip_assist = steering_slip_assist
	vehicle.countersteer_assist = countersteer_assist
	vehicle.steering_exponent = steering_exponent
	vehicle.front_steering_ratio = front_steering_ratio
	vehicle.rear_steering_ratio = rear_steering_ratio

	vehicle.front_brake_bias = front_brake_bias
	vehicle.braking_speed = braking_speed
	vehicle.brake_force_multiplier = brake_force_multiplier
	vehicle.braking_grip_multiplier = braking_grip_multiplier
	vehicle.traction_control_max_slip = traction_control_max_slip

	vehicle.enable_stability = enable_stability
	vehicle.stability_yaw_engage_angle = stability_yaw_engage_angle
	vehicle.stability_yaw_strength = stability_yaw_strength
	vehicle.stability_yaw_ground_multiplier = stability_yaw_ground_multiplier
	vehicle.stability_upright_spring = stability_upright_spring
	vehicle.stability_upright_damping = stability_upright_damping
