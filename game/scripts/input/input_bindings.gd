extends Node

## InputBindings: single, authoritative source of truth for all input actions.
##
## Registered in project.godot [autoload] BEFORE TelemetryManager and any scene
## node, so every controller that reads InputMap by name sees these definitions
## (and not any stale/duplicate [input] section). Rebinding happens here only.
##
## Canonical name constants are exposed so scene scripts (e.g.
## F194RustInputController) default their @export action strings to these instead
## of inline literals, keeping the keymap fully decoupled from gameplay code.

# --- Canonical action name constants ---
const THROTTLE := "Throttle"
const BRAKES := "Brakes"
const STEER_LEFT := "Steer Left"
const STEER_RIGHT := "Steer Right"
const HANDBRAKE := "Handbrake"
const CLUTCH := "Clutch"
const SHIFT_UP := "Shift Up"
const SHIFT_DOWN := "Shift Down"
const TOGGLE_TRANSMISSION := "Toggle Transmission"
const TOGGLE_TRACTION_CONTROL := "Toggle Traction Control"
const AID_1 := "aid_1"
const AID_2 := "aid_2"
const AID_3 := "aid_3"
const AID_4 := "aid_4"
const AID_5 := "aid_5"
const UI_BACK_TO_MENU := "ui_back_to_menu"
const SHOW_DEBUG := "ShowDebug"
const DEBUG_NEXT := "DebugNext"
const DEBUG_PREVIOUS := "DebugPrevious"
const RESET_VEHICLE := "Reset Vehicle"
const TOGGLE_CAMERA := "Toggle Camera"


func _init() -> void:
	# Physical keycodes are ported verbatim from the former project.godot [input]
	# section so behavior is identical. Godot 4 encodes physical keys as
	# 0x400000 | keycode; the raw integers are used directly here.
	_register(THROTTLE, 0.5, [key_ev(4194320), joypad_axis_ev(5, 1.0)])
	_register(BRAKES, 0.5, [key_ev(4194322), joypad_axis_ev(4, 1.0)])
	_register(STEER_LEFT, 0.12, [key_ev(4194319), joypad_axis_ev(0, -1.0)])
	_register(STEER_RIGHT, 0.12, [key_ev(4194321), joypad_axis_ev(0, 1.0)])
	_register(HANDBRAKE, 0.5, [key_ev(32), joypad_button_ev(0)])
	_register(CLUTCH, 0.5, [key_ev(88), joypad_button_ev(9)])
	_register(SHIFT_UP, 0.5, [key_ev(65), joypad_button_ev(2)])
	_register(SHIFT_DOWN, 0.5, [key_ev(90), joypad_button_ev(3)])
	_register(AID_1, 0.2, [key_ev(49)])
	_register(AID_2, 0.2, [key_ev(50)])
	_register(AID_3, 0.2, [key_ev(51)])
	_register(AID_4, 0.2, [key_ev(52)])
	_register(AID_5, 0.2, [key_ev(53)])
	_register(UI_BACK_TO_MENU, 0.2, [key_ev(4194305), joypad_button_ev(6)])
	_register(SHOW_DEBUG, 0.5, [key_ev(4194323), joypad_button_ev(11)])
	_register(DEBUG_NEXT, 0.5, [key_ev(46), joypad_button_ev(14)])
	_register(DEBUG_PREVIOUS, 0.5, [key_ev(44), joypad_button_ev(13)])
	_register(TOGGLE_TRANSMISSION, 0.5, [key_ev(4194312), joypad_button_ev(1)])
	# Previously missing in project.godot [input]; now defined authoritatively.
	_register(TOGGLE_TRACTION_CONTROL, 0.5, [key_ev(84), joypad_button_ev(12)])
	_register(RESET_VEHICLE, 0.5, [key_ev(82)])
	# Fixed T-cam toggle. C (67) previously drove Clutch; Clutch moved to X (88).
	_register(TOGGLE_CAMERA, 0.5, [key_ev(67)])


func _register(action: String, deadzone: float, events: Array) -> void:
	if InputMap.has_action(action):
		InputMap.erase_action(action)
	InputMap.add_action(action, deadzone)
	for ev in events:
		InputMap.action_add_event(action, ev)


static func key_ev(physical_keycode: int) -> InputEventKey:
	var ev := InputEventKey.new()
	ev.physical_keycode = physical_keycode
	return ev


static func joypad_axis_ev(axis: int, axis_value: float) -> InputEventJoypadMotion:
	var ev := InputEventJoypadMotion.new()
	ev.axis = axis
	ev.axis_value = axis_value
	return ev


static func joypad_button_ev(button_index: int) -> InputEventJoypadButton:
	var ev := InputEventJoypadButton.new()
	ev.button_index = button_index
	return ev
