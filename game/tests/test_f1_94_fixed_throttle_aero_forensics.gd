extends SceneTree

const SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"
const SETTLE_FRAMES := 180
const RUN_FRAMES := 900
const YAW_DECAY_EXPONENT := 1.30

func _init() -> void:
	call_deferred("_run")

func _run() -> void:
	var packed := load(SCENE_PATH) as PackedScene
	if packed == null:
		printerr("[FAIL] scene missing")
		quit(1)
		return
	var root_node = packed.instantiate()
	root.add_child(root_node)
	for _i in range(4):
		await process_frame
	var vehicle = root_node.find_child("VehicleRigidBody", true, false)
	if vehicle == null:
		printerr("[FAIL] VehicleRigidBody missing")
		quit(1)
		return
	vehicle.enable_player_input = false
	vehicle.throttle_amount = 0.0
	for _i in range(SETTLE_FRAMES):
		await physics_frame
	var file := FileAccess.open("user://f1_94_fixed_throttle_aero_forensics.csv", FileAccess.WRITE)
	file.store_line("frame,time_ms,forward_kmh,lateral_kmh,total_speed_kmh,rpm,gear,throttle,lat_g,long_g,yaw_rate_rad_s,rear_slip_max,rl_comp,rr_comp,beta_rad,f_yaw_est")
	var max_abs_lat: float = 0.0
	var max_abs_yaw_rate: float = 0.0
	var max_rear_slip: float = 0.0
	for frame in range(RUN_FRAMES):
		vehicle.throttle_amount = 1.0
		await physics_frame
		var basis: Basis = vehicle.global_transform.basis
		var linear_velocity: Vector3 = vehicle.linear_velocity
		var forward_velocity: float = -basis.z.dot(linear_velocity)
		var lateral_velocity: float = basis.x.dot(linear_velocity)
		var beta: float = atan2(lateral_velocity, maxf(absf(forward_velocity), 1e-6))
		var f_yaw: float = pow(clampf(cos(beta), 0.0, 1.0), YAW_DECAY_EXPONENT)
		var slips = vehicle.get_wheel_slips()
		var comp = vehicle.get_wheel_compressions()
		var rear_slip: float = maxf(absf(float(slips[2])), absf(float(slips[3])))
		var yaw_rate: float = vehicle.angular_velocity.y
		max_abs_lat = maxf(max_abs_lat, absf(vehicle.lat_g))
		max_abs_yaw_rate = maxf(max_abs_yaw_rate, absf(yaw_rate))
		max_rear_slip = maxf(max_rear_slip, rear_slip)
		if frame % 5 == 0 or frame < 10:
			file.store_line("%d,%d,%.6f,%.6f,%.6f,%.1f,%d,1.0,%.6f,%.6f,%.6f,%.6f,%.3f,%.3f,%.8f,%.8f" % [
				frame, int(frame * 1000.0 / Engine.physics_ticks_per_second), forward_velocity * 3.6, lateral_velocity * 3.6,
				linear_velocity.length() * 3.6, vehicle.motor_rpm, vehicle.current_gear, vehicle.lat_g, vehicle.long_g,
				yaw_rate, rear_slip, comp[2], comp[3], beta, f_yaw])
	file.close()
	print("[OK] fixed throttle aero: max_abs_lat_g=%.6f max_abs_yaw_rate=%.6f max_rear_slip=%.6f final_forward_kmh=%.3f" % [
		max_abs_lat, max_abs_yaw_rate, max_rear_slip,
		(-vehicle.global_transform.basis.z.dot(vehicle.linear_velocity)) * 3.6])
	quit(0)
