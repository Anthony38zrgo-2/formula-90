extends SceneTree

const SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"
const SETTLE_FRAMES := 180
const RUN_FRAMES := 900
const YAW_DECAY_EXPONENT := 1.30
const YAW_ONSET_THRESHOLD := 0.01

func _init() -> void:
	call_deferred("_run")

func _run() -> void:
	var packed := load(SCENE_PATH) as PackedScene
	if packed == null:
		printerr("[FAIL] CORR-05 scene missing")
		quit(1)
		return
	var session = packed.instantiate()
	root.add_child(session)
	for _i in range(4):
		await process_frame
	var vehicle = session.find_child("VehicleRigidBody", true, false)
	if vehicle == null:
		printerr("[FAIL] CORR-05 VehicleRigidBody missing")
		quit(1)
		return

	vehicle.enable_player_input = false
	vehicle.throttle_amount = 0.0
	for _i in range(SETTLE_FRAMES):
		await physics_frame

	var diff_preload := float(vehicle.get_diff_preload())
	var power_ramp := float(vehicle.get_diff_power_ramp_angle_deg())
	var coast_ramp := float(vehicle.get_diff_coast_ramp_angle_deg())
	var clutches := float(vehicle.get_diff_clutches())
	var mu := float(vehicle.get_diff_clutch_friction_coeff())
	print("[CORR-05] defaults LSD=%.1f/%.1f/%.1f/%.1f/%.1f settle=%d throttle=1.0 aero_exp=%.2f" % [diff_preload, power_ramp, coast_ramp, clutches, mu, SETTLE_FRAMES, YAW_DECAY_EXPONENT])
	if not is_equal_approx(diff_preload, 170.0) or not is_equal_approx(power_ramp, 65.0) or not is_equal_approx(coast_ramp, 75.0) or not is_equal_approx(clutches, 4.0) or not is_zero_approx(mu):
		printerr("[FAIL] CORR-05 defaults differ from requested current state")
		quit(1)
		return

	var file := FileAccess.open("user://f1_94_corr_05_aero_diagnostic.csv", FileAccess.WRITE)
	file.store_line("frame,time_ms,forward_velocity_mps,lateral_velocity_mps,beta_rad,f_yaw_est,fl_slip,fr_slip,rl_slip,rr_slip,fl_normal_force,fr_normal_force,rl_normal_force,rr_normal_force,fl_comp,fr_comp,rl_comp,rr_comp,lat_g,yaw_rate_rad_s")
	var onset_frame := -1
	var onset_speed := 0.0
	var onset_f_yaw := 0.0
	var max_abs_lat_g := 0.0
	var max_abs_yaw_rate := 0.0
	for frame in range(RUN_FRAMES):
		vehicle.throttle_amount = 1.0
		await physics_frame
		var basis: Basis = vehicle.global_transform.basis
		var velocity: Vector3 = vehicle.linear_velocity
		var forward_velocity := -basis.z.dot(velocity)
		var lateral_velocity := basis.x.dot(velocity)
		var beta := atan2(lateral_velocity, maxf(absf(forward_velocity), 1e-6))
		var f_yaw := pow(clampf(cos(beta), 0.0, 1.0), YAW_DECAY_EXPONENT)
		var slips = vehicle.get_wheel_slips()
		var normals = vehicle.get_normal_forces()
		var comp = vehicle.get_wheel_compressions()
		var yaw_rate := float(vehicle.angular_velocity.y)
		max_abs_lat_g = maxf(max_abs_lat_g, absf(float(vehicle.lat_g)))
		max_abs_yaw_rate = maxf(max_abs_yaw_rate, absf(yaw_rate))
		if onset_frame < 0 and absf(yaw_rate) >= YAW_ONSET_THRESHOLD:
			onset_frame = frame
			onset_speed = forward_velocity * 3.6
			onset_f_yaw = f_yaw
		file.store_line("%d,%d,%.9f,%.9f,%.9f,%.9f,%.9f,%.9f,%.9f,%.9f,%.3f,%.3f,%.3f,%.3f,%.3f,%.3f,%.3f,%.3f,%.9f,%.9f" % [
			frame, int(frame * 1000.0 / Engine.physics_ticks_per_second), forward_velocity, lateral_velocity, beta, f_yaw,
			slips[0], slips[1], slips[2], slips[3], normals[0], normals[1], normals[2], normals[3],
			comp[0], comp[1], comp[2], comp[3], vehicle.lat_g, yaw_rate])
	file.close()
	print("[CORR-05] onset_abs_yaw_rate>=%.3f frame=%d speed_kmh=%.3f f_yaw=%.9f" % [YAW_ONSET_THRESHOLD, onset_frame, onset_speed, onset_f_yaw])
	print("[CORR-05] max_abs_lat_g=%.6f max_abs_yaw_rate=%.6f final_forward_kmh=%.3f" % [max_abs_lat_g, max_abs_yaw_rate, -vehicle.global_transform.basis.z.dot(vehicle.linear_velocity) * 3.6])
	quit(0)
