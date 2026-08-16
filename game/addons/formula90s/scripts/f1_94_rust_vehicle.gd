## F1-94 Rust Vehicle — High-performance GDScript controller & wrapper for F194RustVehicle GDExtension
## Bridges the deterministic 6-DOF Rust physics engine with Godot's audio, telemetry, HUD, and tuning contracts.

class_name F194RustVehicleGD
extends F194RustVehicle

# ==============================================================================
# 1. 12-RAYCAST SENSOR RIG REFERENCES (FL, FR, RL, RR x Inner, Center, Outer)
# ==============================================================================
@export_group("12-RayCast Sensor Rig")
@export var ray_fl_in: RayCast3D
@export var ray_fl_mid: RayCast3D
@export var ray_fl_out: RayCast3D

@export var ray_fr_in: RayCast3D
@export var ray_fr_mid: RayCast3D
@export var ray_fr_out: RayCast3D

@export var ray_rl_in: RayCast3D
@export var ray_rl_mid: RayCast3D
@export var ray_rl_out: RayCast3D

@export var ray_rr_in: RayCast3D
@export var ray_rr_mid: RayCast3D
@export var ray_rr_out: RayCast3D

# Short property aliases for direct access
var fl_in: RayCast3D:
	get: return ray_fl_in
	set(v): ray_fl_in = v

var fl_mid: RayCast3D:
	get: return ray_fl_mid
	set(v): ray_fl_mid = v

var fl_out: RayCast3D:
	get: return ray_fl_out
	set(v): ray_fl_out = v

var fr_in: RayCast3D:
	get: return ray_fr_in
	set(v): ray_fr_in = v

var fr_mid: RayCast3D:
	get: return ray_fr_mid
	set(v): ray_fr_mid = v

var fr_out: RayCast3D:
	get: return ray_fr_out
	set(v): ray_fr_out = v

var rl_in: RayCast3D:
	get: return ray_rl_in
	set(v): ray_rl_in = v

var rl_mid: RayCast3D:
	get: return ray_rl_mid
	set(v): ray_rl_mid = v

var rl_out: RayCast3D:
	get: return ray_rl_out
	set(v): ray_rl_out = v

var rr_in: RayCast3D:
	get: return ray_rr_in
	set(v): ray_rr_in = v

var rr_mid: RayCast3D:
	get: return ray_rr_mid
	set(v): ray_rr_mid = v

var rr_out: RayCast3D:
	get: return ray_rr_out
	set(v): ray_rr_out = v

# ==============================================================================
# 2. TUNING & VEHICLE SPEC PROPERTIES (VehicleTunableContract Compatibility)
# ==============================================================================
@export_group("Tuning & Physics Specifications")
@export var front_weight_distribution: float = 0.45
@export var center_of_gravity_height_offset: float = 0.0
@export var inertia_multiplier: float = 1.1

# Controller input property forwarders (for GEVP / FormulaVehicleController parity)
var throttle_input: float:
	get: return throttle_amount
	set(v): throttle_amount = v

var brake_input: float:
	get: return brake_amount
	set(v): brake_amount = v

var handbrake_input: float:
	get: return handbrake_amount
	set(v): handbrake_amount = v

var clutch_input: float:
	get: return clutch_amount
	set(v): clutch_amount = v

# Node references for GEVP / TelemetryManager / Audio duck typing
@onready var front_left_wheel: Node3D = get_node_or_null("FrontLeftWheel")
@onready var front_right_wheel: Node3D = get_node_or_null("FrontRightWheel")
@onready var rear_left_wheel: Node3D = get_node_or_null("RearLeftWheel")
@onready var rear_right_wheel: Node3D = get_node_or_null("RearRightWheel")

# ==============================================================================
# 3. INITIALIZATION & LIFECYCLE
# ==============================================================================
func _ready() -> void:
	_resolve_raycast_references()

func _integrate_forces(state: PhysicsDirectBodyState3D) -> void:
	solve_forces_for_state(state)

func _resolve_raycast_references() -> void:
	if ray_fl_in == null: ray_fl_in = _find_ray("RayCast_FL_In", "FL_In")
	if ray_fl_mid == null: ray_fl_mid = _find_ray("RayCast_FL_Mid", "FL_Mid")
	if ray_fl_out == null: ray_fl_out = _find_ray("RayCast_FL_Out", "FL_Out")

	if ray_fr_in == null: ray_fr_in = _find_ray("RayCast_FR_In", "FR_In")
	if ray_fr_mid == null: ray_fr_mid = _find_ray("RayCast_FR_Mid", "FR_Mid")
	if ray_fr_out == null: ray_fr_out = _find_ray("RayCast_FR_Out", "FR_Out")

	if ray_rl_in == null: ray_rl_in = _find_ray("RayCast_RL_In", "RL_In")
	if ray_rl_mid == null: ray_rl_mid = _find_ray("RayCast_RL_Mid", "RL_Mid")
	if ray_rl_out == null: ray_rl_out = _find_ray("RayCast_RL_Out", "RL_Out")

	if ray_rr_in == null: ray_rr_in = _find_ray("RayCast_RR_In", "RR_In")
	if ray_rr_mid == null: ray_rr_mid = _find_ray("RayCast_RR_Mid", "RR_Mid")
	if ray_rr_out == null: ray_rr_out = _find_ray("RayCast_RR_Out", "RR_Out")

func _find_ray(name_primary: String, name_alt: String) -> RayCast3D:
	var n = get_node_or_null(name_primary)
	if n is RayCast3D:
		return n as RayCast3D
	n = get_node_or_null(name_alt)
	if n is RayCast3D:
		return n as RayCast3D
	n = find_child(name_primary, true, false)
	if n is RayCast3D:
		return n as RayCast3D
	n = find_child(name_alt, true, false)
	if n is RayCast3D:
		return n as RayCast3D
	return null

# ==============================================================================
# 4. GEARBOX & TRANSMISSION CONTROLLER
# ==============================================================================
func manual_shift(direction: int) -> void:
	if automatic_transmission:
		return
	if direction > 0:
		gear_request = current_gear + 1
	elif direction < 0:
		gear_request = current_gear - 1
	else:
		gear_request = 0

# ==============================================================================
# 5. TELEMETRY HELPERS & SENSOR RIG ACCESSORS
# ==============================================================================
func get_raycast_list() -> Array[RayCast3D]:
	return [
		ray_fl_in, ray_fl_mid, ray_fl_out,
		ray_fr_in, ray_fr_mid, ray_fr_out,
		ray_rl_in, ray_rl_mid, ray_rl_out,
		ray_rr_in, ray_rr_mid, ray_rr_out
	]

func get_raycast_dict() -> Dictionary:
	return {
		"FL_In": ray_fl_in, "FL_Mid": ray_fl_mid, "FL_Out": ray_fl_out,
		"FR_In": ray_fr_in, "FR_Mid": ray_fr_mid, "FR_Out": ray_fr_out,
		"RL_In": ray_rl_in, "RL_Mid": ray_rl_mid, "RL_Out": ray_rl_out,
		"RR_In": ray_rr_in, "RR_Mid": ray_rr_mid, "RR_Out": ray_rr_out
	}

func get_max_front_slip() -> float:
	var slips = get_wheel_slips()
	if slips.size() >= 2:
		return maxf(absf(slips[0]), absf(slips[1]))
	return 0.0

func get_max_rear_slip() -> float:
	var slips = get_wheel_slips()
	if slips.size() >= 4:
		return maxf(absf(slips[2]), absf(slips[3]))
	return 0.0

func get_telemetry_snapshot() -> Dictionary:
	var comp = get_wheel_compressions()
	var spins = get_wheel_spins()
	var slips = get_wheel_slips()
	return {
		"speed_ms": speed,
		"speed_kmh": speed_kmh,
		"rpm": motor_rpm,
		"gear": current_gear,
		"throttle": throttle_amount,
		"brake": brake_amount,
		"steering": steering_input,
		"lat_g": lat_g,
		"long_g": long_g,
		"vert_g": vert_g,
		"wheel_compressions": comp,
		"wheel_spins": spins,
		"wheel_slips": slips,
		"position": global_position,
		"linear_velocity": linear_velocity,
		"angular_velocity": angular_velocity
	}
