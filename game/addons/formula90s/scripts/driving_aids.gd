extends Node
class_name DrivingAidsController

@export var vehicle_node: Node
var aids := [true, false, false, false, false]
var _baseline: Dictionary = {}
var _captured := false

const MULT_STABILITY := 2.0
const MULT_STEERING := 1.5
const MULT_BRAKING := 1.5
const MULT_GRIP := 1.3
const FLOOR_STABILITY_STRENGTH := 4.0
const FLOOR_GRIP := 1.5

func _ready():
	if vehicle_node:
		_capture_baseline()
		_apply_aids()

func _capture_baseline():
	_baseline["enable_stability"] = vehicle_node.get("enable_stability")
	_baseline["stability_yaw_strength"] = vehicle_node.get("stability_yaw_strength")
	_baseline["steering_exponent"] = vehicle_node.get("steering_exponent")
	_baseline["brake_force_multiplier"] = vehicle_node.get("brake_force_multiplier")
	_baseline["friction"] = vehicle_node.get("coefficient_of_friction").duplicate()
	_baseline["lateral_grip_assist"] = vehicle_node.get("lateral_grip_assist").duplicate() if vehicle_node.get("lateral_grip_assist") else {}
	_baseline["automatic_transmission"] = vehicle_node.get("automatic_transmission")
	_captured = true

func _physics_process(_delta):
	if not vehicle_node: return
	if not _captured: _capture_baseline()
	for i in range(aids.size()):
		if Input.is_action_just_pressed("aid_%d" % (i + 1)):
			toggle(i)

func toggle(index: int):
	aids[index] = not aids[index]
	if not aids[index]:
		_restore(index)
	else:
		_apply_aid(index)

func _apply_aids():
	for i in range(aids.size()):
		if aids[i]: _apply_aid(i)

func _apply_aid(index: int):
	match index:
		0:
			vehicle_node.set("automatic_transmission", true)
		1:
			vehicle_node.set("enable_stability", true)
			vehicle_node.set("stability_yaw_strength", max(_baseline["stability_yaw_strength"] * MULT_STABILITY, FLOOR_STABILITY_STRENGTH))
		2:
			vehicle_node.set("steering_exponent", _baseline["steering_exponent"] * MULT_STEERING)
		3:
			vehicle_node.set("brake_force_multiplier", _baseline["brake_force_multiplier"] * MULT_BRAKING)
		4:
			var friction = _baseline["friction"].duplicate()
			for key in friction:
				friction[key] = max(friction[key] * MULT_GRIP, FLOOR_GRIP)
			vehicle_node.set("coefficient_of_friction", friction)
			if _baseline.has("lateral_grip_assist") and _baseline["lateral_grip_assist"].size() > 0:
				var grip = _baseline["lateral_grip_assist"].duplicate()
				for key in grip:
					grip[key] = max(grip[key] + 0.15, 0.15)
				vehicle_node.set("lateral_grip_assist", grip)

func _restore(index: int):
	match index:
		0:
			vehicle_node.set("automatic_transmission", _baseline["automatic_transmission"])
		1:
			vehicle_node.set("enable_stability", _baseline["enable_stability"])
			vehicle_node.set("stability_yaw_strength", _baseline["stability_yaw_strength"])
		2:
			vehicle_node.set("steering_exponent", _baseline["steering_exponent"])
		3:
			vehicle_node.set("brake_force_multiplier", _baseline["brake_force_multiplier"])
		4:
			vehicle_node.set("coefficient_of_friction", _baseline["friction"])
			if _baseline.has("lateral_grip_assist") and _baseline["lateral_grip_assist"].size() > 0:
				vehicle_node.set("lateral_grip_assist", _baseline["lateral_grip_assist"])

func is_aid_enabled(index: int) -> bool:
	return aids[index] if index >= 0 and index < aids.size() else false

func get_aid_label(index: int) -> String:
	match index:
		0: return "AUTO"
		1: return "ESTAB"
		2: return "DIRECC"
		3: return "FRENOS"
		4: return "GRIP"
	return ""

func get_aid_status(index: int) -> String:
	return "ON" if is_aid_enabled(index) else "OFF"
