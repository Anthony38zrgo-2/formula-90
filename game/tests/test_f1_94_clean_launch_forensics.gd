extends SceneTree

const SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"
const SETTLE_FRAMES := 180
const LAUNCH_FRAMES := 600

func _init() -> void:
	call_deferred("_run")

func _wait_for_settle(vehicle) -> bool:
	for _i in range(SETTLE_FRAMES):
		await physics_frame

	var basis: Basis = vehicle.global_transform.basis
	var linear_velocity: Vector3 = vehicle.linear_velocity
	var lateral_velocity: float = basis.x.dot(linear_velocity)
	var yaw_rate: float = vehicle.angular_velocity.y
	var comp = vehicle.get_wheel_compressions()
	var failures: Array[String] = []
	# Mechanical suspension upgrade: compression_mm is the wheel-center suspension
	# travel, which settles at spring_length*resting_ratio (70 mm front, 70 mm rear).
	# The ~8 mm static deflection now lives in the tire carcass (tire_deflection_m).
	for index in [0, 1]:
		if absf(float(comp[index]) - 70.0) > 2.0:
			failures.append("front compression[%d]=%.3fmm, expected 70+-2mm" % [index, comp[index]])
	for index in [2, 3]:
		if absf(float(comp[index]) - 70.0) > 2.0:
			failures.append("rear compression[%d]=%.3fmm, expected 70+-2mm" % [index, comp[index]])
	if absf(lateral_velocity) >= 0.02:
		failures.append("lateral velocity=%.6fm/s, expected <0.02m/s" % lateral_velocity)
	if absf(yaw_rate) >= 0.01:
		failures.append("yaw rate=%.6frad/s, expected <0.01rad/s" % yaw_rate)
	if not failures.is_empty():
		for failure in failures:
			printerr("[FAIL] pre-throttle settle: " + failure)
		return false
	print("[OK] pre-throttle settle: FL=%.3fmm FR=%.3fmm RL=%.3fmm RR=%.3fmm lateral=%.6fm/s yaw=%.6frad/s" % [
		comp[0], comp[1], comp[2], comp[3], lateral_velocity, yaw_rate])
	return true

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
	if not await _wait_for_settle(vehicle):
		quit(1)
		return
	var file := FileAccess.open("user://f1_94_clean_launch_forensics.csv", FileAccess.WRITE)
	file.store_line("phase,frame,time_ms,body_y,total_speed_kmh,forward_kmh,lateral_kmh,vertical_kmh,rpm,gear,engine_torque,clutch_engagement,clutch_torque,fl_drive_torque,fr_drive_torque,rl_drive_torque,rr_drive_torque,fl_normal_force,fr_normal_force,rl_normal_force,rr_normal_force,lat_g,long_g,vert_g,yaw_rate_rad_s,yaw_rad,rear_slip_max,front_slip_max,fl_spin,fr_spin,rl_spin,rr_spin,fl_slip,fr_slip,rl_slip,rr_slip,fl_comp,fr_comp,rl_comp,rr_comp")
	var max_rear_slip: float = 0.0
	var max_abs_lat_g: float = 0.0
	var max_abs_yaw_rate: float = 0.0
	var max_clutch_engagement: float = 0.0
	var launch_min_forward: float = 1e30
	for frame in range(LAUNCH_FRAMES):
		vehicle.throttle_amount = 1.0
		await physics_frame
		var basis: Basis = vehicle.global_transform.basis
		var linear_velocity: Vector3 = vehicle.linear_velocity
		var forward_velocity: float = -basis.z.dot(linear_velocity)
		var lateral_velocity: float = basis.x.dot(linear_velocity)
		var vertical_velocity: float = linear_velocity.y
		var spins = vehicle.get_wheel_spins()
		var slips = vehicle.get_wheel_slips()
		var comp = vehicle.get_wheel_compressions()
		var drive_torques = vehicle.get_drive_torques()
		var normal_forces = vehicle.get_normal_forces()
		var rear_slip: float = maxf(absf(float(slips[2])), absf(float(slips[3])))
		var front_slip: float = maxf(absf(float(slips[0])), absf(float(slips[1])))
		var clutch_engagement: float = float(vehicle.get("clutch_engagement"))
		var yaw_rate: float = vehicle.angular_velocity.y
		var yaw_rad: float = vehicle.global_transform.basis.get_euler().y
		max_rear_slip = maxf(max_rear_slip, rear_slip)
		max_abs_lat_g = maxf(max_abs_lat_g, absf(vehicle.lat_g))
		max_abs_yaw_rate = maxf(max_abs_yaw_rate, absf(yaw_rate))
		max_clutch_engagement = maxf(max_clutch_engagement, clutch_engagement)
		launch_min_forward = minf(launch_min_forward, forward_velocity)
		if frame % 5 == 0 or frame < 10:
			file.store_line("launch,%d,%d,%.6f,%.6f,%.6f,%.6f,%.6f,%.1f,%d,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.3f,%.3f,%.3f,%.3f" % [
				frame, int(frame * 1000.0 / Engine.physics_ticks_per_second), vehicle.global_position.y,
				linear_velocity.length() * 3.6, forward_velocity * 3.6, lateral_velocity * 3.6, vertical_velocity * 3.6,
				vehicle.motor_rpm, vehicle.current_gear, float(vehicle.get("engine_torque")), clutch_engagement, vehicle.clutch_torque,
				drive_torques[0], drive_torques[1], drive_torques[2], drive_torques[3],
				normal_forces[0], normal_forces[1], normal_forces[2], normal_forces[3],
				vehicle.lat_g, vehicle.long_g, vehicle.vert_g, yaw_rate, yaw_rad, rear_slip, front_slip,
				spins[0], spins[1], spins[2], spins[3], slips[0], slips[1], slips[2], slips[3],
				comp[0], comp[1], comp[2], comp[3]])
	file.close()
	print("[OK] clean launch: max_rear_slip=%.6f max_abs_lat_g=%.6f max_abs_yaw_rate=%.6f max_clutch=%.6f min_forward_mps=%.6f final_forward_kmh=%.3f" % [
		max_rear_slip, max_abs_lat_g, max_abs_yaw_rate, max_clutch_engagement, launch_min_forward,
		(-vehicle.global_transform.basis.z.dot(vehicle.linear_velocity)) * 3.6])
	quit(0)
