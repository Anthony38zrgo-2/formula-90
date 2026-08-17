extends SceneTree

const SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"
const SETTLE_TICKS := 180
const THROTTLE_TICKS := 600
const BASELINE_NAME := "A"

const VARIANTS := [
	{"name": "A", "preload": 120.0, "power": 65.0, "coast": 75.0, "clutches": 4.0, "mu": 0.15},
	{"name": "B", "preload": 170.0, "power": 65.0, "coast": 75.0, "clutches": 4.0, "mu": 0.0},
	{"name": "C", "preload": 120.0, "power": 65.0, "coast": 75.0, "clutches": 4.0, "mu": 0.05},
	{"name": "D", "preload": 150.0, "power": 75.0, "coast": 85.0, "clutches": 4.0, "mu": 0.05},
]

func _init() -> void:
	call_deferred("_run")

func _run() -> void:
	var packed := load(SCENE_PATH) as PackedScene
	if packed == null:
		printerr("[FAIL] CORR-03 scene missing: %s" % SCENE_PATH)
		quit(1)
		return

	var telemetry := FileAccess.open("user://corr_03_diff_matrix.csv", FileAccess.WRITE)
	var summary := FileAccess.open("user://corr_03_diff_matrix_summary.csv", FileAccess.WRITE)
	if telemetry == null or summary == null:
		printerr("[FAIL] CORR-03 could not open user:// output files")
		quit(1)
		return

	telemetry.store_line("variant,tick,time_s,fl_slip,fr_slip,rl_slip,rr_slip,fl_spin,fr_spin,rl_spin,rr_spin,fl_drive_torque,fr_drive_torque,rl_drive_torque,rr_drive_torque,fl_normal_force,fr_normal_force,rl_normal_force,rr_normal_force,forward_velocity_mps,lateral_velocity_mps,lat_g,yaw_rate_rad_s,rl_compression,rr_compression,diff_preload,diff_power_ramp_deg,diff_coast_ramp_deg,diff_clutches,diff_mu,clutch_torque")
	summary.store_line("variant,preload,power_ramp_deg,coast_ramp_deg,clutches,mu,max_abs_lat_g,max_abs_yaw_rate_rad_s,max_rear_slip,baseline_max_rear_slip,final_forward_velocity_mps,speed_delta_pct,lat_g_pass,yaw_pass,rear_slip_pass,speed_pass,qualifies")

	var results: Array[Dictionary] = []
	for config in VARIANTS:
		var result := await _run_variant(packed, config, telemetry)
		results.append(result)

	var baseline := results[0]
	var selected := "NONE"
	var selected_result: Dictionary = {}
	for result in results:
		var rear_slip_pass: bool = result.max_rear_slip <= baseline.max_rear_slip + 1e-6
		var speed_delta_pct: float = _speed_delta_pct(result.final_forward_velocity_mps, baseline.final_forward_velocity_mps)
		var speed_pass: bool = absf(speed_delta_pct) <= 5.0
		var lat_pass: bool = result.max_abs_lat_g <= 0.05
		var yaw_pass: bool = result.max_abs_yaw_rate_rad_s <= 0.05
		var qualifies: bool = lat_pass and yaw_pass and rear_slip_pass and speed_pass
		result["baseline_max_rear_slip"] = baseline.max_rear_slip
		result["speed_delta_pct"] = speed_delta_pct
		result["qualifies"] = qualifies
		result["lat_pass"] = lat_pass
		result["yaw_pass"] = yaw_pass
		result["rear_slip_pass"] = rear_slip_pass
		result["speed_pass"] = speed_pass
		summary.store_line("%s,%.3f,%.3f,%.3f,%.3f,%.3f,%.6f,%.6f,%.6f,%.6f,%.6f,%.3f,%s,%s,%s,%s,%s" % [
			result.variant, result.preload, result.power, result.coast, result.clutches, result.mu,
			result.max_abs_lat_g, result.max_abs_yaw_rate_rad_s, result.max_rear_slip,
			result.baseline_max_rear_slip, result.final_forward_velocity_mps, result.speed_delta_pct,
			str(result.lat_pass), str(result.yaw_pass), str(result.rear_slip_pass), str(result.speed_pass), str(result.qualifies)])
		if qualifies and (selected_result.is_empty() or result.max_abs_lat_g < selected_result.max_abs_lat_g or (is_equal_approx(result.max_abs_lat_g, selected_result.max_abs_lat_g) and result.max_abs_yaw_rate_rad_s < selected_result.max_abs_yaw_rate_rad_s)):
			selected_result = result
			selected = result.variant

	telemetry.close()
	summary.close()
	print("=== CORR-03 differential matrix ===")
	for result in results:
		print("[METRIC] %s max_abs_lat_g=%.6f max_abs_yaw_rate=%.6f max_rear_slip=%.6f final_forward_mps=%.6f speed_delta_pct=%.3f qualifies=%s diff=%.3f/%.3f/%.3f/%.3f/%.3f" % [
			result.variant, result.max_abs_lat_g, result.max_abs_yaw_rate_rad_s, result.max_rear_slip,
			result.final_forward_velocity_mps, result.speed_delta_pct, str(result.qualifies),
			result.preload, result.power, result.coast, result.clutches, result.mu])
	print("[DECISION] selected=%s defaults_changed=false telemetry=user://corr_03_diff_matrix.csv summary=user://corr_03_diff_matrix_summary.csv" % selected)
	quit(0)

func _run_variant(packed: PackedScene, config: Dictionary, telemetry: FileAccess) -> Dictionary:
	var session = packed.instantiate()
	root.add_child(session)
	for _i in range(4):
		await process_frame
	var vehicle = session.find_child("VehicleRigidBody", true, false)
	if vehicle == null:
		printerr("[FAIL] CORR-03 VehicleRigidBody missing for %s" % config.name)
		quit(1)
		return {}
	vehicle.enable_player_input = false
	vehicle.throttle_amount = 0.0
	for _i in range(SETTLE_TICKS):
		vehicle.throttle_amount = 0.0
		await physics_frame

	vehicle.set_diff_preload(config.preload)
	vehicle.set_diff_power_ramp_angle_deg(config.power)
	vehicle.set_diff_coast_ramp_angle_deg(config.coast)
	vehicle.set_diff_clutches(config.clutches)
	vehicle.set_diff_clutch_friction_coeff(config.mu)
	await physics_frame

	var max_abs_lat_g := 0.0
	var max_abs_yaw_rate := 0.0
	var max_rear_slip := 0.0
	var final_forward_velocity := 0.0
	for tick in range(THROTTLE_TICKS):
		vehicle.throttle_amount = 1.0
		await physics_frame
		var basis: Basis = vehicle.global_transform.basis
		var velocity: Vector3 = vehicle.linear_velocity
		var forward_velocity := -basis.z.dot(velocity)
		var lateral_velocity := basis.x.dot(velocity)
		var slips = vehicle.get_wheel_slips()
		var spins = vehicle.get_wheel_spins()
		var drives = vehicle.get_drive_torques()
		var normals = vehicle.get_normal_forces()
		var compressions = vehicle.get_wheel_compressions()
		var rear_slip := maxf(absf(float(slips[2])), absf(float(slips[3])))
		max_abs_lat_g = maxf(max_abs_lat_g, absf(float(vehicle.lat_g)))
		max_abs_yaw_rate = maxf(max_abs_yaw_rate, absf(vehicle.angular_velocity.y))
		max_rear_slip = maxf(max_rear_slip, rear_slip)
		final_forward_velocity = forward_velocity
		telemetry.store_line("%s,%d,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f" % [
			config.name, tick, float(tick + 1) / Engine.physics_ticks_per_second,
			slips[0], slips[1], slips[2], slips[3], spins[0], spins[1], spins[2], spins[3],
			drives[0], drives[1], drives[2], drives[3], normals[0], normals[1], normals[2], normals[3],
			forward_velocity, lateral_velocity, vehicle.lat_g, vehicle.angular_velocity.y,
			compressions[2], compressions[3], vehicle.get_diff_preload(),
			vehicle.get_diff_power_ramp_angle_deg(), vehicle.get_diff_coast_ramp_angle_deg(),
			vehicle.get_diff_clutches(), vehicle.get_diff_clutch_friction_coeff(), vehicle.clutch_torque])

	var result := {
		"variant": config.name, "preload": config.preload, "power": config.power, "coast": config.coast,
		"clutches": config.clutches, "mu": config.mu, "max_abs_lat_g": max_abs_lat_g,
		"max_abs_yaw_rate_rad_s": max_abs_yaw_rate, "max_rear_slip": max_rear_slip,
		"final_forward_velocity_mps": final_forward_velocity, "qualifies": false,
	}
	session.queue_free()
	await process_frame
	return result

func _speed_delta_pct(value: float, baseline: float) -> float:
	if is_zero_approx(baseline):
		return 0.0 if is_zero_approx(value) else INF
	return ((value - baseline) / absf(baseline)) * 100.0
