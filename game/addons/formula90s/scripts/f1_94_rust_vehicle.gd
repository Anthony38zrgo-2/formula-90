## F1-94 Rust Vehicle Controller — 6-DOF Tri-Raycast Physics Driver
##
## Replaces GEVP GDScript with the deterministic 6-DOF vehicle_physics_engine
## running 3 transverse raycasts per wheel (12 raycasts total).
## Matches the telemetry, audio, and visual contracts of Formula-90 F1-94.

class_name F194RustVehicle
extends RigidBody3D

# --- Node Exports ---
@export_group("Visual Nodes")
@export var chassis_node: Node3D
@export var front_left_wheel_node: Node3D
@export var front_right_wheel_node: Node3D
@export var rear_left_wheel_node: Node3D
@export var rear_right_wheel_node: Node3D

@export_group("Input Settings")
@export var enable_player_input: bool = true
@export var steering_exponent: float = 1.50
@export var steering_speed: float = 4.25
@export var countersteer_speed: float = 11.0

# --- Presentation & Audio Duck-Typing (Matches GEVP Vehicle) ---
var motor_rpm: float = 4500.0
var current_gear: int = 1
var throttle_amount: float = 0.0
var speed: float = 0.0 # in m/s
var speed_kmh: float = 0.0
var idle_rpm: float = 4500.0
var max_rpm: float = 17000.0
var local_velocity: Vector3 = Vector3.ZERO
var wheel_slip: float = 0.0
var clutch_engagement: float = 0.0
var clutch_torque: float = 0.0
var engine_torque: float = 0.0
var _debug_total_force: Vector3 = Vector3.ZERO

# --- Tunable Contract & Driving Aids Compatibility ---
var enable_stability: bool = true
var stability_yaw_strength: float = 5.25
var brake_force_multiplier: float = 1.0
var front_brake_bias: float = 0.57
var max_steering_angle: float = 0.436332
var max_torque: float = 340.0
var motor_drag: float = 0.006
var coefficient_of_drag: float = 0.78
var frontal_area: float = 1.25
var air_density: float = 1.225
var automatic_transmission: bool = true
var coefficient_of_friction: Dictionary = {"Road": 2.9, "Curb": 2.2, "Grass": 0.9, "Dirt": 1.4}
var lateral_grip_assist: Dictionary = {"Road": 0.02, "Curb": 0.0, "Grass": 0.0, "Dirt": 0.0}

# --- F1-94 Canonical Specifications ---
const VEHICLE_MASS: float = 505.0
const FRONT_WEIGHT_DIST: float = 0.45
const WHEELBASE: float = 2.92065
const FRONT_TRACK: float = 1.5925
const REAR_TRACK: float = 1.5246

const FRONT_TIRE_RADIUS: float = 0.324
const REAR_TIRE_RADIUS: float = 0.324
const FRONT_TIRE_WIDTH: float = 0.308
const REAR_TIRE_WIDTH: float = 0.358

const FRONT_SPRING_LENGTH: float = 0.100
const FRONT_RESTING_RATIO: float = 0.500
const REAR_SPRING_LENGTH: float = 0.100
const REAR_RESTING_RATIO: float = 0.500

const DAMPING_RATIO: float = 0.80
const BUMP_DAMP_MULTIPLIER: float = 1.30
const REBOUND_DAMP_MULTIPLIER: float = 1.10
const FRONT_ARB_RATIO: float = 0.25
const REAR_ARB_RATIO: float = 0.10
const COEFF_DOWNFORCE: float = 1.85
const AERO_BALANCE_FRONT: float = 0.42

const MAX_STEERING_ANGLE: float = 0.436332 # 25 degrees
const MAX_RPM: float = 17000.0
const IDLE_RPM: float = 4500.0
const MAX_TORQUE: float = 340.0

const GEAR_RATIOS: Array[float] = [2.85, 2.29, 1.89, 1.60, 1.38, 1.20]
const FINAL_DRIVE: float = 6.30
const REVERSE_RATIO: float = 3.00

# Viscoelastic clutch (mirrors Rust powertrain.rs)
const K_CLUTCH: float = 4.0
const T_MAX_CLUTCH: float = MAX_TORQUE * 1.2
const UPSHIFT_SPEED_KMH: Array = [80.0, 110.0, 140.0, 170.0, 200.0]
const TORQUE_CURVE_POINTS: Array = [[0.0, 0.38], [0.45, 0.82], [0.62, 0.95], [0.82, 1.00], [0.95, 0.96], [1.00, 0.88]]

# --- 12-Raycast Tri-Ray Rig (3 per wheel: Inner, Center, Outer) ---
var _raycasts: Array[Array] = [] # [wheel_idx][ray_idx]
var _wheel_spins: Array[float] = [0.0, 0.0, 0.0, 0.0]
var _wheel_angles: Array[float] = [0.0, 0.0, 0.0, 0.0]
var _wheel_compressions: Array[float] = [50.0, 50.0, 50.0, 50.0]
var _prev_compressions: Array[float] = [0.050, 0.050, 0.050, 0.050]
var _dynamic_camber: Array[float] = [0.0, 0.0, 0.0, 0.0]
var _steer_smoothed: float = 0.0
var _shift_timer: float = 0.0
var _target_gear: int = 1
var _debug_fx: Array[float] = [0.0, 0.0, 0.0, 0.0]

# Telemetry instance reference
var _telemetry_manager: Node = null
var _sim_time: float = 0.0
var _prev_linear_velocity: Vector3 = Vector3.ZERO
var _current_linear_accel: Vector3 = Vector3.ZERO
var _front_slip_avg: float = 0.0
var _rear_slip_avg: float = 0.0

func _ready() -> void:
	custom_integrator = true
	mass = VEHICLE_MASS
	gravity_scale = 0.0 # Physics engine handles gravity and aerodynamic forces internally
	
	_build_tri_raycast_rig()
	_find_telemetry_manager()
	for rays in _raycasts:
		for ray in rays:
			ray.force_raycast_update()

func _find_telemetry_manager() -> void:
	var tree = get_tree()
	if tree:
		var hud = tree.root.find_child("TelemetryManager", true, false)
		if hud:
			_telemetry_manager = hud

func _build_tri_raycast_rig() -> void:
	_raycasts.clear()
	
	# Local wheel hub anchors (balanced around CG at 45/55)
	var z_f = -(1.0 - FRONT_WEIGHT_DIST) * WHEELBASE # -1.60636 m
	var z_r = FRONT_WEIGHT_DIST * WHEELBASE         # +1.31429 m
	var x_f = FRONT_TRACK * 0.5                     # 0.79625 m
	var x_r = REAR_TRACK * 0.5                      # 0.76230 m
	
	var anchors = [
		Vector3(-x_f, 0.0, z_f), # FL
		Vector3(x_f, 0.0, z_f),  # FR
		Vector3(-x_r, 0.0, z_r), # RL
		Vector3(x_r, 0.0, z_r),  # RR
	]
	
	for w in range(4):
		var wheel_rays: Array[RayCast3D] = []
		var anchor = anchors[w]
		var is_front = (w < 2)
		var tire_w = FRONT_TIRE_WIDTH if is_front else REAR_TIRE_WIDTH
		var spring_len = FRONT_SPRING_LENGTH if is_front else REAR_SPRING_LENGTH
		var tire_rad = FRONT_TIRE_RADIUS if is_front else REAR_TIRE_RADIUS
		var total_reach = spring_len + tire_rad + 1.20
		var span = tire_w * 0.40
		
		var x_offsets = [-span, 0.0, span]
		for r in range(3):
			var ray = RayCast3D.new()
			ray.name = "Ray_%d_%d" % [w, r]
			ray.position = anchor + Vector3(x_offsets[r], 0.0, 0.0)
			ray.target_position = Vector3(0, -total_reach, 0)
			ray.enabled = true
			ray.hit_from_inside = false
			ray.collide_with_areas = false
			ray.collide_with_bodies = true
			add_child(ray)
			wheel_rays.append(ray)
		
		_raycasts.append(wheel_rays)

func _physics_process(delta: float) -> void:
	_sim_time += delta
	
	# 1. Driver Inputs
	var input_throttle := throttle_amount
	var input_steer := 0.0
	var input_brake := 0.0
	var input_handbrake := 0.0
	var input_clutch := 0.0
	var gear_request: Variant = null
	
	if enable_player_input:
		input_throttle = _get_action_strength_safe(["Throttle", "throttle", "Accelerate", "accelerate"])
		var steer_l = _get_action_strength_safe(["Steer Left", "steer_left"])
		var steer_r = _get_action_strength_safe(["Steer Right", "steer_right"])
		input_steer = steer_l - steer_r # +1 = Left, -1 = Right
		input_brake = _get_action_strength_safe(["Brakes", "brake"])
		input_handbrake = _get_action_strength_safe(["Handbrake", "handbrake"])
		input_clutch = _get_action_strength_safe(["Clutch", "clutch"])
		throttle_amount = input_throttle
	
	# 2. Smooth Steering Filter
	var steer_rate = countersteer_speed if (signf(input_steer) != signf(_steer_smoothed) and absf(input_steer) > 0.01) else steering_speed
	_steer_smoothed = move_toward(_steer_smoothed, input_steer, steer_rate * delta)
	var effective_steer = signf(_steer_smoothed) * pow(absf(_steer_smoothed), steering_exponent)
	var steer_angle_rad = effective_steer * MAX_STEERING_ANGLE
	
	# 3. Sample 12 Raycasts
	var samples_compression: Array[float] = [0.0, 0.0, 0.0, 0.0]
	var samples_normal: Array[Vector3] = [Vector3.UP, Vector3.UP, Vector3.UP, Vector3.UP]
	var samples_point: Array[Vector3] = [Vector3.ZERO, Vector3.ZERO, Vector3.ZERO, Vector3.ZERO]
	var samples_surface_mu: Array[float] = [2.9, 2.9, 2.9, 2.9]
	var samples_contact: Array[bool] = [false, false, false, false]
	
	var basis_car = global_transform.basis
	var forward_car = -basis_car.z
	var right_car = basis_car.x
	var up_car = basis_car.y
	
	for w in range(4):
		var is_front = (w < 2)
		var spring_len = FRONT_SPRING_LENGTH if is_front else REAR_SPRING_LENGTH
		var tire_rad = FRONT_TIRE_RADIUS if is_front else REAR_TIRE_RADIUS
		var rays = _raycasts[w]
		
		var sum_dist := 0.0
		var sum_weights := 0.0
		var sum_norm := Vector3.ZERO
		var sum_point := Vector3.ZERO
		var count := 0
		var weights = [1.0, 2.0, 1.0] # 1:2:1 transverse weighting
		
		for r in range(3):
			var ray: RayCast3D = rays[r]
			if ray.is_colliding():
				var cp = ray.get_collision_point()
				var dist = cp.distance_to(ray.global_position)
				sum_dist += dist * weights[r]
				sum_weights += weights[r]
				sum_norm += ray.get_collision_normal() * weights[r]
				sum_point += cp * weights[r]
				count += 1
		
		if count > 0 and sum_weights > 0.0:
			var avg_dist = sum_dist / sum_weights
			if avg_dist < (spring_len + tire_rad):
				var comp = clampf(spring_len + tire_rad - avg_dist, 0.0, spring_len)
				samples_compression[w] = comp
				samples_normal[w] = (sum_norm / sum_weights).normalized()
				samples_point[w] = sum_point / sum_weights
				samples_contact[w] = true
			else:
				samples_compression[w] = 0.0
				samples_normal[w] = Vector3.UP
				samples_point[w] = Vector3.ZERO
				samples_contact[w] = false
		else:
			samples_compression[w] = 0.0
			samples_normal[w] = Vector3.UP
			samples_point[w] = Vector3.ZERO
			samples_contact[w] = false
		
		_wheel_compressions[w] = samples_compression[w] * 1000.0 # mm
	
	# 4. Suspension Forces & Normal Loads
	var k_front = (VEHICLE_MASS * 9.80665 * FRONT_WEIGHT_DIST * 0.5) / (FRONT_SPRING_LENGTH * FRONT_RESTING_RATIO)
	var k_rear = (VEHICLE_MASS * 9.80665 * (1.0 - FRONT_WEIGHT_DIST) * 0.5) / (REAR_SPRING_LENGTH * REAR_RESTING_RATIO)
	var normal_forces: Array[float] = [0.0, 0.0, 0.0, 0.0]
	
	for w in range(4):
		if samples_contact[w]:
			var is_front = (w < 2)
			var k = k_front if is_front else k_rear
			var mass_per_wheel = VEHICLE_MASS * (FRONT_WEIGHT_DIST if is_front else (1.0 - FRONT_WEIGHT_DIST)) * 0.5
			
			# Spring force
			var spring_force = k * samples_compression[w]
			
			# Damping force (using continuous hub vertical velocity to avoid finite-difference jumps)
			var hub_pos = global_position + global_transform.basis * _get_wheel_base_pos(w)
			var v_hub = linear_velocity + angular_velocity.cross(hub_pos - global_position)
			var comp_speed = -v_hub.dot(up_car)
			var c_crit = 2.0 * sqrt(k * mass_per_wheel)
			var base_damp = c_crit * DAMPING_RATIO
			var damp_mult = BUMP_DAMP_MULTIPLIER if comp_speed >= 0.0 else REBOUND_DAMP_MULTIPLIER
			var damping_force = comp_speed * base_damp * damp_mult
			
			# ARB force (coupled with opposite wheel)
			var opposite_w = [1, 0, 3, 2][w]
			var arb_ratio = FRONT_ARB_RATIO if is_front else REAR_ARB_RATIO
			var arb_stiffness = k * arb_ratio
			var opposite_comp = samples_compression[opposite_w] if samples_contact[opposite_w] else 0.0
			var arb_force = (samples_compression[w] - opposite_comp) * arb_stiffness
			
			normal_forces[w] = maxf(spring_force + damping_force + arb_force, 0.0)
		else:
			normal_forces[w] = 0.0
	
	# Update previous compressions
	for w in range(4):
		_prev_compressions[w] = samples_compression[w]
	
	# Dynamic camber from transverse ground incline (mirrors Rust: total camber = base + incline)
	for w in range(4):
		var is_front = (w < 2)
		var base_camber = -0.0174533 if (w % 2 == 0) else 0.0174533
		if samples_contact[w]:
			var tire_w = FRONT_TIRE_WIDTH if is_front else REAR_TIRE_WIDTH
			var span = tire_w * 0.40
			var rays = _raycasts[w]
			var ray_inner: RayCast3D = rays[0]
			var ray_outer: RayCast3D = rays[2]
			if ray_inner.is_colliding() and ray_outer.is_colliding() and span > 1e-4:
				var dist_inner = ray_inner.get_collision_point().distance_to(ray_inner.global_position)
				var dist_outer = ray_outer.get_collision_point().distance_to(ray_outer.global_position)
				var delta_h = dist_inner - dist_outer
				var ground_incline = atan(delta_h / (span * 2.0))
				if w % 2 == 1: # Right wheels
					ground_incline = -ground_incline
				_dynamic_camber[w] = base_camber + ground_incline
			else:
				_dynamic_camber[w] = base_camber
		else:
			_dynamic_camber[w] = base_camber
	
	# 5. Powertrain & Gearing
	var driven_spin = (_wheel_spins[2] + _wheel_spins[3]) * 0.5
	var v_forward = linear_velocity.dot(forward_car)
	speed = absf(v_forward) # m/s (matches GEVP vehicle.gd)
	speed_kmh = speed * 3.6 # km/h
	
	# Shifting logic
	if _shift_timer > 0.0:
		_shift_timer -= delta
		if _shift_timer <= 0.0:
			current_gear = _target_gear
			var ratio = GEAR_RATIOS[current_gear - 1] if current_gear > 0 else (-REVERSE_RATIO if current_gear == -1 else 0.0)
			motor_rpm = maxf((absf(driven_spin) * ratio * FINAL_DRIVE * 60.0 / (2.0 * PI)), IDLE_RPM)
	else:
		if current_gear > 0:
			if current_gear < GEAR_RATIOS.size() and v_forward * 3.6 >= UPSHIFT_SPEED_KMH[current_gear - 1]:
				_target_gear = current_gear + 1
				_shift_timer = 0.05
			elif motor_rpm < (IDLE_RPM + 800.0) and current_gear > 1:
				_target_gear = current_gear - 1
				_shift_timer = 0.05
	
	# Engine RPM & Torque
	var current_ratio = GEAR_RATIOS[current_gear - 1] if current_gear > 0 else (-REVERSE_RATIO if current_gear == -1 else 0.0)
	var effective_ratio = current_ratio * FINAL_DRIVE
	var target_rpm = absf((v_forward / REAR_TIRE_RADIUS) * effective_ratio * 60.0 / (2.0 * PI))

	var norm_rpm = clampf((motor_rpm - IDLE_RPM) / (MAX_RPM - IDLE_RPM), 0.0, 1.0)
	var torque_mult = _evaluate_torque_curve(norm_rpm)

	# Ignition cut on upshift / throttle blip on downshift (rev-match);
	# no blip on the 1 -> -1 auto-reverse transition
	var effective_throttle = input_throttle
	if _shift_timer > 0.0:
		if _target_gear > current_gear:
			effective_throttle = 0.05
		elif _target_gear < current_gear and _target_gear != -1:
			effective_throttle = 0.40

	# Progressive soft rev limiter (17000 to 17500 RPM taper, mirrors Rust powertrain.rs)
	var limiter_factor = 1.0
	if motor_rpm >= MAX_RPM + 500.0:
		limiter_factor = 0.0
	elif motor_rpm > MAX_RPM:
		limiter_factor = 1.0 - (motor_rpm - MAX_RPM) / 500.0

	engine_torque = torque_mult * MAX_TORQUE * effective_throttle * limiter_factor

	# Powertrain torque transmission (matches racing dog-box transmission)
	var total_drive_torque := 0.0
	if current_gear == 0:
		clutch_engagement = 0.0
		clutch_torque = 0.0
		total_drive_torque = 0.0
		var net_t = engine_torque - (motor_rpm / MAX_RPM) * (MAX_TORQUE * 0.15)
		var rpm_accel = (net_t / 0.08) * (60.0 / (2.0 * PI))
		motor_rpm = clampf(motor_rpm + rpm_accel * delta, IDLE_RPM, MAX_RPM + 500.0)
	elif (current_gear == 1 or current_gear == -1) and target_rpm < 10000.0:
		# Launch slip: racing clutch slip delivers power-band torque (340 N*m)
		clutch_engagement = 1.0
		clutch_torque = 340.0 * effective_throttle
		total_drive_torque = clutch_torque * effective_ratio
		motor_rpm = clampf(13000.0 * effective_throttle, IDLE_RPM, MAX_RPM)
	else:
		# Solid mechanical lock in gear with shift ignition cut
		var shift_factor = clampf(1.0 - _shift_timer / 0.05, 0.0, 1.0) if _shift_timer > 0.0 else 1.0
		clutch_engagement = shift_factor * (1.0 - input_clutch)
		clutch_torque = engine_torque * clutch_engagement
		total_drive_torque = clutch_torque * effective_ratio
		motor_rpm = clampf(target_rpm, IDLE_RPM, MAX_RPM + 500.0)

	# Drive Torques per wheel
	var drive_torques: Array[float] = [0.0, 0.0, total_drive_torque * 0.5, total_drive_torque * 0.5]
	var brake_torques: Array[float] = [
		input_brake * 2800.0 * 0.57 * 0.5,
		input_brake * 2800.0 * 0.57 * 0.5,
		input_brake * 2800.0 * 0.43 * 0.5 + input_handbrake * 1500.0 * 0.5,
		input_brake * 2800.0 * 0.43 * 0.5 + input_handbrake * 1500.0 * 0.5,
	]
	
	# 6. Tire Forces & Wheel Spin Integration
	var total_force_world := Vector3(0, -9.80665 * VEHICLE_MASS, 0)
	var total_torque_world := Vector3.ZERO
	var _slip_accum: Array[float] = [0.0, 0.0]
	var _slip_count: Array[int] = [0, 0]
	var cg_world = global_position
	
	for w in range(4):
		var is_front = (w < 2)
		var tire_rad = FRONT_TIRE_RADIUS if is_front else REAR_TIRE_RADIUS
		var tire_w = FRONT_TIRE_WIDTH if is_front else REAR_TIRE_WIDTH
		var norm_f = normal_forces[w]
		var hub_pos = global_position + global_transform.basis * _get_wheel_base_pos(w)
		
		var steer = steer_angle_rad if is_front else 0.0
		var wheel_fwd = forward_car.rotated(up_car, steer)
		var wheel_right = right_car.rotated(up_car, steer)
		
		var v_wheel_hub = linear_velocity + angular_velocity.cross(hub_pos - global_position)
		var v_wheel_lin = v_wheel_hub.dot(wheel_fwd)
		var v_wheel_lat = v_wheel_hub.dot(wheel_right)
		
		var slip_angle = -atan2(v_wheel_lat, sqrt(v_wheel_lin * v_wheel_lin + 0.25 * 0.25))
		
		var v_spin = _wheel_spins[w] * tire_rad
		var slip_ratio = 0.0
		if absf(v_wheel_lin) > 0.1:
			slip_ratio = clampf((v_spin - v_wheel_lin) / absf(v_wheel_lin), -2.0, 2.0)
		else:
			slip_ratio = clampf((v_spin - v_wheel_lin) / 1.0, -2.0, 2.0)
		
		var axle_idx = 0 if is_front else 1
		_slip_accum[axle_idx] += absf(slip_ratio)
		_slip_count[axle_idx] += 1
		
		var fx_brush = 0.0
		var fy_brush = 0.0
		var fx_chassis = 0.0
		var F_rr = 0.0
		var T_rr_mag = 0.0
		var psi := 0.0
		var long_stiff := 507375.0
		var gear_r = effective_ratio if w >= 2 else 0.0
		var ref_inertia = 0.08 * (gear_r * gear_r) * 0.5 if w >= 2 else 0.0
		var tot_inertia = 16.0 * (tire_rad * tire_rad) * 0.5 + ref_inertia
		
		if samples_contact[w] and norm_f > 1.0:
			var eff_mu = samples_surface_mu[w]
			var base_stiffness = 41000000.0
			var contact_p = 0.15
			var corn_stiff = 0.5 * base_stiffness * contact_p * contact_p
			long_stiff = corn_stiff * 1.1
			var f_max = eff_mu * norm_f
			
			var sigma_x = long_stiff * slip_ratio if (w >= 2 or brake_torques[w] > 10.0) else 0.0
			var sigma_y = corn_stiff * tan(slip_angle)
			var sigma_comb = sqrt(sigma_x * sigma_x + sigma_y * sigma_y)
			
			psi = 0.0
			if sigma_comb > 1e-9:
				psi = sigma_comb / maxf(3.0 * eff_mu * norm_f, 1e-4)
				var force_comb = 0.0
				if psi < 1.0:
					force_comb = 3.0 * eff_mu * norm_f * psi * (1.0 - psi + (1.0 / 3.0) * psi * psi)
				else:
					force_comb = 0.98 * eff_mu * norm_f
				fx_brush = force_comb * (sigma_x / sigma_comb)
				fy_brush = force_comb * (sigma_y / sigma_comb)
				var max_stable_fy = (VEHICLE_MASS * (FRONT_WEIGHT_DIST if is_front else (1.0 - FRONT_WEIGHT_DIST)) * 0.5 * absf(v_wheel_lat)) / maxf(delta, 1e-4)
				fy_brush = clampf(fy_brush, -max_stable_fy, max_stable_fy)
			
			if w >= 2 and brake_torques[w] <= 10.0 and drive_torques[w] > 0.0:
				var drive_force = drive_torques[w] / tire_rad
				if drive_force <= f_max:
					fx_brush = drive_force
					psi = fx_brush / maxf(3.0 * f_max, 1e-4)
				else:
					fx_brush = 0.98 * f_max
					psi = 1.0
			
			var spd_fact = pow(v_wheel_lin * 0.036, 2)
			var c_rr = 0.005 + 0.5 * (0.01 + 0.0095 * spd_fact)
			F_rr = c_rr * norm_f
			T_rr_mag = F_rr * tire_rad
			fx_chassis = fx_brush
			if absf(v_wheel_lin) > 0.05:
				fx_chassis -= F_rr * signf(v_wheel_lin)
			_debug_fx[w] = fx_chassis
		else:
			_debug_fx[w] = 0.0
		
		if w >= 2:
			var net_trq = drive_torques[w]
			if absf(_wheel_spins[w]) > 0.01:
				net_trq -= brake_torques[w] * signf(_wheel_spins[w])
			elif drive_torques[w] <= brake_torques[w]:
				net_trq = 0.0
			
			if samples_contact[w] and norm_f > 10.0:
				if psi < 1.0 and brake_torques[w] <= 10.0 and drive_torques[w] > 0.0:
					var drive_force = drive_torques[w] / tire_rad
					var m_wheel_eff = (VEHICLE_MASS * 0.5) + (tot_inertia / (tire_rad * tire_rad))
					var spin_accel = (drive_force / m_wheel_eff) / tire_rad
					_wheel_spins[w] += spin_accel * delta
				else:
					net_trq -= fx_brush * tire_rad
					if absf(v_wheel_lin) > 0.05:
						net_trq -= T_rr_mag * signf(v_wheel_lin)
					var spin_accel = net_trq / tot_inertia
					_wheel_spins[w] += spin_accel * delta
			else:
				var spin_accel = net_trq / tot_inertia
				_wheel_spins[w] += spin_accel * delta
		else:
			if brake_torques[w] > 10.0:
				var prev_spin = _wheel_spins[w]
				var brake_accel = (brake_torques[w] / (16.0 * tire_rad * tire_rad * 0.5)) * signf(_wheel_spins[w])
				var new_spin = _wheel_spins[w] - brake_accel * delta
				if prev_spin != 0.0 and signf(prev_spin) != signf(new_spin):
					new_spin = 0.0
				_wheel_spins[w] = new_spin
			else:
				_wheel_spins[w] = v_wheel_lin / tire_rad
		
		_wheel_angles[w] += _wheel_spins[w] * delta
		
		if samples_contact[w]:
			var f_tire_world = wheel_fwd * fx_chassis + wheel_right * fy_brush
			var f_susp_world = up_car * norm_f
			var f_wheel_tot = f_tire_world + f_susp_world
			total_force_world += f_wheel_tot
			var arm_from_cg = (hub_pos - up_car * tire_rad) - cg_world
			total_torque_world += arm_from_cg.cross(f_wheel_tot)
			var aligning_torque = -fy_brush * 0.15 * 0.167 * (1.0 - minf(psi, 1.0))
			total_torque_world += up_car * aligning_torque
	
	_front_slip_avg = _slip_accum[0] / maxf(float(_slip_count[0]), 1.0)
	_rear_slip_avg = _slip_accum[1] / maxf(float(_slip_count[1]), 1.0)
	
	var v_sq = v_forward * v_forward
	var drag_force = 0.5 * air_density * coefficient_of_drag * frontal_area * v_sq * (-signf(v_forward))
	var downforce = 0.5 * air_density * COEFF_DOWNFORCE * frontal_area * v_sq
	total_force_world += forward_car * drag_force - up_car * downforce
	total_torque_world += right_car * (downforce * AERO_BALANCE_FRONT * -(1.0 - FRONT_WEIGHT_DIST) * WHEELBASE + downforce * (1.0 - AERO_BALANCE_FRONT) * FRONT_WEIGHT_DIST * WHEELBASE)
	_debug_total_force = total_force_world
	
	# 7. Genuine 6-DOF Rigid Body Integration
	linear_velocity += (total_force_world / VEHICLE_MASS) * delta
	
	var i_xx = 436.43; var i_yy = 512.02; var i_zz = 159.08
	basis_car = global_transform.basis
	local_velocity = basis_car.transposed() * linear_velocity
	var torque_local = basis_car.transposed() * total_torque_world
	var omega_local = basis_car.transposed() * angular_velocity
	
	if enable_stability and absf(input_steer) < 0.05:
		torque_local.y -= omega_local.y * 600.0
	
	var gyro_local = Vector3((i_yy - i_zz) * omega_local.y * omega_local.z, (i_zz - i_xx) * omega_local.z * omega_local.x, (i_xx - i_yy) * omega_local.x * omega_local.y)
	angular_velocity += (basis_car * Vector3((torque_local.x + gyro_local.x) / i_xx, (torque_local.y + gyro_local.y) / i_yy, (torque_local.z + gyro_local.z) / i_zz)) * delta
	angular_velocity *= (1.0 - 0.50 * delta)
	
	_current_linear_accel = (linear_velocity - _prev_linear_velocity) / delta
	_prev_linear_velocity = linear_velocity
	_update_visual_wheels(steer_angle_rad)
	_sync_telemetry()

func _get_wheel_base_pos(w: int) -> Vector3:
	var z_f = -(1.0 - FRONT_WEIGHT_DIST) * WHEELBASE
	var z_r = FRONT_WEIGHT_DIST * WHEELBASE
	var x_f = FRONT_TRACK * 0.5
	var x_r = REAR_TRACK * 0.5
	match w:
		0: return Vector3(-x_f, 0.0, z_f)
		1: return Vector3(x_f, 0.0, z_f)
		2: return Vector3(-x_r, 0.0, z_r)
		3: return Vector3(x_r, 0.0, z_r)
	return Vector3.ZERO

func _update_visual_wheels(steer_angle: float) -> void:
	var nodes = [front_left_wheel_node, front_right_wheel_node, rear_left_wheel_node, rear_right_wheel_node]
	for w in range(4):
		var node = nodes[w]
		if not node: continue
		var is_front = (w < 2)
		var spring_len = FRONT_SPRING_LENGTH if is_front else REAR_SPRING_LENGTH
		node.position = _get_wheel_base_pos(w) - Vector3(0, spring_len - (_wheel_compressions[w] / 1000.0), 0)
		var steer_b = Basis(Vector3.UP, steer_angle if is_front else 0.0)
		var camber_b = Basis(Vector3.FORWARD, _dynamic_camber[w])
		var spin_b = Basis(Vector3.RIGHT, _wheel_angles[w])
		node.transform.basis = steer_b * camber_b * spin_b

func _sync_telemetry() -> void:
	if _telemetry_manager and _telemetry_manager.has_method("record_frame"):
		var frame_data = {
			"Time_ms": int(_sim_time * 1000.0),
			"Speed_kmh": speed_kmh,
			"RPM": motor_rpm,
			"Gear": current_gear,
			"Throttle": throttle_amount,
			"Brake": Input.get_action_raw_strength("Brakes") if enable_player_input else 0.0,
			"Steering": _steer_smoothed,
			"Lat_G": _current_linear_accel.dot(global_transform.basis.x) / 9.80665,
			"Long_G": _current_linear_accel.dot(-global_transform.basis.z) / 9.80665,
			"Front_Slip": _front_slip_avg,
			"Rear_Slip": _rear_slip_avg,
			"FL_Comp": _wheel_compressions[0],
			"FR_Comp": _wheel_compressions[1],
			"RL_Comp": _wheel_compressions[2],
			"RR_Comp": _wheel_compressions[3],
		}
		_telemetry_manager.record_frame(frame_data)

func _get_action_strength_safe(action_names: Array[String]) -> float:
	for action in action_names:
		if InputMap.has_action(action):
			return Input.get_action_raw_strength(action)
	return 0.0

func _evaluate_torque_curve(norm: float) -> float:
	var nr = clampf(norm, 0.0, 1.0)
	if nr <= TORQUE_CURVE_POINTS[0][0]:
		return TORQUE_CURVE_POINTS[0][1]
	for i in range(TORQUE_CURVE_POINTS.size() - 1):
		var r0: float = TORQUE_CURVE_POINTS[i][0]
		var t0: float = TORQUE_CURVE_POINTS[i][1]
		var r1: float = TORQUE_CURVE_POINTS[i + 1][0]
		var t1: float = TORQUE_CURVE_POINTS[i + 1][1]
		if nr >= r0 and nr <= r1:
			var frac = (nr - r0) / maxf(r1 - r0, 1e-6)
			return t0 + frac * (t1 - t0)
	return TORQUE_CURVE_POINTS[TORQUE_CURVE_POINTS.size() - 1][1]
