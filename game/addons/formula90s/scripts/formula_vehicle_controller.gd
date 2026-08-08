extends VehicleController
class_name FormulaVehicleController

## Formula90s input layer on top of the upstream GEVP controller.
## Keeps GEVP vendor code untouched while enforcing Formula90s gearbox semantics.

## Reverse can only be selected from neutral when the vehicle is effectively stopped.
## A small tolerance avoids blocking reverse because of residual RigidBody3D motion.
@export_range(0.0, 2.0, 0.01, "suffix:m/s") var reverse_engage_max_speed := 0.15

func _physics_process(_delta: float) -> void:
	if not is_instance_valid(vehicle_node):
		return

	if string_brake_input != "":
		vehicle_node.brake_input = Input.get_action_strength(string_brake_input)

	if string_steer_left != "" and string_steer_right != "":
		vehicle_node.steering_input = (
			Input.get_action_strength(string_steer_left)
			- Input.get_action_strength(string_steer_right)
		)

	if string_throttle_input != "":
		# Preserve upstream GEVP throttle shaping.
		vehicle_node.throttle_input = pow(Input.get_action_strength(string_throttle_input), 2.0)

	if string_handbrake_input != "":
		vehicle_node.handbrake_input = Input.get_action_strength(string_handbrake_input)

	if string_clutch_input != "":
		var handbrake_strength := 0.0
		if string_handbrake_input != "":
			handbrake_strength = Input.get_action_strength(string_handbrake_input)
		vehicle_node.clutch_input = clampf(
			Input.get_action_strength(string_clutch_input) + handbrake_strength,
			0.0,
			1.0
		)

	_process_transmission_inputs()

func _process_transmission_inputs() -> void:
	if string_toggle_transmission != "" and Input.is_action_just_pressed(string_toggle_transmission):
		vehicle_node.automatic_transmission = not vehicle_node.automatic_transmission

	if string_shift_up != "" and Input.is_action_just_pressed(string_shift_up):
		vehicle_node.manual_shift(1)

	if string_shift_down != "" and Input.is_action_just_pressed(string_shift_down):
		if _can_manual_shift_down():
			vehicle_node.manual_shift(-1)

func _can_manual_shift_down() -> bool:
	# manual_shift() is already ignored by GEVP while automatic mode is enabled.
	if vehicle_node.automatic_transmission:
		return true

	# All normal downshifts remain unchanged. Only Neutral -> Reverse is gated.
	if vehicle_node.current_gear != 0:
		return true

	return vehicle_node.speed <= reverse_engage_max_speed
