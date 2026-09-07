class_name F1WheelVisualController
extends Node

## Visual wheel controller for the F1-2026-2008.
##
## The physics extension remains the only suspension authority. This node only
## consumes compression/telemetry and drives the visual hierarchy:
## Hub -> SteerPivot -> CamberPivot -> Spinner -> Visual.
##
## The native wheel visual paths are intentionally empty in the scene. That
## prevents two systems from writing the same transform and keeps this script
## as the single visual owner.

@export var vehicle: Node3D
@export var physics_config_path: String = "res://data/vehicles/f1_2026_2008/f1_2026_2008_physics.json"

@export var hub_fl: Node3D
@export var hub_fr: Node3D
@export var hub_rl: Node3D
@export var hub_rr: Node3D

@export var steer_fl: Node3D
@export var steer_fr: Node3D
@export var steer_rl: Node3D
@export var steer_rr: Node3D

@export var camber_fl: Node3D
@export var camber_fr: Node3D
@export var camber_rl: Node3D
@export var camber_rr: Node3D

@export var spinner_fl: Node3D
@export var spinner_fr: Node3D
@export var spinner_rl: Node3D
@export var spinner_rr: Node3D

@export_group("Tire smoke")
## Normalized longitudinal scrub severity, not throttle/brake pedal travel.
@export_range(0.05, 0.95, 0.01) var smoke_severity_on: float = 0.60
@export_range(0.0, 0.90, 0.01) var smoke_severity_off: float = 0.45

var front_spring_length: float = 0.295
var front_resting_ratio: float = 0.175
var front_camber_base: float = -0.0483972
var front_camber_gain: float = -0.02
var front_tire_radius: float = 0.33
var front_toe: float = -0.0018

var rear_spring_length: float = 0.265
var rear_resting_ratio: float = 0.26
var rear_camber_base: float = -0.029
var rear_camber_gain: float = -0.02
var rear_tire_radius: float = 0.33
var rear_toe: float = 0.0042

var max_steering_angle: float = 0.4363
var caster_angle_rad: float = deg_to_rad(6.5)

var _hubs: Array = []
var _steer_pivots: Array = []
var _camber_pivots: Array = []
var _spinners: Array = []
var _base_anchors: Array = [
	Vector3(-0.854, 0.243375, -1.75),
	Vector3(0.854, 0.243375, -1.75),
	Vector3(-0.795, 0.1961, 1.75),
	Vector3(0.795, 0.1961, 1.75)
]

var _wheel_angles: Array = [0.0, 0.0, 0.0, 0.0]
var _spin_rates: Array = [0.0, 0.0, 0.0, 0.0]
var _compression_m: Array = [0.0, 0.0, 0.0, 0.0]
var _previous_compression_m: Array = [0.0, 0.0, 0.0, 0.0]
var _surface_types: Array = [0, 0, 0, 0]
var _previous_surface_types: Array = [0, 0, 0, 0]
var _normal_forces: Array = [0.0, 0.0, 0.0, 0.0]
var _valid_spin_samples: Array = [false, false, false, false]
var _kerb_energy: Array = [0.0, 0.0, 0.0, 0.0]
var _smoke_intensity: Array = [0.0, 0.0, 0.0, 0.0]
var _smoke_severity: Array = [0.0, 0.0, 0.0, 0.0]
var _smoke_active: Array = [false, false, false, false]

var _target_positions: Array = [Vector3.ZERO, Vector3.ZERO, Vector3.ZERO, Vector3.ZERO]
var _target_steer: Array = [0.0, 0.0, 0.0, 0.0]
var _target_camber: Array = [0.0, 0.0, 0.0, 0.0]
var _smoke_emitters: Array = []
var _time_accum: float = 0.0
var _visual_initialized: bool = false
var _has_physics_sample: bool = false

const WHEEL_KEYS := ["FL", "FR", "RL", "RR"]
const KERB_SURFACE_CODE := 1
const MAX_CAMBER_VISUAL_RAD := 0.0872665
# Keep these curves/envelope aligned with vehicle-audio-engine/src/tire_scrub.rs
# and mixer.rs. Audio starts softly below the separate strong-smoke threshold.
const SCRUB_SPIN_SLIP := Vector2(0.08, 0.35)
const SCRUB_LOCK_SLIP := Vector2(0.12, 0.70)
const SCRUB_ATTACK_SECONDS := 0.025
const SCRUB_RELEASE_SECONDS := 0.160

func _ready() -> void:
	if vehicle == null:
		vehicle = get_parent() as Node3D
	_load_physics_specs()
	_resolve_nodes()
	_init_smoke_emitters()
	if vehicle != null:
		# Children become ready before F194RustVehicle initializes its solver.
		# Keep the scene layout until the parent can supply real anchors.
		if vehicle.is_node_ready():
			_refresh_physics_anchors()
		else:
			vehicle.ready.connect(_refresh_physics_anchors, CONNECT_ONE_SHOT)

func _load_physics_specs() -> void:
	if physics_config_path.is_empty() or not FileAccess.file_exists(physics_config_path):
		return
	var file := FileAccess.open(physics_config_path, FileAccess.READ)
	if file == null:
		return
	var json_data: Variant = JSON.parse_string(file.get_as_text())
	if not (json_data is Dictionary):
		return

	var suspension: Variant = json_data.get("suspension", {})
	if suspension is Dictionary:
		var front: Variant = suspension.get("front", {})
		if front is Dictionary:
			front_spring_length = float(front.get("spring_length", front_spring_length))
			front_resting_ratio = float(front.get("resting_ratio", front_resting_ratio))
			front_camber_base = float(front.get("camber", front_camber_base))
			front_camber_gain = float(front.get("camber_gain_rad_per_m", front_camber_gain))
			front_toe = float(front.get("toe", front_toe))
		var rear: Variant = suspension.get("rear", {})
		if rear is Dictionary:
			rear_spring_length = float(rear.get("spring_length", rear_spring_length))
			rear_resting_ratio = float(rear.get("resting_ratio", rear_resting_ratio))
			rear_camber_base = float(rear.get("camber", rear_camber_base))
			rear_camber_gain = float(rear.get("camber_gain_rad_per_m", rear_camber_gain))
			rear_toe = float(rear.get("toe", rear_toe))

	var steering: Variant = json_data.get("steering", {})
	if steering is Dictionary:
		max_steering_angle = float(steering.get("max_steering_angle", max_steering_angle))

	var tires: Variant = json_data.get("tires", {})
	if tires is Dictionary:
		var front_tire: Variant = tires.get("front", {})
		if front_tire is Dictionary:
			front_tire_radius = float(front_tire.get("radius", front_tire_radius))
		var rear_tire: Variant = tires.get("rear", {})
		if rear_tire is Dictionary:
			rear_tire_radius = float(rear_tire.get("radius", rear_tire_radius))

func _resolve_nodes() -> void:
	if vehicle == null:
		return
	_hubs = [
		hub_fl if hub_fl != null else vehicle.get_node_or_null("FrontLeftWheel"),
		hub_fr if hub_fr != null else vehicle.get_node_or_null("FrontRightWheel"),
		hub_rl if hub_rl != null else vehicle.get_node_or_null("RearLeftWheel"),
		hub_rr if hub_rr != null else vehicle.get_node_or_null("RearRightWheel")
	]
	_steer_pivots = [
		steer_fl if steer_fl != null else _node_from(_hubs[0], "SteerPivot"),
		steer_fr if steer_fr != null else _node_from(_hubs[1], "SteerPivot"),
		steer_rl if steer_rl != null else _node_from(_hubs[2], "SteerPivot"),
		steer_rr if steer_rr != null else _node_from(_hubs[3], "SteerPivot")
	]
	_camber_pivots = [
		camber_fl if camber_fl != null else _node_from(_steer_pivots[0], "CamberPivot"),
		camber_fr if camber_fr != null else _node_from(_steer_pivots[1], "CamberPivot"),
		camber_rl if camber_rl != null else _node_from(_steer_pivots[2], "CamberPivot"),
		camber_rr if camber_rr != null else _node_from(_steer_pivots[3], "CamberPivot")
	]
	_spinners = [
		spinner_fl if spinner_fl != null else _node_from(_camber_pivots[0], "Spinner"),
		spinner_fr if spinner_fr != null else _node_from(_camber_pivots[1], "Spinner"),
		spinner_rl if spinner_rl != null else _node_from(_camber_pivots[2], "Spinner"),
		spinner_rr if spinner_rr != null else _node_from(_camber_pivots[3], "Spinner")
	]

	for wheel_index in range(4):
		var hub: Node3D = _hubs[wheel_index] as Node3D
		if hub == null:
			continue
		# A hub is authored at static ride height, not at the spring anchor.
		var spring_length := _spring_length(wheel_index)
		var rest_compression := spring_length * _resting_ratio(wheel_index)
		_base_anchors[wheel_index] = hub.position + Vector3.UP * (spring_length - rest_compression)

func _refresh_physics_anchors() -> void:
	if not vehicle.has_method(&"get_wheel_anchor_local"):
		return
	for wheel_index in range(4):
		var queried_anchor: Variant = vehicle.call(&"get_wheel_anchor_local", wheel_index)
		if queried_anchor is Vector3 and _finite_vector(queried_anchor):
			# Zero is also the native fallback for an unavailable solver. It is
			# never a wheel anchor on this car; accepting it hides all four meshes
			# inside the chassis at x=z=0.
			if not queried_anchor.is_zero_approx():
				_base_anchors[wheel_index] = queried_anchor

func _node_from(parent_node: Variant, child_name: String) -> Node3D:
	if parent_node is Node:
		return (parent_node as Node).get_node_or_null(child_name) as Node3D
	return null

func _init_smoke_emitters() -> void:
	_smoke_emitters.clear()
	for wheel_index in range(4):
		var hub: Node3D = _hubs[wheel_index] as Node3D
		_smoke_emitters.append(_create_smoke_emitter(hub, wheel_index) if hub != null else null)

func _create_smoke_emitter(parent_node: Node3D, wheel_index: int) -> CPUParticles3D:
	var particles := CPUParticles3D.new()
	particles.name = "TireSmokeEmitter"
	particles.emitting = false
	particles.amount = 72
	particles.lifetime = 0.65
	particles.explosiveness = 0.0
	particles.randomness = 0.3
	particles.local_coords = false
	particles.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	particles.position = Vector3(0.0, -_tire_radius(wheel_index) + 0.04, 0.0)
	particles.direction = Vector3(0.0, 0.35, 1.0)
	particles.spread = 22.0
	particles.gravity = Vector3(0.0, 1.5, 0.0)
	particles.initial_velocity_min = 1.2
	particles.initial_velocity_max = 3.8
	particles.linear_accel_min = -0.5
	particles.linear_accel_max = 0.0
	particles.scale_amount_min = 0.25
	particles.scale_amount_max = 0.85

	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.billboard_mode = BaseMaterial3D.BILLBOARD_PARTICLES
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	material.vertex_color_use_as_albedo = true
	material.albedo_color = Color(0.95, 0.95, 0.95, 0.4)
	# A soft radial sprite avoids visible square billboards in the plume.
	var sprite_gradient := Gradient.new()
	sprite_gradient.offsets = PackedFloat32Array([0.0, 0.45, 1.0])
	sprite_gradient.colors = PackedColorArray([Color.WHITE, Color(1, 1, 1, 0.65), Color(1, 1, 1, 0)])
	var sprite_texture := GradientTexture2D.new()
	sprite_texture.width = 64
	sprite_texture.height = 64
	sprite_texture.fill = GradientTexture2D.FILL_RADIAL
	sprite_texture.fill_from = Vector2(0.5, 0.5)
	sprite_texture.fill_to = Vector2(1.0, 0.5)
	sprite_texture.gradient = sprite_gradient
	material.albedo_texture = sprite_texture
	var quad := QuadMesh.new()
	quad.material = material
	quad.size = Vector2(0.45, 0.45)
	particles.mesh = quad

	var gradient := Gradient.new()
	# Set both endpoints explicitly; inserting a point changes its array index.
	gradient.offsets = PackedFloat32Array([0.0, 0.12, 0.55, 1.0])
	gradient.colors = PackedColorArray([
		Color(0.95, 0.95, 0.95, 0.0), Color(0.95, 0.95, 0.95, 0.85),
		Color(0.92, 0.92, 0.92, 0.55), Color(0.88, 0.88, 0.88, 0.0)
	])
	particles.color_ramp = gradient
	parent_node.add_child(particles)
	return particles

func _physics_process(delta: float) -> void:
	if vehicle == null:
		return
	_time_accum += delta

	var compressions: Variant = vehicle.call(&"get_wheel_compressions") if vehicle.has_method(&"get_wheel_compressions") else []
	var surfaces: Variant = vehicle.call(&"get_wheel_surface_types") if vehicle.has_method(&"get_wheel_surface_types") else []
	var normal_forces: Variant = vehicle.call(&"get_normal_forces") if vehicle.has_method(&"get_normal_forces") else []
	var brake_state: Dictionary = _read_brake_snapshot()
	var signed_speed := _signed_forward_speed()
	var speed_ms := absf(signed_speed)
	var steer_rad := _steer_angle_rad()
	var local_velocity := _local_body_velocity(&"get_linear_velocity", Vector3(0.0, 0.0, -signed_speed))
	var local_angular_velocity := _local_body_velocity(&"get_angular_velocity", Vector3.ZERO)

	for wheel_index in range(4):
		var is_front := wheel_index < 2
		# Both native telemetry routes expose compression in millimeters.
		var rest_compression_mm := _spring_length(wheel_index) * _resting_ratio(wheel_index) * 1000.0
		var compression := _array_float(compressions, wheel_index, rest_compression_mm) * 0.001
		compression = clampf(compression, 0.0, _spring_length(wheel_index))
		_compression_m[wheel_index] = compression
		_surface_types[wheel_index] = _array_int(surfaces, wheel_index, 0)
		_normal_forces[wheel_index] = _array_float(normal_forces, wheel_index, 0.0)

		var wheel_key: String = WHEEL_KEYS[wheel_index]
		var radius := _tire_radius(wheel_index)
		var spin_rate := signed_speed / radius
		_valid_spin_samples[wheel_index] = false
		if brake_state.has(wheel_key) and brake_state[wheel_key] is Dictionary:
			var wheel_state: Dictionary = brake_state[wheel_key]
			if wheel_state.has("spin_post_rad_s"):
				var reported_spin := float(wheel_state["spin_post_rad_s"])
				if is_finite(reported_spin):
					spin_rate = reported_spin
					_valid_spin_samples[wheel_index] = true
		_spin_rates[wheel_index] = spin_rate
		_wheel_angles[wheel_index] += spin_rate * delta

		var on_kerb: bool = _surface_types[wheel_index] == KERB_SURFACE_CODE
		var entered_kerb: bool = on_kerb and _previous_surface_types[wheel_index] != KERB_SURFACE_CODE
		var compression_velocity := absf(compression - _previous_compression_m[wheel_index]) / maxf(delta, 0.0001)
		if entered_kerb:
			_kerb_energy[wheel_index] = 1.0
		elif on_kerb:
			_kerb_energy[wheel_index] = maxf(_kerb_energy[wheel_index] - delta * 1.4, 0.0)
		else:
			_kerb_energy[wheel_index] = move_toward(_kerb_energy[wheel_index], 0.0, delta * 8.0)
		if on_kerb:
			_kerb_energy[wheel_index] = maxf(_kerb_energy[wheel_index], clampf(compression_velocity * 0.035, 0.0, 0.75))

		# Deterministic chatter: the phase is per-wheel, so replayed physics has
		# the same motion and no frame-dependent randf() noise is introduced.
		var phase := _time_accum * (28.0 + speed_ms * 1.5) + float(wheel_index) * 1.73
		var chatter_gain: float = _kerb_energy[wheel_index] * clampf(speed_ms / 18.0, 0.15, 1.0)
		var vertical_chatter: float = sin(phase * TAU) * 0.0045 * chatter_gain
		var lateral_chatter: float = cos(phase * 0.83 * TAU) * 0.0022 * chatter_gain
		if wheel_index % 2 == 1:
			lateral_chatter = -lateral_chatter

		var anchor: Vector3 = _base_anchors[wheel_index]
		_target_positions[wheel_index] = Vector3(
			anchor.x + lateral_chatter,
			anchor.y - _spring_length(wheel_index) + compression + vertical_chatter,
			anchor.z
		)

		var resting_compression := _spring_length(wheel_index) * _resting_ratio(wheel_index)
		var delta_travel := compression - resting_compression
		var dynamic_camber := _camber_base(wheel_index) + delta_travel * _camber_gain(wheel_index)
		dynamic_camber = clampf(dynamic_camber, -MAX_CAMBER_VISUAL_RAD, MAX_CAMBER_VISUAL_RAD)
		var caster_tilt := -sin(steer_rad) * sin(caster_angle_rad) if is_front else 0.0
		_target_camber[wheel_index] = dynamic_camber + caster_tilt
		_target_steer[wheel_index] = (steer_rad if is_front else 0.0) + _toe(wheel_index) * (1.0 if wheel_index % 2 == 0 else -1.0)

		# Use each wheel's rolling direction/point speed, not the visual spinner
		# angle or chassis speed alone (inside/outside wheels differ in a turn).
		var contact_offset: Vector3 = _target_positions[wheel_index] - Vector3.UP * radius
		var point_velocity := local_velocity + local_angular_velocity.cross(contact_offset)
		var wheel_forward := Vector3.FORWARD.rotated(Vector3.UP, _target_steer[wheel_index])
		# F90Core publishes the authoritative scalar speed while the native vehicle's
		# cached Godot velocity can lag one bridge tick behind. Use the larger value for
		# the visual gate so a valid lock/spin is not hidden by that stale vector.
		_update_smoke(wheel_index, point_velocity.dot(wheel_forward), maxf(local_velocity.length(), speed_ms), delta)
		_previous_compression_m[wheel_index] = compression
		_previous_surface_types[wheel_index] = _surface_types[wheel_index]
	_has_physics_sample = true

func _process(delta: float) -> void:
	if not _has_physics_sample:
		return
	var blend := 1.0 if not _visual_initialized else (1.0 - exp(-delta * 32.0))
	for wheel_index in range(4):
		var hub: Node3D = _hubs[wheel_index] as Node3D
		if hub == null:
			continue
		hub.position = hub.position.lerp(_target_positions[wheel_index], blend)
		var steer_pivot: Node3D = _steer_pivots[wheel_index] as Node3D
		if steer_pivot != null:
			var steer_pitch: float = _target_steer[wheel_index] * sin(caster_angle_rad) * 0.5 if wheel_index < 2 else 0.0
			steer_pivot.rotation = steer_pivot.rotation.lerp(Vector3(steer_pitch, _target_steer[wheel_index], 0.0), blend)
		var camber_pivot: Node3D = _camber_pivots[wheel_index] as Node3D
		if camber_pivot != null:
			camber_pivot.rotation.z = lerpf(camber_pivot.rotation.z, _target_camber[wheel_index], blend)
		var spinner: Node3D = _spinners[wheel_index] as Node3D
		if spinner != null:
			spinner.rotation.x = -_wheel_angles[wheel_index]
	_visual_initialized = true

func _update_smoke(wheel_index: int, forward_speed: float, speed_ms: float, delta: float) -> void:
	var emitter: CPUParticles3D = _smoke_emitters[wheel_index] as CPUParticles3D
	if emitter == null:
		return
	# get_wheel_slips() is legacy lateral/axle telemetry, NOT signed kappa;
	# tc_slip_ratio excludes non-driven wheels. Neither can detect front locks.
	# Reconstruct tire.rs's geometric slip from the exposed spin and velocity.
	# This is a proxy: effective rolling radius/contact-plane telemetry and the
	# mixer's final gain are not currently exposed through the GDScript bridge.
	var wheel_speed: float = _spin_rates[wheel_index] * _tire_radius(wheel_index)
	var denominator := maxf(maxf(absf(forward_speed), absf(wheel_speed)), 1.0)
	var slip_ratio := (wheel_speed - forward_speed) / denominator
	var wheelspin := smoothstep(SCRUB_SPIN_SLIP.x, SCRUB_SPIN_SLIP.y, slip_ratio)
	var lockup := smoothstep(SCRUB_LOCK_SLIP.x, SCRUB_LOCK_SLIP.y, -slip_ratio)
	var speed_gate := smoothstep(5.0, 20.0, speed_ms * 3.6)
	var load_gain := smoothstep(150.0, 3500.0, maxf(_normal_forces[wheel_index], 0.0))
	var has_contact := _has_wheel_contact(wheel_index)
	# F90Core does not mirror per-wheel normal load on every bridge path. Contact
	# remains authoritative for airborne rejection; when load is unavailable, keep a
	# modest visual floor so strong slip still produces visible smoke.
	var visual_load_gain := maxf(load_gain, 0.35) if has_contact else 0.0
	var eligible: bool = _valid_spin_samples[wheel_index] and has_contact and speed_gate > 0.0
	var target_severity := maxf(wheelspin, lockup) * speed_gate if eligible else 0.0
	var response_time := SCRUB_ATTACK_SECONDS if target_severity > _smoke_severity[wheel_index] else SCRUB_RELEASE_SECONDS
	_smoke_severity[wheel_index] = lerpf(_smoke_severity[wheel_index], target_severity, 1.0 - exp(-maxf(delta, 0.0) / response_time))
	var threshold_on := clampf(smoke_severity_on, 0.05, 0.95)
	var threshold_off := clampf(smoke_severity_off, 0.0, threshold_on - 0.01)
	if not eligible:
		_smoke_active[wheel_index] = false
		_smoke_severity[wheel_index] = 0.0
	elif _smoke_active[wheel_index]:
		_smoke_active[wheel_index] = _smoke_severity[wheel_index] > threshold_off
	else:
		_smoke_active[wheel_index] = _smoke_severity[wheel_index] >= threshold_on
	# Load/surface weight opacity as in scrub audio; they must not turn light
	# slip into smoke simply because aero load increases at high speed.
	var surface_gain := 1.0
	match int(_surface_types[wheel_index]):
		1: surface_gain = 0.70
		3, 5: surface_gain = 0.22
		0: surface_gain = 1.0
		_: surface_gain = 0.50
	_smoke_intensity[wheel_index] = _smoke_severity[wheel_index] * visual_load_gain * surface_gain
	# CPUParticles3D.set_amount() deactivates the whole particle pool, even
	# when the value is unchanged. Keep it fixed and modulate opacity instead.
	emitter.color = Color(1.0, 1.0, 1.0, _smoke_intensity[wheel_index])
	emitter.emitting = _smoke_active[wheel_index] and _smoke_intensity[wheel_index] > 0.03

func _local_body_velocity(method: StringName, fallback: Vector3) -> Vector3:
	if vehicle.has_method(method):
		var value: Variant = vehicle.call(method)
		if value is Vector3 and _finite_vector(value):
			var local_value: Vector3 = vehicle.global_transform.basis.orthonormalized().transposed() * value
			if method == &"get_linear_velocity":
				# In the F90Core bridge the scalar speed is current, but the vehicle's
				# cached linear_velocity may still be near zero. Replace only its stale
				# longitudinal component and preserve lateral/vertical motion.
				var expected_forward := absf(fallback.z)
				if expected_forward > 0.5 and absf(local_value.z) < maxf(expected_forward * 0.35, 0.25):
					local_value.z = fallback.z
			return local_value
	return fallback

func _read_brake_snapshot() -> Dictionary:
	if not vehicle.has_method(&"get_brake_state_snapshot"):
		return {}
	var snapshot: Variant = vehicle.call(&"get_brake_state_snapshot")
	return snapshot if snapshot is Dictionary else {}

func _signed_forward_speed() -> float:
	var body_signed := 0.0
	if vehicle.has_method(&"get_linear_velocity"):
		var velocity: Variant = vehicle.call(&"get_linear_velocity")
		if velocity is Vector3 and _finite_vector(velocity):
			body_signed = -vehicle.global_transform.basis.z.dot(velocity)
	if vehicle.has_method(&"get_speed_kmh"):
		var reported_kmh := float(vehicle.call(&"get_speed_kmh"))
		if is_finite(reported_kmh):
			var reported_ms := absf(reported_kmh) / 3.6
			var direction := -1.0 if body_signed < -0.25 else 1.0
			if reported_ms > 0.05 or absf(body_signed) < 0.05:
				return reported_ms * direction
	if vehicle.has_method(&"get_linear_velocity"):
		var velocity: Variant = vehicle.call(&"get_linear_velocity")
		if velocity is Vector3 and _finite_vector(velocity):
			return body_signed
	var speed_value: Variant = vehicle.get("speed")
	return float(speed_value) if speed_value != null and is_finite(float(speed_value)) else 0.0

func _has_wheel_contact(wheel_index: int) -> bool:
	var ray_prefixes := ["FL", "FR", "RL", "RR"]
	var ray_found := false
	for suffix in ["_In", "_Mid", "_Out"]:
		var ray := vehicle.get_node_or_null("RayCast_%s%s" % [ray_prefixes[wheel_index], suffix]) as RayCast3D
		if ray == null:
			continue
		ray_found = true
		if ray.is_colliding():
			return true
	if ray_found:
		return false
	# Test fixtures and non-raycast vehicles still have enough information in
	# suspension compression to distinguish contact from an airborne wheel.
	return _compression_m[wheel_index] > 0.001 or _normal_forces[wheel_index] > 1.0

func _steer_angle_rad() -> float:
	if vehicle.has_method(&"get_steer_angle_rad"):
		var direct_angle := float(vehicle.call(&"get_steer_angle_rad"))
		if is_finite(direct_angle):
			return direct_angle
	var amount := _property_float("true_steering_amount")
	return amount * max_steering_angle

func _property_float(property_name: String) -> float:
	var value: Variant = vehicle.get(property_name)
	return float(value) if value != null and is_finite(float(value)) else 0.0

func _array_float(values: Variant, index: int, fallback: float) -> float:
	if values is Array or values is PackedFloat32Array or values is PackedFloat64Array:
		if values.size() > index and is_finite(float(values[index])):
			return float(values[index])
	return fallback

func _array_int(values: Variant, index: int, fallback: int) -> int:
	if values is Array or values is PackedInt32Array or values is PackedInt64Array:
		if values.size() > index:
			return int(values[index])
	return fallback

func _finite_vector(value: Vector3) -> bool:
	return is_finite(value.x) and is_finite(value.y) and is_finite(value.z)

func _spring_length(wheel_index: int) -> float:
	return front_spring_length if wheel_index < 2 else rear_spring_length

func _resting_ratio(wheel_index: int) -> float:
	return front_resting_ratio if wheel_index < 2 else rear_resting_ratio

func _camber_base(wheel_index: int) -> float:
	var base := front_camber_base if wheel_index < 2 else rear_camber_base
	return base if wheel_index % 2 == 0 else -base

func _camber_gain(wheel_index: int) -> float:
	var gain := front_camber_gain if wheel_index < 2 else rear_camber_gain
	return gain if wheel_index % 2 == 0 else -gain

func _tire_radius(wheel_index: int) -> float:
	return front_tire_radius if wheel_index < 2 else rear_tire_radius

func _toe(wheel_index: int) -> float:
	return front_toe if wheel_index < 2 else rear_toe
