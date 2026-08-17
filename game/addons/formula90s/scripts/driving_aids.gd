extends Node
class_name DrivingAidsController

signal aid_toggled(aid_label: String, enabled: bool)

@export var vehicle_node: Node
var aids := [false, false, false, false, false]
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
	_baseline["enable_stability"] = VehicleTunableContract.get_value(vehicle_node, "enable_stability")
	_baseline["stability_yaw_strength"] = VehicleTunableContract.get_value(vehicle_node, "stability_yaw_strength")
	_baseline["steering_exponent"] = VehicleTunableContract.get_value(vehicle_node, "steering_exponent")
	_baseline["brake_force_multiplier"] = VehicleTunableContract.get_value(vehicle_node, "brake_force_multiplier")
	var fric = VehicleTunableContract.get_value(vehicle_node, "coefficient_of_friction")
	_baseline["friction"] = fric.duplicate() if fric else {}
	var lat_grip = VehicleTunableContract.get_value(vehicle_node, "lateral_grip_assist")
	_baseline["lateral_grip_assist"] = lat_grip.duplicate() if lat_grip else {}
	var auto_trans = VehicleTunableContract.get_value(vehicle_node, "automatic_transmission")
	_baseline["automatic_transmission"] = auto_trans
	if auto_trans != null:
		aids[0] = bool(auto_trans)
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
	aid_toggled.emit(get_aid_label(index), aids[index])

func _apply_aids():
	for i in range(aids.size()):
		if aids[i]: _apply_aid(i)

func _apply_aid(index: int):
	match index:
		0:
			VehicleTunableContract.set_value(vehicle_node, "automatic_transmission", true)
		1:
			if _baseline.get("enable_stability") != null and _baseline.get("stability_yaw_strength") != null:
				VehicleTunableContract.set_value(vehicle_node, "enable_stability", true)
				VehicleTunableContract.set_value(vehicle_node, "stability_yaw_strength", max(_baseline["stability_yaw_strength"] * MULT_STABILITY, FLOOR_STABILITY_STRENGTH))
		2:
			if _baseline.get("steering_exponent") != null:
				VehicleTunableContract.set_value(vehicle_node, "steering_exponent", _baseline["steering_exponent"] * MULT_STEERING)
		3:
			if _baseline.get("brake_force_multiplier") != null:
				VehicleTunableContract.set_value(vehicle_node, "brake_force_multiplier", _baseline["brake_force_multiplier"] * MULT_BRAKING)
		4:
			if _baseline.get("friction") is Dictionary and not _baseline["friction"].is_empty():
				var friction = _baseline["friction"].duplicate()
				for key in friction:
					friction[key] = max(friction[key] * MULT_GRIP, FLOOR_GRIP)
				VehicleTunableContract.set_value(vehicle_node, "coefficient_of_friction", friction)
			if _baseline.get("lateral_grip_assist") is Dictionary and not _baseline["lateral_grip_assist"].is_empty():
				var grip = _baseline["lateral_grip_assist"].duplicate()
				for key in grip:
					grip[key] = max(grip[key] + 0.15, 0.15)
				VehicleTunableContract.set_value(vehicle_node, "lateral_grip_assist", grip)

func _restore(index: int):
	match index:
		0:
			var base_auto = _baseline.get("automatic_transmission")
			var restore_val: bool = bool(base_auto) if base_auto != null else false
			VehicleTunableContract.set_value(vehicle_node, "automatic_transmission", restore_val)
		1:
			if _baseline.get("enable_stability") != null:
				VehicleTunableContract.set_value(vehicle_node, "enable_stability", _baseline["enable_stability"])
			if _baseline.get("stability_yaw_strength") != null:
				VehicleTunableContract.set_value(vehicle_node, "stability_yaw_strength", _baseline["stability_yaw_strength"])
		2:
			if _baseline.get("steering_exponent") != null:
				VehicleTunableContract.set_value(vehicle_node, "steering_exponent", _baseline["steering_exponent"])
		3:
			if _baseline.get("brake_force_multiplier") != null:
				VehicleTunableContract.set_value(vehicle_node, "brake_force_multiplier", _baseline["brake_force_multiplier"])
		4:
			if _baseline.get("friction") is Dictionary and not _baseline["friction"].is_empty():
				VehicleTunableContract.set_value(vehicle_node, "coefficient_of_friction", _baseline["friction"])
			if _baseline.get("lateral_grip_assist") is Dictionary and not _baseline["lateral_grip_assist"].is_empty():
				VehicleTunableContract.set_value(vehicle_node, "lateral_grip_assist", _baseline["lateral_grip_assist"])

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
