extends SceneTree

const DEFAULT_SESSION_PATH := "res://scenes/runtime/williams_hockenheim_villenueve_fuji_test_session.tscn"
const SETTLE_PHYSICS_TICKS := 300
const PHASE_PHYSICS_TICKS := 30
const SAMPLED_TAIL_TICKS := 8
const SPEED_PHASES_KILOMETRES_PER_HOUR := [200.0, 250.0, 300.0]
const GROUND_QUERY_HEIGHT_METRES := 2.0
const GROUND_QUERY_DEPTH_METRES := 5.0

func _initialize() -> void:
	call_deferred("run_verification")

func run_verification() -> void:
	var session_path := session_path_argument()
	var report := {
		"scene": session_path,
		"environment": backend_environment(),
		"phases": [],
	}
	var session: Node = load(session_path).instantiate()
	root.add_child(session)
	current_scene = session
	var race_session: Node = session.get_node_or_null("WorldViewport/RaceSession")
	if race_session == null:
		push_error("Session does not expose WorldViewport/RaceSession")
		quit(1)
		return
	for composition_frame in range(90):
		if race_session.get("active_vehicle") != null:
			break
		await process_frame
	var vehicle: RigidBody3D = race_session.get("active_vehicle")
	if vehicle == null:
		push_error("Session did not compose a vehicle")
		quit(1)
		return
	vehicle.set("enable_player_input", false)
	for settle_tick in range(SETTLE_PHYSICS_TICKS):
		await physics_frame
	report["settled"] = collect_sample(vehicle)
	for speed_kmh in SPEED_PHASES_KILOMETRES_PER_HOUR:
		vehicle.linear_velocity = Vector3(0.0, 0.0, -float(speed_kmh) / 3.6)
		vehicle.angular_velocity = Vector3.ZERO
		var samples: Array = []
		for phase_tick in range(PHASE_PHYSICS_TICKS):
			await physics_frame
			samples.append(collect_sample(vehicle))
		report["phases"].append({
			"target_speed_kmh": speed_kmh,
			"tail_samples": samples.slice(samples.size() - SAMPLED_TAIL_TICKS),
		})
	print("AERO_FLOOR_BALANCE_VERIFICATION ", JSON.stringify(report, "\t"))
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--output="):
			var file := FileAccess.open(argument.trim_prefix("--output="), FileAccess.WRITE)
			if file != null:
				file.store_string(JSON.stringify(report, "\t"))
	quit(0)

func session_path_argument() -> String:
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--scene="):
			return argument.trim_prefix("--scene=")
	return DEFAULT_SESSION_PATH

func backend_environment() -> Dictionary:
	return {
		"aerodynamics": OS.get_environment("FORMULA90_AERODYNAMICS_BACKEND"),
		"suspension": OS.get_environment("FORMULA90_SUSPENSION_BACKEND"),
		"tires": OS.get_environment("FORMULA90_TIRE_BACKEND"),
		"slow_states": OS.get_environment("FORMULA90_SLOW_STATE_BACKEND"),
	}

func collect_sample(vehicle: RigidBody3D) -> Dictionary:
	var ground_height := ground_height_at(vehicle, vehicle.global_position)
	var probe_heights: Dictionary = {}
	for child in vehicle.get_children():
		var ray := child as RayCast3D
		if ray == null:
			continue
		if ray.name.begins_with("Underfloor") or ray.name.begins_with("Diffuser"):
			probe_heights[String(ray.name)] = {
				"local_y": ray.position.y,
				"height_above_ground_metres": ray.global_position.y - ground_height,
			}
	var snapshot: Dictionary = vehicle.call("get_underfloor_state_snapshot")
	return {
		"sim_time_seconds": vehicle.get("sim_time") if vehicle.get("sim_time") != null else null,
		"speed_kmh": vehicle.call("get_speed_kmh"),
		"origin_height_above_ground_metres": vehicle.global_position.y - ground_height,
		"probe_heights": probe_heights,
		"aero": snapshot["aero"],
		"underfloor_contact": {
			"valid_mask": snapshot["valid_mask"],
			"clearance_m": snapshot["clearance_m"],
			"total_normal_force_n": snapshot["total_normal_force_n"],
			"scrape_phase": snapshot["scrape_phase"],
			"rigid_normal_impulse_ns": snapshot["rigid_normal_impulse_ns"],
		},
		"normal_forces_newtons": Array(vehicle.call("get_normal_forces")),
		"compressions_millimetres": Array(vehicle.call("get_wheel_compressions")),
	}

func ground_height_at(vehicle: RigidBody3D, position: Vector3) -> float:
	var space_state := vehicle.get_world_3d().direct_space_state
	var query := PhysicsRayQueryParameters3D.create(
		position + Vector3.UP * GROUND_QUERY_HEIGHT_METRES,
		position + Vector3.DOWN * GROUND_QUERY_DEPTH_METRES,
		1,
		[vehicle.get_rid()]
	)
	var result := space_state.intersect_ray(query)
	return result["position"].y if result.has("position") else NAN
