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
var engine_torque: float = 0.0

# --- Tunable Contract & Driving Aids Compatibility ---
var enable_stability: bool = true
var stability_yaw_strength: float = 5.25
var brake_force_multiplier: float = 1.0
var front_brake_bias: float = 0.57
var max_steering_angle: float = 0.436332
var max_torque: float = 340.0
var motor_drag: float = 0.006
var coefficient_of_drag: float = 0.15
var frontal_area: float = 0.45
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

const FRONT_TIRE_RADIUS: float = 0.31695
const REAR_TIRE_RADIUS: float = 0.32901
const FRONT_TIRE_WIDTH: float = 0.30030
const REAR_TIRE_WIDTH: float = 0.36832

const FRONT_SPRING_LENGTH: float = 0.250
const FRONT_RESTING_RATIO: float = 0.400
const REAR_SPRING_LENGTH: float = 0.180
const REAR_RESTING_RATIO: float = 0.350

const MAX_STEERING_ANGLE: float = 0.436332 # 25 degrees
const MAX_RPM: float = 17000.0
const IDLE_RPM: float = 4500.0
const MAX_TORQUE: float = 340.0

const GEAR_RATIOS: Array[float] = [2.85, 2.29, 1.89, 1.60, 1.38, 1.20]
const FINAL_DRIVE: float = 6.30
const REVERSE_RATIO: float = 3.00

# --- 12-Raycast Tri-Ray Rig (3 per wheel: Inner, Center, Outer) ---
var _raycasts: Array[Array] = [] # [wheel_idx][ray_idx]
var _wheel_spins: Array[float] = [0.0, 0.0, 0.0, 0.0]
var _wheel_angles: Array[float] = [0.0, 0.0, 0.0, 0.0]
var _wheel_compressions: Array[float] = [100.0, 100.0, 63.0, 63.0]
var _steer_smoothed: float = 0.0
var _shift_timer: float = 0.0
var _target_gear: int = 1

# Telemetry instance reference
var _telemetry_manager: Node = null
var _sim_time: float = 0.0
var _prev_linear_velocity: Vector3 = Vector3.ZERO
var _current_linear_accel: Vector3 = Vector3.ZERO

func _ready() -> void:
	custom_integrator = true
	mass = VEHICLE_MASS
	gravity_scale = 0.0 # Physics engine handles gravity and aerodynamic forces internally
	
	_build_tri_raycast_rig()
	_find_telemetry_manager()

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
		Vector3(-x_f, 0.144, z_f), # FL
		Vector3(x_f, 0.144, z_f),  # FR
		Vector3(-x_r, 0.186, z_r), # RL
		Vector3(x_r, 0.186, z_r),  # RR
	]
	
	for w in range(4):
		var wheel_rays: Array[RayCast3D] = []
		var anchor = anchors[w]
		var is_front = (w < 2)
		var tire_w = FRONT_TIRE_WIDTH if is_front else REAR_TIRE_WIDTH
		var spring_len = FRONT_SPRING_LENGTH if is_front else REAR_SPRING_LENGTH
		var tire_rad = FRONT_TIRE_RADIUS if is_front else REAR_TIRE_RADIUS
		var total_reach = spring_len + tire_rad + 0.10
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
		input_throttle = _get_action_strength_safe(["Accelerate", "accelerate"])
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
		var count := 0
		var weights = [1.0, 2.0, 1.0] # 1:2:1 transverse weighting
		
		for r in range(3):
			var ray: RayCast3D = rays[r]
			if ray.is_colliding():
				var dist = ray.get_collision_point().distance_to(ray.global_position)
				sum_dist += dist * weights[r]
				sum_weights += weights[r]
				sum_norm += ray.get_collision_normal() * weights[r]
				count += 1
		
		if count > 0 && sum_weights > 0.0:
			var avg_dist = sum_dist / sum_weights
			var comp = clampf(spring_len + tire_rad - avg_dist, 0.0, spring_len)
			samples_compression[w] = comp
			samples_normal[w] = (sum_norm / sum_weights).normalized()
			samples_contact[w] = true
		else:
			samples_compression[w] = 0.0
			samples_normal[w] = up_car
			samples_contact[w] = false
		
		_wheel_compressions[w] = samples_compression[w] * 1000.0 # mm
	
	# 4. Suspension Forces & Normal Loads
	var k_front = (VEHICLE_MASS * 9.80665 * FRONT_WEIGHT_DIST * 0.5) / (FRONT_SPRING_LENGTH * (1.0 - FRONT_RESTING_RATIO))
	var k_rear = (VEHICLE_MASS * 9.80665 * (1.0 - FRONT_WEIGHT_DIST) * 0.5) / (REAR_SPRING_LENGTH * (1.0 - REAR_RESTING_RATIO))
	var normal_forces: Array[float] = [0.0, 0.0, 0.0, 0.0]
	
	for w in range(4):
		if samples_contact[w]:
			var k = k_front if (w < 2) else k_rear
			normal_forces[w] = k * samples_compression[w]
		else:
			normal_forces[w] = 0.0
	
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
			var ratio = GEAR_RATIOS[current_gear - 1] if current_gear > 0 else REVERSE_RATIO
			motor_rpm = maxf((absf(driven_spin) * ratio * FINAL_DRIVE * 60.0 / (2.0 * PI)), IDLE_RPM)
	else:
		if current_gear > 0:
			if motor_rpm > MAX_RPM * 0.90 && current_gear < GEAR_RATIOS.size():
				_target_gear = current_gear + 1
				_shift_timer = 0.12
			elif motor_rpm < (IDLE_RPM + 800.0) && current_gear > 1:
				_target_gear = current_gear - 1
				_shift_timer = 0.12
	
	# Engine RPM & Torque
	var current_ratio = GEAR_RATIOS[current_gear - 1] if current_gear > 0 else (REVERSE_RATIO if current_gear == -1 else 0.0)
	var effective_ratio = current_ratio * FINAL_DRIVE
	var target_rpm = (absf(driven_spin) * effective_ratio * 60.0 / (2.0 * PI))
	
	var norm_rpm = clampf((motor_rpm - IDLE_RPM) / (MAX_RPM - IDLE_RPM), 0.0, 1.0)
	var torque_mult = 0.38 + norm_rpm * 0.62
	engine_torque = torque_mult * MAX_TORQUE * input_throttle
	
	var clutch_bite = 1.0
	if current_gear == 1 && target_rpm < IDLE_RPM * 1.6:
		clutch_bite = clampf((motor_rpm - IDLE_RPM) / (IDLE_RPM * 0.6), 0.0, 1.0)
	clutch_engagement = clutch_bite * (1.0 - input_clutch) * (0.0 if _shift_timer > 0.0 else 1.0)
	
	if clutch_engagement >= 0.85 && current_gear != 0:
		motor_rpm = move_toward(motor_rpm, maxf(target_rpm, IDLE_RPM), 18000.0 * delta)
	else:
		var net_t = engine_torque - (motor_rpm / MAX_RPM) * 30.0 - clutch_engagement * engine_torque * 0.85
		var rpm_accel = (net_t / 0.08) * (60.0 / (2.0 * PI))
		motor_rpm = clampf(motor_rpm + rpm_accel * delta, IDLE_RPM, MAX_RPM + 500.0)
	
	# Drive Torques per wheel
	var total_drive_torque = engine_torque * effective_ratio * clutch_engagement
	var drive_torques: Array[float] = [0.0, 0.0, total_drive_torque * 0.5, total_drive_torque * 0.5]
	var brake_torques: Array[float] = [
		input_brake * 2800.0 * 0.57 * 0.5,
		input_brake * 2800.0 * 0.57 * 0.5,
		input_brake * 2800.0 * 0.43 * 0.5 + input_handbrake * 1500.0 * 0.5,
		input_brake * 2800.0 * 0.43 * 0.5 + input_handbrake * 1500.0 * 0.5,
	]
	
	# 6. Tire Forces & Wheel Spin Integration
	var total_force_world := Vector3(0, -9.80665 * VEHICLE_MASS, 0) # Gravity
	var total_torque_world := Vector3.ZERO
	
	for w in range(4):
		var is_front = (w < 2)
		var tire_rad = FRONT_TIRE_RADIUS if is_front else REAR_TIRE_RADIUS
		var tire_w = FRONT_TIRE_WIDTH if is_front else REAR_TIRE_WIDTH
		var norm_f = normal_forces[w]
		
		# Wheel orientation
		var wheel_yaw = steer_angle_rad if is_front else 0.0
		var wheel_basis = basis_car.rotated(up_car, wheel_yaw)
		var wheel_fwd = -wheel_basis.z
		var wheel_right = wheel_basis.x
		
		var v_wheel_lin = linear_velocity.dot(wheel_fwd)
		var v_wheel_lat = linear_velocity.dot(wheel_right)
		
		# Wheel spin dynamics
		if w >= 2: # Driven rear wheels
			var gear_r = effective_ratio
			var ref_inertia = 0.08 * (gear_r * gear_r) * 0.5
			var tot_inertia = 16.0 * (tire_rad * tire_rad) * 0.5 + ref_inertia
			var net_trq = drive_torques[w]
			if absf(_wheel_spins[w]) > 0.01:
				net_trq -= brake_torques[w] * signf(_wheel_spins[w])
			var target_spin = v_wheel_lin / tire_rad
			var trq_acc = (net_trq / tot_inertia) * delta
			if samples_contact[w]:
				var grip_damping = (norm_f * 2.9 * tire_rad / tot_inertia) * delta
				_wheel_spins[w] = move_toward(_wheel_spins[w] + trq_acc, target_spin, grip_damping)
			else:
				_wheel_spins[w] += trq_acc
		else: # Free rolling front wheels
			if brake_torques[w] > 10.0:
				var brk_acc = (brake_torques[w] / 1.2) * signf(_wheel_spins[w])
				_wheel_spins[w] = move_toward(_wheel_spins[w], 0.0, brk_acc * delta)
			else:
				_wheel_spins[w] = v_wheel_lin / tire_rad
		
		_wheel_angles[w] += _wheel_spins[w] * delta
		
		# Forces
		if samples_contact[w]:
			var f_max = norm_f * 2.9
			var v_spin = _wheel_spins[w] * tire_rad
			var slip_ratio = clampf((v_spin - v_wheel_lin) / maxf(absf(v_wheel_lin), 1.0), -2.0, 2.0)
			var slip_angle = atan2(v_wheel_lat, maxf(absf(v_wheel_lin), 0.5))
			
			var sigma_x = 45100000.0 * 0.04 * slip_ratio
			var sigma_y = -41000000.0 * 0.04 * tan(slip_angle)
			var sigma_comb = sqrt(sigma_x * sigma_x + sigma_y * sigma_y)
			
			var crit = 0.5 * f_max * maxf(1.0 - absf(slip_ratio), 0.1)
			var force_comb = minf(sigma_comb, f_max)
			if sigma_comb > crit:
				var brush = maxf(1.0 - crit / (3.0 * sigma_comb), 0.0)
				force_comb = f_max * brush
			
			var fx = force_comb * (sigma_x / maxf(sigma_comb, 1e-4))
			var fy = force_comb * (sigma_y / maxf(sigma_comb, 1e-4))
			
			if w >= 2 and brake_torques[w] <= 10.0:
				var drive_f = drive_torques[w] / tire_rad
				fx = clampf(drive_f, -f_max, f_max)
			
			var f_tire_world = wheel_fwd * fx + wheel_right * fy
			var f_susp_world = samples_normal[w] * norm_f
			total_force_world += f_tire_world + f_susp_world
	
	# Aerodynamics (Downforce & Drag)
	var v_sq = v_forward * v_forward
	var drag_force = 0.5 * 1.225 * 0.15 * 0.45 * v_sq * (-signf(v_forward))
	var downforce = 0.5 * 1.225 * 1.85 * 0.45 * v_sq
	total_force_world += forward_car * drag_force - up_car * downforce
	
	# 7. RigidBody Motion Integration
	var lin_acc = total_force_world / VEHICLE_MASS
	linear_velocity += lin_acc * delta
	global_position += linear_velocity * delta
	
	# Angular damping & yaw alignment
	var yaw_torque = (steer_angle_rad * absf(v_forward) * 450.0) - (angular_velocity.y * 350.0)
	angular_velocity.y += (yaw_torque / 450.0) * delta
	angular_velocity.x = move_toward(angular_velocity.x, 0.0, 10.0 * delta)
	angular_velocity.z = move_toward(angular_velocity.z, 0.0, 10.0 * delta)
	global_rotation.y += angular_velocity.y * delta
	
	_current_linear_accel = (linear_velocity - _prev_linear_velocity) / delta
	_prev_linear_velocity = linear_velocity
	
	# 8. Animate Visual Wheels
	_update_visual_wheels(steer_angle_rad)
	
	# 9. Sync Telemetry
	_sync_telemetry()

func _update_visual_wheels(steer_angle: float) -> void:
	var z_f = -(1.0 - FRONT_WEIGHT_DIST) * WHEELBASE
	var z_r = FRONT_WEIGHT_DIST * WHEELBASE
	var x_f = FRONT_TRACK * 0.5
	var x_r = REAR_TRACK * 0.5
	
	var base_positions = [
		Vector3(-x_f, 0.144, z_f),
		Vector3(x_f, 0.144, z_f),
		Vector3(-x_r, 0.186, z_r),
		Vector3(x_r, 0.186, z_r),
	]
	
	var nodes = [front_left_wheel_node, front_right_wheel_node, rear_left_wheel_node, rear_right_wheel_node]
	
	for w in range(4):
		var node = nodes[w]
		if not node:
			continue
		
		var is_front = (w < 2)
		var spring_len = FRONT_SPRING_LENGTH if is_front else REAR_SPRING_LENGTH
		var comp = _wheel_compressions[w] / 1000.0
		var pos_y = base_positions[w].y - (spring_len - comp)
		node.position = Vector3(base_positions[w].x, pos_y, base_positions[w].z)
		
		# Rotation: Steer (Y) -> Camber (Z) -> Spin (X)
		var rot = Vector3.ZERO
		if is_front:
			rot.y = steer_angle
		rot.x = _wheel_angles[w]
		rot.z = -0.0174533 if (w % 2 == 0) else 0.0174533 # Static camber
		node.rotation = rot

func _sync_telemetry() -> void:
	if _telemetry_manager and _telemetry_manager.has_method("record_frame"):
		var long_g = _current_linear_accel.dot(-global_transform.basis.z) / 9.80665
		var lat_g = _current_linear_accel.dot(global_transform.basis.x) / 9.80665
		var frame_data = {
			"Time_ms": int(_sim_time * 1000.0),
			"Speed_kmh": speed,
			"RPM": motor_rpm,
			"Gear": current_gear,
			"Throttle": throttle_amount,
			"Brake": Input.get_action_raw_strength("brake") if enable_player_input else 0.0,
			"Steering": _steer_smoothed,
			"Lat_G": lat_g,
			"Long_G": long_g,
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
