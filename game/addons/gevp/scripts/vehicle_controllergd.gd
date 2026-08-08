extends Node3D
class_name VehicleController

@export var vehicle_node : Vehicle

@export_group("Input Maps", "string_")
@export var string_brake_input: String = "Brakes"
@export var string_steer_left: String = "Steer Left"
@export var string_steer_right: String = "Steer Right"
@export var string_throttle_input: String = "Throttle"
@export var string_handbrake_input: String = "Handbrake"
@export var string_clutch_input: String = "Clutch"
@export var string_toggle_transmission: String = "Toggle Transmission"
@export var string_shift_up: String = "Shift Up"
@export var string_shift_down: String = "Shift Down"

const STOP_SPEED = 0.1
const INPUT_THRESHOLD = 0.5

var _brake_was_down = false
var _throttle_was_down = false

func _physics_process(_delta):
	if not vehicle_node:
		return

	if string_steer_left != "" and string_steer_right != "":
		vehicle_node.steering_input = Input.get_action_strength(string_steer_left) - Input.get_action_strength(string_steer_right)

	if string_throttle_input != "":
		vehicle_node.throttle_input = pow(Input.get_action_strength(string_throttle_input), 2.0)

	if string_brake_input != "":
		vehicle_node.brake_input = Input.get_action_strength(string_brake_input)

	if string_handbrake_input != "":
		vehicle_node.handbrake_input = Input.get_action_strength(string_handbrake_input)

	if string_clutch_input != "":
		vehicle_node.clutch_input = clampf(Input.get_action_strength(string_clutch_input) + Input.get_action_strength(string_handbrake_input), 0.0, 1.0)

	if string_toggle_transmission != "":
		if Input.is_action_just_pressed(string_toggle_transmission):
			vehicle_node.automatic_transmission = not vehicle_node.automatic_transmission

	if string_shift_up != "":
		if Input.is_action_just_pressed(string_shift_up):
			vehicle_node.manual_shift(1)

	if string_shift_down != "":
		if Input.is_action_just_pressed(string_shift_down):
			vehicle_node.manual_shift(-1)

	if vehicle_node.automatic_transmission:
		var stopped = abs(vehicle_node.speed) < STOP_SPEED
		var brake_down = Input.get_action_strength(string_brake_input) > INPUT_THRESHOLD
		var throttle_down = Input.get_action_strength(string_throttle_input) > INPUT_THRESHOLD

		if stopped and brake_down and not _brake_was_down and vehicle_node.current_gear > 0:
			vehicle_node.current_gear = -1
		elif stopped and throttle_down and not _throttle_was_down and vehicle_node.current_gear < 0:
			vehicle_node.current_gear = 1

		_brake_was_down = brake_down
		_throttle_was_down = throttle_down

	if vehicle_node.current_gear == -1:
		var actual_throttle = vehicle_node.throttle_input
		vehicle_node.throttle_input = vehicle_node.brake_input
		vehicle_node.brake_input = actual_throttle
