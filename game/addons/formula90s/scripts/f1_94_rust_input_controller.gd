extends Node3D
class_name F194RustInputController

@export var vehicle_node: Node

@export var action_throttle: String = "Throttle"
@export var action_brake: String = "Brakes"
@export var action_steer_left: String = "Steer Left"
@export var action_steer_right: String = "Steer Right"
@export var action_handbrake: String = "Handbrake"
@export var action_clutch: String = "Clutch"
@export var action_shift_up: String = "Shift Up"
@export var action_shift_down: String = "Shift Down"
@export var action_toggle_transmission: String = "Toggle Transmission"

var _has_required_interface: bool = false

func _ready() -> void:
	if vehicle_node == null:
		push_error("[F194RustInputController] vehicle_node is null on '%s'" % get_path())
		return

	# Validate required interface
	var required_methods = [
		"set_throttle_amount",
		"set_steering_input",
		"set_brake_amount",
		"set_handbrake_amount",
		"set_clutch_amount",
		"set_gear_request",
		"get_current_gear",
		"get_speed_kmh"
	]

	for method in required_methods:
		if not vehicle_node.has_method(method):
			push_error("[F194RustInputController] vehicle_node '%s' is missing required method '%s'" % [vehicle_node.name, method])
			_has_required_interface = false
			return

	_has_required_interface = true

func _physics_process(_delta: float) -> void:
	if not _has_required_interface or vehicle_node == null:
		return

	if "enable_player_input" in vehicle_node and not vehicle_node.enable_player_input:
		return

	var brake_val := 0.0
	if action_brake != "" and InputMap.has_action(action_brake):
		brake_val = Input.get_action_strength(action_brake)

	var steer_left := 0.0
	if action_steer_left != "" and InputMap.has_action(action_steer_left):
		steer_left = Input.get_action_strength(action_steer_left)

	var steer_right := 0.0
	if action_steer_right != "" and InputMap.has_action(action_steer_right):
		steer_right = Input.get_action_strength(action_steer_right)
	var steering_val := steer_left - steer_right

	var throttle_val := 0.0
	if action_throttle != "" and InputMap.has_action(action_throttle):
		throttle_val = pow(Input.get_action_strength(action_throttle), 2.0)

	var handbrake_val := 0.0
	if action_handbrake != "" and InputMap.has_action(action_handbrake):
		handbrake_val = Input.get_action_strength(action_handbrake)

	var clutch_raw := 0.0
	if action_clutch != "" and InputMap.has_action(action_clutch):
		clutch_raw = Input.get_action_strength(action_clutch)
	var clutch_val := clampf(clutch_raw + handbrake_val, 0.0, 1.0)

	if action_toggle_transmission != "" and InputMap.has_action(action_toggle_transmission):
		if Input.is_action_just_pressed(action_toggle_transmission):
			if vehicle_node.has_method("get_automatic_transmission") and vehicle_node.has_method("set_automatic_transmission"):
				var auto: bool = vehicle_node.get_automatic_transmission()
				vehicle_node.set_automatic_transmission(not auto)

	# Manual shift handling
	var current_gear: int = vehicle_node.get_current_gear()
	var next_gear: int = -2

	var shift_up := false
	if action_shift_up != "" and InputMap.has_action(action_shift_up):
		shift_up = Input.is_action_just_pressed(action_shift_up)

	var shift_down := false
	if action_shift_down != "" and InputMap.has_action(action_shift_down):
		shift_down = Input.is_action_just_pressed(action_shift_down)

	if shift_up:
		if current_gear == -1:
			next_gear = 0
		elif current_gear < 6:
			next_gear = current_gear + 1
	elif shift_down:
		if current_gear > 0:
			next_gear = current_gear - 1
		elif current_gear == 0:
			if absf(vehicle_node.get_speed_kmh()) < 3.0:
				next_gear = -1

	if current_gear == -1:
		var raw_throttle = throttle_val
		var raw_brake = brake_val
		throttle_val = raw_brake
		brake_val = raw_throttle

	vehicle_node.set_throttle_amount(throttle_val)
	vehicle_node.set_steering_input(steering_val)
	vehicle_node.set_brake_amount(brake_val)
	vehicle_node.set_handbrake_amount(handbrake_val)
	vehicle_node.set_clutch_amount(clutch_val)
	if next_gear != -2:
		vehicle_node.set_gear_request(next_gear)
