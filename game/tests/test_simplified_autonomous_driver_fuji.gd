extends SceneTree

var connection := StreamPeerTCP.new()
var response_buffer := ""
var vehicle: RigidBody3D
var track: Node3D
var telemetry_file: FileAccess
var output_directory := ""
var physics_ticks := 0
var elapsed_seconds := 0.0
var travelled_metres := 0.0
var previous_arc_metres := 0.0
var lap_length_metres := 0.0
var lap_crossing_seconds: Array[float] = []
var maximum_speed_metres_per_second := 0.0
var maximum_lateral_metres := 0.0
var maximum_tilt_degrees := 0.0
var last_progress_seconds := 0.0
var full_throttle_ticks := 0
var braking_ticks := 0
var control_ticks := 0
var failure_reason := ""
var finished := false
var session_scene_path := "res://scenes/runtime/vehicle_test_session.tscn"
var target_laps := 2
var driver_port := 39194
var maximum_simulated_seconds := 600.0
var diagnostic_file: FileAccess
var start_milliseconds := 0
var steering_mapping: Array = []
var observation_mode := false
var controls_capture_requested := false
var observations_file: FileAccess
var driver_configuration: Dictionary
var control_elapsed_seconds := 0.0
var control_duration_seconds := 0.0
var capacity_elapsed_seconds := 0.0
var slow_state_elapsed_seconds := 0.0

func _initialize() -> void:
	call_deferred("initialize_validation")

func initialize_validation() -> void:
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--port="):
			driver_port = int(argument.trim_prefix("--port="))
		if argument.begins_with("--scene="):
			session_scene_path = argument.trim_prefix("--scene=")
		if argument.begins_with("--laps="):
			target_laps = int(argument.trim_prefix("--laps="))
		if argument.begins_with("--maximum-seconds="):
			maximum_simulated_seconds = float(argument.trim_prefix("--maximum-seconds="))
		if argument == "--observe":
			observation_mode = true
		if argument.begins_with("--output="):
			output_directory = argument.trim_prefix("--output=")
	if output_directory.is_empty():
		push_error("Canonical autonomous validation requires an output directory")
		quit(1)
		return
	driver_configuration = JSON.parse_string(FileAccess.get_file_as_string(output_directory.path_join("driver_tuning.json")))
	steering_mapping = JSON.parse_string(FileAccess.get_file_as_string(output_directory.path_join("steering_mapping.json")))
	if steering_mapping.size() != 1001:
		push_error("Missing source-derived steering adapter calibration")
		quit(1)
		return
	if float(driver_configuration["live_grip_control"]["control_hertz"]) > float(Engine.physics_ticks_per_second):
		push_error("Driver control frequency exceeds the mechanical frequency")
		quit(1)
		return
	connection.connect_to_host("127.0.0.1", driver_port)
	var connection_deadline := Time.get_ticks_msec() + 10000
	while connection.get_status() != StreamPeerTCP.STATUS_CONNECTED:
		connection.poll()
		if Time.get_ticks_msec() > connection_deadline:
			push_error("Cannot connect to the autonomous Rust driver")
			quit(1)
			return
		await process_frame
	connection.set_no_delay(true)
	var session: Node = load(session_scene_path).instantiate()
	root.add_child(session)
	current_scene = session
	for settling_frame in 10:
		await process_frame
	var race_session: Node = session.get_node("WorldViewport/RaceSession")
	vehicle = race_session.get("active_vehicle") as RigidBody3D
	track = race_session.get("active_track") as Node3D
	if vehicle == null or track == null:
		push_error("Canonical Fuji scene did not compose")
		quit(1)
		return
	disable_player_input(session)
	vehicle.set("enable_player_input", false)
	var racing_line: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://tracks/fuji76_77/metadata/racing_line.json"))
	lap_length_metres = float(racing_line["lap_length_m"])
	var first: Array = racing_line["waypoints"][0]["pos"]
	var second: Array = racing_line["waypoints"][1]["pos"]
	var origin := track.to_global(Vector3(float(first[0]), float(first[1]), float(first[2])))
	var next := track.to_global(Vector3(float(second[0]), float(second[1]), float(second[2])))
	vehicle.call("reset_vehicle", origin + Vector3.UP * float(vehicle.call("get_default_spawn_height")), atan2(-(next.x - origin.x), -(next.z - origin.z)))
	vehicle.set("automatic_transmission", true)
	vehicle.call("set_throttle_amount", 0.0)
	vehicle.call("set_brake_amount", 0.2)
	for settling_tick in 240:
		await physics_frame
	telemetry_file = FileAccess.open(output_directory.path_join("fuji_telemetry.csv"), FileAccess.WRITE)
	telemetry_file.store_csv_line(["time_seconds", "arc_metres", "travelled_metres", "lateral_metres", "speed_metres_per_second", "target_speed_metres_per_second", "steering", "throttle", "brake", "gear", "engine_revolutions_per_minute", "height_metres", "tilt_degrees", "recovery_speed_fraction", "available_half_width_metres", "canonical_steering", "position_x_metres", "position_z_metres", "heading_radians", "planned_acceleration_metres_per_second_squared", "regularized_acceleration_metres_per_second_squared", "longitudinal_demand_fraction", "front_capacity_scale", "rear_capacity_scale", "front_peak_utilization", "rear_peak_utilization", "understeer_detected", "critical_grip_state", "capacity_updates", "planning_updates", "slow_state_updates"])
	if bool(driver_configuration["live_grip_control"]["record_observations"]):
		observations_file = FileAccess.open(output_directory.path_join("driver_observations.jsonl"), FileAccess.WRITE)
	if "--diagnostics" in OS.get_cmdline_user_args():
		diagnostic_file = FileAccess.open(output_directory.path_join("physical_diagnostics.jsonl"), FileAccess.WRITE)
	start_milliseconds = Time.get_ticks_msec()
	physics_frame.connect(advance_validation)
	print("FUJI_SIMPLIFIED_DRIVER_READY physics_hertz=%d control_hertz=%s maximum_speed=null target_laps=%d" % [Engine.physics_ticks_per_second, driver_configuration["live_grip_control"]["control_hertz"], target_laps])

func disable_player_input(node: Node) -> void:
	var script: Script = node.get_script()
	if script != null and script.resource_path.contains("input"):
		node.set_process(false)
		node.set_physics_process(false)
		node.set_process_input(false)
		node.set_process_unhandled_input(false)
	for child in node.get_children():
		disable_player_input(child)

func request_driver(observation: Dictionary) -> Dictionary:
	if connection.put_data((JSON.stringify(observation) + "\n").to_utf8_buffer()) != OK:
		failure_reason = "Cannot submit driver observation"
		return {}
	var deadline := Time.get_ticks_msec() + 10000
	while not response_buffer.contains("\n"):
		connection.poll()
		if connection.get_status() != StreamPeerTCP.STATUS_CONNECTED or Time.get_ticks_msec() > deadline:
			failure_reason = "Autonomous driver disconnected or timed out"
			return {}
		if connection.get_available_bytes() > 0:
			response_buffer += connection.get_utf8_string(connection.get_available_bytes())
		else:
			OS.delay_usec(100)
	var line_end := response_buffer.find("\n")
	var response: Dictionary = JSON.parse_string(response_buffer.substr(0, line_end))
	response_buffer = response_buffer.substr(line_end + 1)
	if not response.get("success", false):
		failure_reason = str(response.get("error", "Driver rejected observation"))
		return {}
	return response["command"]

func advance_validation() -> void:
	if finished:
		return
	physics_ticks += 1
	elapsed_seconds = float(physics_ticks - 1) / float(Engine.physics_ticks_per_second)
	var physical_duration := 1.0 / float(Engine.physics_ticks_per_second)
	control_elapsed_seconds += physical_duration
	control_duration_seconds += physical_duration
	capacity_elapsed_seconds += physical_duration
	slow_state_elapsed_seconds += physical_duration
	var live_settings: Dictionary = driver_configuration["live_grip_control"]
	var control_interval := 1.0 / float(live_settings["control_hertz"])
	if control_elapsed_seconds + 0.000000001 < control_interval:
		return
	var control_duration := control_duration_seconds
	control_duration_seconds = 0.0
	control_elapsed_seconds = maxf(0.0, control_elapsed_seconds - control_interval)
	var local_position := track.to_local(vehicle.global_position)
	var local_forward := track.global_basis.inverse() * -vehicle.global_basis.z
	var speed := vehicle.linear_velocity.length()
	var observation := {"position_x_metres": local_position.x, "position_z_metres": local_position.z, "heading_radians": atan2(-local_forward.x, -local_forward.z), "speed_metres_per_second": speed, "duration_seconds": control_duration, "current_gear": int(vehicle.call("get_current_gear"))}
	if bool(live_settings["enabled"]):
		var include_capacity := control_ticks == 0 or capacity_elapsed_seconds + 0.000000001 >= 1.0 / float(live_settings["capacity_estimation_hertz"])
		var include_slow_state := control_ticks == 0 or slow_state_elapsed_seconds + 0.000000001 >= 1.0 / float(live_settings["slow_state_hertz"])
		var dynamics: Dictionary = vehicle.call("get_driver_state_snapshot", include_capacity, include_slow_state)
		if dynamics.is_empty():
			failure_reason = "Canonical vehicle did not expose live driver dynamics"
			finish_validation()
			return
		var body_velocity := vehicle.global_basis.inverse() * vehicle.linear_velocity
		var body_rotation := vehicle.global_basis.inverse() * vehicle.angular_velocity
		dynamics["fast_state"]["lateral_speed_metres_per_second"] = -body_velocity.x
		dynamics["fast_state"]["yaw_rate_radians_per_second"] = -body_rotation.y
		observation["vehicle_dynamics"] = dynamics
		if include_capacity:
			capacity_elapsed_seconds = 0.0 if control_ticks == 0 else maxf(0.0, capacity_elapsed_seconds - 1.0 / float(live_settings["capacity_estimation_hertz"]))
		if include_slow_state:
			slow_state_elapsed_seconds = 0.0 if control_ticks == 0 else maxf(0.0, slow_state_elapsed_seconds - 1.0 / float(live_settings["slow_state_hertz"]))
	if observations_file != null:
		observations_file.store_line(JSON.stringify(observation))
	var command := request_driver(observation)
	if command.is_empty():
		finish_validation()
		return
	if diagnostic_file != null:
		var local_velocity := vehicle.global_basis.inverse() * vehicle.linear_velocity
		var local_angular_velocity := vehicle.global_basis.inverse() * vehicle.angular_velocity
		diagnostic_file.store_line(JSON.stringify({"time_seconds": elapsed_seconds, "position": vehicle.global_position, "body_basis": vehicle.global_basis, "local_velocity": local_velocity, "local_angular_velocity": local_angular_velocity, "steer_angle_radians": vehicle.call("get_steer_angle_rad"), "normal_forces": Array(vehicle.call("get_normal_forces")), "compressions": Array(vehicle.call("get_wheel_compressions")), "tire_state": vehicle.call("get_tire_state_snapshot"), "brake_state": vehicle.call("get_brake_state_snapshot"), "command": command, "input": vehicle.get("steering_input")}))
	var arc := float(command["arc_length_metres"])
	var progress := arc - previous_arc_metres
	if progress < -lap_length_metres * 0.5:
		progress += lap_length_metres
	if control_ticks == 0:
		progress = 0.0
	if absf(progress) > 5.0:
		failure_reason = "Discontinuous racing line projection"
	travelled_metres += progress
	previous_arc_metres = arc
	if progress > 0.01:
		last_progress_seconds = elapsed_seconds
	var lateral := absf(float(command["lateral_offset_metres"]))
	var tilt := rad_to_deg(acos(clampf(vehicle.global_basis.y.dot(Vector3.UP), -1.0, 1.0)))
	maximum_speed_metres_per_second = maxf(maximum_speed_metres_per_second, speed)
	maximum_lateral_metres = maxf(maximum_lateral_metres, lateral)
	maximum_tilt_degrees = maxf(maximum_tilt_degrees, tilt)
	if lateral + float(command["vehicle_half_width_metres"]) > float(command["available_half_width_metres"]):
		failure_reason = "Vehicle footprint exceeded track width"
	if not vehicle.global_position.is_finite() or not is_finite(speed) or tilt > 60.0:
		failure_reason = "Invalid or overturned physical state"
	if elapsed_seconds - last_progress_seconds > 15.0:
		failure_reason = "Vehicle stalled"
	if elapsed_seconds >= maximum_simulated_seconds:
		if observation_mode and lap_crossing_seconds.size() >= target_laps:
			finish_validation()
			return
		failure_reason = "Time limit reached"
	var throttle := float(command["throttle_fraction"])
	var brake := float(command["brake_fraction"])
	var mapping_position := clampf((float(command["steering_fraction"]) + 1.0) * 500.0, 0.0, 1000.0)
	var mapping_index := mini(floori(mapping_position), 999)
	var canonical_steering := lerpf(float(steering_mapping[mapping_index]), float(steering_mapping[mapping_index + 1]), mapping_position - float(mapping_index))
	vehicle.call("set_steering_input", canonical_steering)
	vehicle.call("set_throttle_amount", throttle)
	vehicle.call("set_brake_amount", brake)
	if command["gear_request"] != null:
		vehicle.call("set_gear_request", int(command["gear_request"]))
	if throttle >= 0.95:
		full_throttle_ticks += 1
	if brake > 0.01:
		braking_ticks += 1
	control_ticks += 1
	telemetry_file.store_csv_line([elapsed_seconds, arc, travelled_metres, command["lateral_offset_metres"], speed, command["target_speed_metres_per_second"], command["steering_fraction"], throttle, brake, vehicle.call("get_current_gear"), vehicle.call("get_motor_rpm"), vehicle.global_position.y, tilt, command["recovery_speed_fraction"], command["available_half_width_metres"], canonical_steering, local_position.x, local_position.z, atan2(-local_forward.x, -local_forward.z), command.get("planned_acceleration_metres_per_second_squared", 0.0), command.get("regularized_acceleration_metres_per_second_squared", 0.0), command.get("longitudinal_demand_fraction", throttle - brake), command["live_grip"]["front_capacity_scale"], command["live_grip"]["rear_capacity_scale"], command["live_grip"]["front_peak_utilization"], command["live_grip"]["rear_peak_utilization"], command["live_grip"]["understeer_detected"], command["live_grip"]["critical_grip_state"], command["live_grip"]["capacity_updates"], command["live_grip"]["planning_updates"], command["live_grip"]["slow_state_updates"]])
	if travelled_metres >= lap_length_metres * float(lap_crossing_seconds.size() + 1):
		lap_crossing_seconds.append(elapsed_seconds)
	if control_ticks % 600 == 0:
		telemetry_file.flush()
		print("FUJI_DRIVER_PROGRESS time=%.2f distance=%.2f speed=%.2f lateral=%.3f" % [elapsed_seconds, travelled_metres, speed * 3.6, lateral])
	if observation_mode and not controls_capture_requested and elapsed_seconds >= 2.0:
		controls_capture_requested = true
		call_deferred("capture_controls_overlay")
	if not failure_reason.is_empty() or (lap_crossing_seconds.size() >= target_laps and not observation_mode):
		finish_validation()

func capture_controls_overlay() -> void:
	await RenderingServer.frame_post_draw
	root.get_texture().get_image().save_png(output_directory.path_join("controls_overlay.png"))

func finish_validation() -> void:
	finished = true
	vehicle.call("set_throttle_amount", 0.0)
	vehicle.call("set_brake_amount", 1.0)
	if diagnostic_file != null:
		diagnostic_file.close()
	if observations_file != null:
		observations_file.close()
	telemetry_file.close()
	connection.disconnect_from_host()
	var passed := failure_reason.is_empty() and lap_crossing_seconds.size() >= target_laps
	var report := {"passed": passed, "failure_reason": failure_reason, "completed_laps": lap_crossing_seconds.size(), "lap_crossing_seconds": lap_crossing_seconds, "flying_lap_seconds": lap_crossing_seconds[1] - lap_crossing_seconds[0] if lap_crossing_seconds.size() >= 2 else null, "simulated_seconds": elapsed_seconds, "travelled_metres": travelled_metres, "lap_length_metres": lap_length_metres, "maximum_speed_kilometres_per_hour": maximum_speed_metres_per_second * 3.6, "maximum_lateral_metres": maximum_lateral_metres, "maximum_tilt_degrees": maximum_tilt_degrees, "full_throttle_ticks": full_throttle_ticks, "braking_ticks": braking_ticks, "control_ticks": control_ticks, "physical_frequency_hertz": Engine.physics_ticks_per_second, "control_frequency_hertz": driver_configuration["live_grip_control"]["control_hertz"], "maximum_speed_metres_per_second": null, "automatic_transmission": vehicle.get("automatic_transmission"), "driving_aids_mask": vehicle.get("aids_enabled_mask"), "wall_seconds": float(Time.get_ticks_msec() - start_milliseconds) / 1000.0, "build_source": FileAccess.get_file_as_string("res://BUILD_SOURCE").strip_edges(), "physics_source": "Canonical Godot rigid body with Rust force evaluation", "backend_selection": {"aerodynamics": OS.get_environment("FORMULA90_AERODYNAMICS_BACKEND"), "suspension": OS.get_environment("FORMULA90_SUSPENSION_BACKEND"), "tires": OS.get_environment("FORMULA90_TIRE_BACKEND"), "slow_states": OS.get_environment("FORMULA90_SLOW_STATE_BACKEND")}}
	FileAccess.open(output_directory.path_join("fuji_report.json"), FileAccess.WRITE).store_string(JSON.stringify(report, "\t"))
	print("FUJI_SIMPLIFIED_DRIVER_RESULT ", JSON.stringify(report))
	quit(0 if passed else 1)
