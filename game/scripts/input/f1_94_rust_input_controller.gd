extends Node3D
class_name F194RustInputController

@export var vehicle_node: Node

@export var action_throttle: String = InputBindings.THROTTLE
@export var action_brake: String = InputBindings.BRAKES
@export var action_steer_left: String = InputBindings.STEER_LEFT
@export var action_steer_right: String = InputBindings.STEER_RIGHT
@export var action_handbrake: String = InputBindings.HANDBRAKE
@export var action_clutch: String = InputBindings.CLUTCH
@export var action_shift_up: String = InputBindings.SHIFT_UP
@export var action_shift_down: String = InputBindings.SHIFT_DOWN
@export var action_toggle_transmission: String = InputBindings.TOGGLE_TRANSMISSION
@export var action_toggle_traction_control: String = InputBindings.TOGGLE_TRACTION_CONTROL
@export var action_reset_vehicle: String = InputBindings.RESET_VEHICLE
@export var throttle_exponent: float = 1.0

var _has_required_interface: bool = false
var _spawn_pos: Vector3 = Vector3()
var _spawn_yaw: float = 0.0
var _max_gear: int = 6

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

	_resolve_max_gear()

	# Capture spawn pose so Reset Vehicle can restore it.
	if vehicle_node.has_method("global_transform"):
		var t: Transform3D = vehicle_node.global_transform
		_spawn_pos = t.origin
		_spawn_yaw = t.basis.get_euler().y

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
		var raw_throttle := Input.get_action_strength(action_throttle)
		throttle_val = pow(raw_throttle, throttle_exponent) if throttle_exponent != 1.0 else raw_throttle

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

	if action_toggle_traction_control != "" and InputMap.has_action(action_toggle_traction_control):
		if Input.is_action_just_pressed(action_toggle_traction_control):
			if vehicle_node.has_method("get_aids_enabled_mask") and vehicle_node.has_method("set_aids_enabled_mask"):
				var mask: int = vehicle_node.get_aids_enabled_mask()
				mask ^= 1 << 1
				vehicle_node.set_aids_enabled_mask(mask)

	if action_reset_vehicle != "" and InputMap.has_action(action_reset_vehicle):
		if Input.is_action_just_pressed(action_reset_vehicle):
			if vehicle_node.has_method("reset_vehicle"):
				vehicle_node.reset_vehicle(_spawn_pos, _spawn_yaw)

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
		elif current_gear < _max_gear:
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


func _resolve_max_gear() -> void:
	# Derive the number of forward gears from the active vehicle's physics config
	# (data-driven: a 7-gear car just needs a 7-element gear_ratios array, no code
	# change). Falls back to 6 if the F90Core/config cannot be read.
	_max_gear = 6
	var core := _find_f90_core()
	if core == null:
		return
	if not core.has_method("get_config_json_path"):
		return
	var cfg_path: String = core.get_config_json_path()
	if cfg_path.is_empty():
		return
	var global_p := cfg_path
	if cfg_path.begins_with("res://"):
		global_p = ProjectSettings.globalize_path(cfg_path)
	if not FileAccess.file_exists(global_p):
		return
	var f := FileAccess.open(global_p, FileAccess.READ)
	if f == null:
		return
	var text := f.get_as_text()
	f.close()
	var parsed: Variant = JSON.parse_string(text)
	if typeof(parsed) != TYPE_DICTIONARY:
		return
	var pt: Dictionary = parsed.get("powertrain", {})
	if typeof(pt) != TYPE_DICTIONARY:
		return
	var ratios: Variant = pt.get("gear_ratios", [])
	if typeof(ratios) == TYPE_ARRAY and ratios.size() > 0:
		_max_gear = ratios.size()


func _find_f90_core() -> Node:
	var root := get_tree().current_scene
	if root == null:
		root = get_owner()
	return _find_by_class(root, "F90Core")


func _find_by_class(node: Node, cls: String) -> Node:
	if node == null:
		return null
	if node.get_class() == cls:
		return node
	for i in node.get_child_count():
		var r := _find_by_class(node.get_child(i), cls)
		if r != null:
			return r
	return null
