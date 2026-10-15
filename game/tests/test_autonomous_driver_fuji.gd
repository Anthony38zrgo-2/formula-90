extends SceneTree

var world_interface: RefCounted
var connection := StreamPeerTCP.new()
var response_buffer := ""
var world_identifier: int = 0
var failed: bool = false

func _initialize() -> void:
	call_deferred("run_fuji_validation")

func argument_value(prefix: String, fallback: String) -> String:
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with(prefix):
			return argument.trim_prefix(prefix)
	return fallback

func request_world(request: Dictionary) -> Dictionary:
	if world_identifier != 0:
		request["world_identifier"] = world_identifier
	var response: Dictionary = world_interface.execute_request(request)
	if not response.get("success", false):
		failed = true
		push_error(str(response.get("error", "Physical world request failed")))
		return {}
	return response["result"]

func request_driver(observation: Dictionary) -> Dictionary:
	if connection.put_data((JSON.stringify(observation) + "\n").to_utf8_buffer()) != OK:
		failed = true
		return {}
	var deadline_milliseconds := Time.get_ticks_msec() + 10000
	while not response_buffer.contains("\n"):
		connection.poll()
		if connection.get_status() != StreamPeerTCP.STATUS_CONNECTED or Time.get_ticks_msec() > deadline_milliseconds:
			failed = true
			push_error("Artificial driver bridge disconnected or timed out")
			return {}
		var available_bytes := connection.get_available_bytes()
		if available_bytes > 0:
			response_buffer += connection.get_utf8_string(available_bytes)
		else:
			await process_frame
	var line_end := response_buffer.find("\n")
	var response: Dictionary = JSON.parse_string(response_buffer.substr(0, line_end))
	response_buffer = response_buffer.substr(line_end + 1)
	if not response.get("success", false):
		failed = true
		push_error(str(response.get("error", "Artificial driver rejected observation")))
		return {}
	return response["command"]

func disable_player_control(node: Node) -> void:
	var script: Script = node.get_script()
	if script != null and script.resource_path.contains("input"):
		node.set_process(false)
		node.set_physics_process(false)
		node.set_process_input(false)
		node.set_process_unhandled_input(false)
	for child in node.get_children():
		disable_player_control(child)

func run_fuji_validation() -> void:
	var repository_root := ProjectSettings.globalize_path("res://..")
	var package_path := argument_value("--package=", "")
	var output_directory := argument_value("--output=", repository_root.path_join("scratch/race_intelligence_correction_20261014"))
	var target_laps := int(argument_value("--laps=", "1"))
	var maximum_seconds := float(argument_value("--maximum-seconds=", "600"))
	var port_number := int(argument_value("--port=", "39194"))
	connection.connect_to_host("127.0.0.1", port_number)
	var deadline_milliseconds := Time.get_ticks_msec() + 10000
	while connection.get_status() != StreamPeerTCP.STATUS_CONNECTED:
		connection.poll()
		if Time.get_ticks_msec() > deadline_milliseconds:
			push_error("Cannot connect to Rust artificial driver bridge")
			quit(1)
			return
		await process_frame
	connection.set_no_delay(true)
	var compositor: Node = load("res://scenes/runtime/vehicle_test_session.tscn").instantiate()
	root.add_child(compositor)
	disable_player_control(compositor)
	var vehicle: Node3D = compositor.find_child("VehicleRigidBody", true, false)
	var presentation_core: Node = compositor.get_node("F90Core")
	vehicle.set_physical_world_controlled(true)
	world_interface = ClassDB.instantiate("PhysicalVehicleWorldInterface")
	var physics_library_path := argument_value("--physics-library=", ProjectSettings.globalize_path("res://addons/formula90s/bin/vehicle_physics_engine.windows.template_debug.x86_64.dll"))
	if not world_interface.initialize_library(physics_library_path):
		push_error("Physical world binary failed BUILD validation")
		quit(1)
		return
	var resolved := request_world({"operation": "resolve_vehicle_profile", "interface_version": 1,
		"repository_root": repository_root,
		"vehicle_profile_path": ProjectSettings.globalize_path("res://data/vehicles/f1_2030/profiles/geometric/vehicle.json")})
	if failed:
		quit(1)
		return
	var configuration: Dictionary = resolved["resolved_vehicle_configuration"]["configuration"]
	var tuning: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://data/race_intelligence/automatic_transmission_tuning.json"))["artificial_driver_transmission"]
	configuration["powertrain"]["automatic_transmission"] = tuning["automatic_transmission"]
	configuration["powertrain"]["automatic_shift"] = tuning["automatic_shift"]
	var created := request_world({"operation": "create_world", "interface_version": 1,
		"repository_root": repository_root, "physical_package_path": package_path,
		"vehicle_profile": configuration})
	if failed:
		quit(1)
		return
	world_identifier = int(created["world_identifier"])
	var racing_line: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://tracks/fuji76_77/metadata/racing_line.json"))
	var first_position: Array = racing_line["waypoints"][0]["pos"]
	var next_position: Array = racing_line["waypoints"][1]["pos"]
	var spawn_heading := atan2(-(float(next_position[0]) - float(first_position[0])), -(float(next_position[2]) - float(first_position[2])))
	var registered := request_world({"operation": "register_vehicle", "vehicle_profile": configuration,
		"position_world_metres": {"x": first_position[0], "y": float(first_position[1]) + 0.35, "z": first_position[2]},
		"yaw_radians": spawn_heading})
	if failed:
		quit(1)
		return
	var entity_identifier := int(registered["entity_identifier"])
	var snapshots: Array = request_world({"operation": "snapshots"})["snapshots"]
	var telemetry := FileAccess.open(output_directory.path_join("fuji_telemetry.csv"), FileAccess.WRITE)
	var physical_diagnostics := FileAccess.open(output_directory.path_join("physical_diagnostics.jsonl"), FileAccess.WRITE)
	telemetry.store_line("time_seconds,arc_metres,lateral_metres,speed_metres_per_second,target_speed_metres_per_second,steering,throttle,brake,gear,height_metres,engine_revolutions_per_minute,planned_acceleration_metres_per_second_squared,recovery_speed_fraction,available_half_width_metres")
	var simulation_seconds := 0.0
	var duration_seconds := 1.0 / 60.0
	var previous_arc_metres := float(racing_line["waypoints"][0]["lap_distance_m"])
	var travelled_metres := 0.0
	var maximum_lateral_metres := 0.0
	var maximum_speed_metres_per_second := 0.0
	var off_track_ticks := 0
	var tick_index := 0
	var start_milliseconds := Time.get_ticks_msec()
	var failure_reason := ""
	var last_progress_seconds := 0.0
	var captured_image := false
	var driver_wait_microseconds: int = 0
	var world_advance_microseconds: int = 0
	var presentation_microseconds: int = 0
	var full_throttle_ticks := 0
	var braking_ticks := 0
	var recovery_ticks := 0
	while simulation_seconds < maximum_seconds and travelled_metres < float(racing_line["lap_length_m"]) * target_laps and not failed:
		var snapshot: Dictionary = snapshots[0]
		var suspension: Dictionary = snapshot["state"]["suspension"]
		var position: Dictionary = suspension["body_origin_world_metres"]
		var orientation: Dictionary = suspension["body_orientation_world"]
		var rotation := Quaternion(float(orientation["x"]), float(orientation["y"]), float(orientation["z"]), float(orientation["w"]))
		var forward: Vector3 = rotation * Vector3.FORWARD
		var speed_metres_per_second := float(snapshot["telemetry"]["speed_kmh"]) / 3.6
		var operation_start_microseconds := Time.get_ticks_usec()
		physical_diagnostics.store_line(JSON.stringify({"time_seconds": simulation_seconds, "snapshot": snapshot}))
		physical_diagnostics.flush()
		var command := await request_driver({"position_x_metres": position["x"], "position_z_metres": position["z"],
			"heading_radians": atan2(-forward.x, -forward.z), "speed_metres_per_second": maxf(speed_metres_per_second, 0.0),
			"duration_seconds": duration_seconds, "current_gear": int(snapshot["telemetry"]["gear"])})
		driver_wait_microseconds += Time.get_ticks_usec() - operation_start_microseconds
		if failed:
			break
		var arc_metres := float(command["arc_length_metres"])
		if float(command["throttle_fraction"]) >= 0.95:
			full_throttle_ticks += 1
		if float(command["brake_fraction"]) > 0.01:
			braking_ticks += 1
		if float(command["recovery_speed_fraction"]) < 0.999:
			recovery_ticks += 1
		var progress_metres := arc_metres - previous_arc_metres
		if progress_metres < -float(racing_line["lap_length_m"]) * 0.5:
			progress_metres += float(racing_line["lap_length_m"])
		if absf(progress_metres) > 5.0:
			failure_reason = "Discontinuous racing line projection"
			failed = true
			break
		travelled_metres += progress_metres
		if progress_metres > 0.01:
			last_progress_seconds = simulation_seconds
		previous_arc_metres = arc_metres
		maximum_lateral_metres = maxf(maximum_lateral_metres, absf(float(command["lateral_offset_metres"])))
		maximum_speed_metres_per_second = maxf(maximum_speed_metres_per_second, speed_metres_per_second)
		if absf(float(command["lateral_offset_metres"])) + float(command["vehicle_half_width_metres"]) > float(command["available_half_width_metres"]):
			off_track_ticks += 1
			failure_reason = "Vehicle footprint exceeded track width"
			failed = true
			break
		if simulation_seconds - last_progress_seconds > 15.0 or not is_finite(float(position["y"])):
			failure_reason = "Vehicle stalled or produced invalid physical state"
			failed = true
			break
		request_world({"operation": "submit_input", "entity_identifier": entity_identifier,
			"sample": {"time_seconds": simulation_seconds, "driving_aids_mask": snapshot["driving_aids_mask"],
			"input": {"steering": command["steering_fraction"], "throttle": command["throttle_fraction"],
				"brake": command["brake_fraction"], "clutch": 0.0, "handbrake": 0.0, "gear_request": command["gear_request"]}}})
		operation_start_microseconds = Time.get_ticks_usec()
		var advanced := request_world({"operation": "advance", "duration_seconds": duration_seconds})
		world_advance_microseconds += Time.get_ticks_usec() - operation_start_microseconds
		if failed:
			break
		snapshots = advanced["snapshots"]
		simulation_seconds = float(advanced["time_seconds"]) + float(advanced["unconsumed_host_time_seconds"])
		operation_start_microseconds = Time.get_ticks_usec()
		if not presentation_core.accept_physical_world_snapshot(vehicle, snapshots[0]):
			failure_reason = "Godot presentation rejected authoritative physical snapshot"
			failed = true
			break
		presentation_microseconds += Time.get_ticks_usec() - operation_start_microseconds
		if tick_index % 6 == 0:
			telemetry.store_line("%f,%f,%f,%f,%f,%f,%f,%f,%d,%f,%f,%f,%f,%f" % [simulation_seconds, arc_metres, command["lateral_offset_metres"], speed_metres_per_second, command["target_speed_metres_per_second"], command["steering_fraction"], command["throttle_fraction"], command["brake_fraction"], snapshot["telemetry"]["gear"], position["y"], snapshot["telemetry"]["rpm"], command["planned_acceleration_metres_per_second_squared"], command["recovery_speed_fraction"], command["available_half_width_metres"]])
		if tick_index % 600 == 0:
			print("FUJI_DRIVER_PROGRESS time=%0.1f distance=%0.1f lateral=%0.2f speed=%0.1f" % [simulation_seconds, travelled_metres, command["lateral_offset_metres"], speed_metres_per_second])
		tick_index += 1
		if tick_index % 60 == 0:
			telemetry.flush()
			await process_frame
		if not captured_image and simulation_seconds >= 20.0 and DisplayServer.get_name() != "headless":
			await RenderingServer.frame_post_draw
			root.get_texture().get_image().save_png(output_directory.path_join("fuji_driver.png"))
			captured_image = true
	telemetry.close()
	var completed_laps := floori(travelled_metres / float(racing_line["lap_length_m"]))
	var passed := not failed and completed_laps >= target_laps and off_track_ticks == 0
	if not passed and failure_reason.is_empty():
		failure_reason = "Driver or physical world request failed" if failed else "Simulation time limit reached before requested laps"
	var report := {"passed": passed, "failure_reason": failure_reason, "completed_laps": completed_laps,
		"travelled_metres": travelled_metres, "simulated_seconds": simulation_seconds,
		"maximum_lateral_metres": maximum_lateral_metres, "maximum_speed_metres_per_second": maximum_speed_metres_per_second,
		"off_track_ticks": off_track_ticks, "wall_seconds": (Time.get_ticks_msec() - start_milliseconds) / 1000.0,
		"physics_source": "source-verified Fuji collision package and CoupledVehicleWorld", "driver_source": "Rust autonomous_driver_bridge",
		"build_source": FileAccess.get_file_as_string("res://BUILD_SOURCE").strip_edges(),
		"physics_library_path": physics_library_path, "rendering_backend": DisplayServer.get_name(),
		"requested_laps": target_laps, "physical_frequency_hertz": configuration["coupled_world"]["physical_frequency_hertz"],
		"control_frequency_hertz": 60, "driver_wait_seconds": driver_wait_microseconds / 1000000.0,
		"world_advance_seconds": world_advance_microseconds / 1000000.0,
		"presentation_seconds": presentation_microseconds / 1000000.0,
		"control_ticks": tick_index, "full_throttle_ticks": full_throttle_ticks,
		"braking_ticks": braking_ticks, "recovery_ticks": recovery_ticks,
		"driver_profile_model": "vehicle-derived nominal grip, aero and brake envelope"}
	FileAccess.open(output_directory.path_join("fuji_report.json"), FileAccess.WRITE).store_string(JSON.stringify(report, "\t"))
	print("FUJI_AUTONOMOUS_DRIVER_RESULT ", JSON.stringify(report))
	request_world({"operation": "destroy_world"})
	world_interface.close_library()
	connection.disconnect_from_host()
	quit(0 if passed else 1)
