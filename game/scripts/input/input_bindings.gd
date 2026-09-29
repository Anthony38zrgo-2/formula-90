extends Node

const INPUT_PROFILE_PATH := "res://data/input/input_profile.json"

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
const PIT_FIELD_UP := "Pit Field Up"
const PIT_FIELD_DOWN := "Pit Field Down"
const PIT_VALUE_LEFT := "Pit Value Left"
const PIT_VALUE_RIGHT := "Pit Value Right"
const PIT_CONFIRM := "Pit Confirm"
const PIT_TOGGLE_MENU := "Pit Toggle Menu"

var input_profile: InputProfile


func _init() -> void:
	input_profile = InputProfile.load_from_path(INPUT_PROFILE_PATH)
	for rejection_message in input_profile.errors:
		push_error("Input profile rejected an entry: " + rejection_message)
	for action_name in input_profile.action_definitions:
		var definition: Dictionary = input_profile.action_definitions[action_name]
		_register(action_name, float(definition["deadzone"]), definition["events"])


func _register(action: String, deadzone: float, events: Array) -> void:
	if InputMap.has_action(action):
		InputMap.erase_action(action)
	InputMap.add_action(action, deadzone)
	for input_event in events:
		InputMap.action_add_event(action, input_event)
