extends SceneTree

## Render regression, not just an emitting=true check. Run with a graphics
## renderer in an empty Godot project (no game DLLs) and --source-root=<game>.
## Optional --controller=<backup.gd> replays the pre-fix implementation.
const WHEELS := ["FrontLeftWheel", "FrontRightWheel", "RearLeftWheel", "RearRightWheel"]
const CENTERS := [Vector3(-0.854, 0, -1.75), Vector3(0.854, 0, -1.75), Vector3(-0.795, 0, 1.75), Vector3(0.795, 0, 1.75)]

class TelemetryVehicle extends Node3D:
	var initialized := false
	var throttle_amount := 0.0
	var brake_amount := 0.0
	var locked := false
	var hard_lock := false
	var mild_spin := false
	var burnout := false
	var grounded := true
	var stale_linear_velocity := false
	var expose_normal_forces := true
	var speed_ms := 20.0
	func _ready() -> void:
		initialized = true
	func get_wheel_anchor_local(index: int) -> Vector3:
		return CENTERS[index] + Vector3.UP * (0.243375 if index < 2 else 0.1961) if initialized else Vector3.ZERO
	func get_wheel_compressions() -> PackedFloat64Array:
		return PackedFloat64Array([51.625, 51.625, 68.9, 68.9]) if grounded else PackedFloat64Array([0, 0, 0, 0])
	func get_normal_forces() -> PackedFloat64Array:
		return PackedFloat64Array([1500, 1500, 1500, 1500]) if grounded and expose_normal_forces else PackedFloat64Array([0, 0, 0, 0])
	func get_drive_torques() -> PackedFloat64Array:
		return PackedFloat64Array([0, 0, 500 * throttle_amount, 500 * throttle_amount])
	func get_linear_velocity() -> Vector3:
		return Vector3.ZERO if stale_linear_velocity else Vector3(0, 0, -speed_ms)
	func get_speed_kmh() -> float:
		return speed_ms * 3.6
	func get_steer_angle_rad() -> float:
		return 0.0
	func get_brake_state_snapshot() -> Dictionary:
		var result := {}
		for index in range(4):
			var spin := speed_ms / 0.33
			if locked and index < 2:
				spin = 0.0
			elif hard_lock and index < 2:
				# -0.60 longitudinal slip: strong enough for scrub audio/smoke,
				# but deliberately not a complete wheel lock.
				spin = speed_ms * 0.40 / 0.33
			elif burnout and index >= 2:
				spin = speed_ms * 1.75 / 0.33
			elif mild_spin and index >= 2:
				# +0.20 slip is inside the audible ramp but below strong smoke.
				spin = speed_ms * 1.20 / 0.33
			result[["FL", "FR", "RL", "RR"][index]] = {"spin_post_rad_s": spin, "brake_torque_nm": brake_amount * 1000.0}
		return result

var _failures: Array[String] = []

func _init() -> void:
	call_deferred("_run")

func _argument(prefix: String, fallback: String = "") -> String:
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with(prefix):
			return argument.trim_prefix(prefix)
	return fallback

func _check(condition: bool, message: String) -> void:
	if not condition:
		_failures.append(message)
		printerr("[FAIL] " + message)

func _bright_pixels() -> int:
	var frame := root.get_texture().get_image()
	var count := 0
	for y in range(frame.get_height()):
		for x in range(frame.get_width()):
			if frame.get_pixel(x, y).r > 0.10:
				count += 1
	return count

func _settle() -> void:
	await create_timer(1.2).timeout
	await RenderingServer.frame_post_draw

func _run() -> void:
	if DisplayServer.get_name() == "headless":
		printerr("[FAIL] This test needs a graphics renderer; do not use --headless.")
		quit(1)
		return
	Engine.physics_ticks_per_second = 120
	Engine.max_fps = 60
	root.size = Vector2i(640, 360)
	root.transparent_bg = false
	var world := Node3D.new()
	root.add_child(world)
	var environment_node := WorldEnvironment.new()
	var environment := Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color.BLACK
	environment_node.environment = environment
	world.add_child(environment_node)
	var camera := Camera3D.new()
	world.add_child(camera)
	camera.position = Vector3(3, 2, 5)
	camera.look_at(Vector3.ZERO)
	camera.current = true
	var source_root := _argument("--source-root=", ProjectSettings.globalize_path("res://"))
	var script_path := _argument("--controller=", source_root.path_join("scripts/vehicle/f1_wheel_visual_controller.gd"))
	var controller_script := load(script_path) as Script
	if controller_script == null or not controller_script.can_instantiate():
		quit(1)
		return
	var vehicle := TelemetryVehicle.new()
	var controller := Node.new()
	controller.set_script(controller_script)
	controller.set("physics_config_path", source_root.path_join("data/vehicles/f1_2026_2008/f1_2026_2008_physics.json"))
	vehicle.add_child(controller)
	for index in range(4):
		var hub := Node3D.new()
		hub.name = WHEELS[index]
		hub.position = CENTERS[index]
		vehicle.add_child(hub)
	world.add_child(vehicle)
	var emitters: Array[CPUParticles3D] = []
	for wheel in WHEELS:
		var emitter := vehicle.get_node(wheel + "/TireSmokeEmitter") as CPUParticles3D
		emitter.use_fixed_seed = true
		emitter.seed = 42
		emitters.append(emitter)
	var pool_size := emitters[0].amount
	# Mirrors the F90Core bridge: scalar speed is current while the cached native
	# linear velocity can lag, and normal load may be unavailable on that route.
	vehicle.stale_linear_velocity = true
	# Ordinary acceleration with torque, but without tire slip, must be clear.
	vehicle.throttle_amount = 0.8
	await _settle()
	_check(_bright_pixels() == 0, "Normal acceleration must not emit smoke")
	for emitter in emitters:
		_check(not emitter.emitting, "No emission without slip")
	vehicle.throttle_amount = 0.0
	vehicle.mild_spin = true
	await _settle()
	_check(_bright_pixels() == 0, "Mild wheelspin must stay below the smoke threshold")
	_check(not emitters[2].emitting and not emitters[3].emitting, "Mild rear wheelspin must not emit")
	vehicle.mild_spin = false
	vehicle.expose_normal_forces = false
	vehicle.brake_amount = 1.0
	vehicle.hard_lock = true
	await _settle()
	_check(_bright_pixels() > 100, "Strong partial lockup must render smoke")
	_check(emitters[0].emitting and emitters[1].emitting and not emitters[2].emitting and not emitters[3].emitting, "Braking smoke follows the locked front wheels")
	vehicle.speed_ms = 40.0
	await _settle()
	_check(_bright_pixels() > 100, "Strong lockup must remain visible above 90 km/h")
	vehicle.speed_ms = 20.0
	vehicle.hard_lock = false
	vehicle.brake_amount = 0.0
	await _settle()
	_check(_bright_pixels() == 0, "Smoke clears after releasing a partial lockup")
	vehicle.throttle_amount = 0.0
	vehicle.brake_amount = 1.0
	vehicle.locked = true
	await _settle()
	var lock_pixels := _bright_pixels()
	print("[RENDER] Locked front wheels: ", lock_pixels, " visible smoke pixels")
	_check(lock_pixels > 100, "Sustained lockup must render a visible plume")
	_check(emitters[0].emitting and emitters[1].emitting and not emitters[2].emitting and not emitters[3].emitting, "Only locked wheels emit")
	for emitter in emitters:
		_check(emitter.amount == pool_size, "Particle pool must stay fixed")
		_check(emitter.color_ramp.sample(1.0).a == 0.0, "Smoke fades out at end of lifetime")
		_check(emitter.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_OFF, "Smoke is not a solid shadow caster")
	vehicle.brake_amount = 0.0
	vehicle.locked = false
	await _settle()
	_check(_bright_pixels() == 0, "Smoke clears after releasing the brakes")
	vehicle.throttle_amount = 0.8
	vehicle.burnout = true
	await _settle()
	_check(_bright_pixels() > 100, "Rear wheelspin renders smoke")
	_check(not emitters[0].emitting and not emitters[1].emitting and emitters[2].emitting and emitters[3].emitting, "Only spinning rear wheels emit")
	vehicle.grounded = false
	await _settle()
	_check(_bright_pixels() == 0, "Airborne wheelspin must not emit smoke")
	print("[RESULT] Tire smoke render regression: %d failure(s)" % _failures.size())
	quit(0 if _failures.is_empty() else 1)
