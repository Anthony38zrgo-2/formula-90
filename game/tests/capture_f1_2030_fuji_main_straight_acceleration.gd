extends SceneTree

const SESSION_SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"
const RACING_LINE_PATH := "res://tracks/fuji76_77/metadata/racing_line.json"
const EXPECTED_TRACK_ID := "fuji76_77"
const START_WAYPOINT := 650
const WRAP_END_WAYPOINT := 818
const EXIT_WAYPOINT := 75
const SPAWN_HEIGHT_M := 0.35
const WHEELBASE_M := 2.95
const MAX_STEERING_ANGLE_RAD := 0.48
const UPSHIFT_REVOLUTIONS_PER_MINUTE := 17600.0
const CAPTURE_SECONDS := 30.0
const LATERAL_LIMIT_M := 8.0
const STRAIGHT_END_MARGIN_M := 20.0

func _init() -> void:
	call_deferred("_run")

func _capture_output_path() -> String:
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--capture-output="):
			return argument.trim_prefix("--capture-output=")
	return ""

func _run() -> void:
	var output_path := _capture_output_path()
	if output_path.is_empty():
		printerr("[FAIL] --capture-output=<absolute path> is required")
		quit(1)
		return
	var packed := load(SESSION_SCENE_PATH) as PackedScene
	if packed == null:
		printerr("[FAIL] could not load %s" % SESSION_SCENE_PATH)
		quit(1)
		return
	var session := packed.instantiate()
	root.add_child(session)
	current_scene = session
	for _frame in 10:
		await process_frame

	var race_session := session.get_node_or_null("WorldViewport/RaceSession")
	if race_session == null:
		printerr("[FAIL] RaceSession was not found")
		quit(1)
		return
	var active_track: Node3D = race_session.get("active_track")
	var vehicle: Node3D = race_session.get("active_vehicle")
	if active_track == null or vehicle == null:
		printerr("[FAIL] Fuji track or F1 2030 vehicle did not compose")
		quit(1)
		return

	var route_points: Array[Vector3] = []
	var route_stations: Array[float] = []
	if not _build_route(route_points, route_stations):
		printerr("[FAIL] could not build the Fuji main-straight route")
		quit(1)
		return

	var world_start := active_track.to_global(route_points[0])
	var world_next := active_track.to_global(route_points[1])
	var direction := world_next - world_start
	var yaw := atan2(-direction.x, -direction.z)
	if vehicle.has_method("reset_vehicle"):
		vehicle.call("reset_vehicle", world_start + Vector3.UP * SPAWN_HEIGHT_M, yaw)
	vehicle.set("enable_player_input", false)
	if vehicle.has_method("set_automatic_transmission"):
		vehicle.set("automatic_transmission", false)
	vehicle.call("set_throttle_amount", 0.0)
	vehicle.call("set_brake_amount", 0.16)
	vehicle.call("set_steering_input", 0.0)

	var file := FileAccess.open(output_path, FileAccess.WRITE)
	if file == null:
		printerr("[FAIL] could not open capture output %s" % output_path)
		quit(1)
		return
	file.store_line("time_s,speed_kmh,rpm,gear,lateral_m,station_m")

	var route_end_station: float = route_stations.back()
	var elapsed_s := 0.0
	var last_in_bounds_station := -1.0
	var last_in_bounds_speed := 0.0
	var last_in_bounds_gear := 0
	var maximum_speed := 0.0
	var maximum_speed_station := -1.0
	while elapsed_s < CAPTURE_SECONDS:
		var local_position := active_track.to_local(vehicle.global_position)
		var nearest := _nearest_route_index(route_points, local_position)
		var station: float = route_stations[nearest]
		var lateral_distance := _lateral_offset(route_points, nearest, local_position)
		var speed_before_step := absf(float(vehicle.call("get_speed_kmh")))
		vehicle.call("set_throttle_amount", 1.0 if elapsed_s >= 3.0 else 0.0)
		vehicle.call("set_brake_amount", 0.0 if elapsed_s >= 3.0 else 0.16)
		vehicle.call("set_steering_input", _steering_for_route(route_points, route_stations, nearest, vehicle, active_track, speed_before_step))
		var current_gear := int(vehicle.call("get_current_gear"))
		if current_gear > 0 and current_gear < 7:
			if float(vehicle.call("get_motor_rpm")) >= UPSHIFT_REVOLUTIONS_PER_MINUTE:
				vehicle.call("set_gear_request", current_gear + 1)
		await physics_frame
		elapsed_s += 1.0 / 120.0
		var speed_kmh := absf(float(vehicle.call("get_speed_kmh")))
		var current_gear_after := int(vehicle.call("get_current_gear"))
		var revolutions_per_minute := float(vehicle.call("get_motor_rpm"))
		file.store_line("%.4f,%.3f,%.1f,%d,%.3f,%.3f" % [
			elapsed_s, speed_kmh, revolutions_per_minute, current_gear_after, lateral_distance, station
		])
		if lateral_distance <= LATERAL_LIMIT_M:
			last_in_bounds_station = station
			last_in_bounds_speed = speed_kmh
			last_in_bounds_gear = current_gear_after
		if speed_kmh > maximum_speed:
			maximum_speed = speed_kmh
			maximum_speed_station = station
		if station >= route_end_station - STRAIGHT_END_MARGIN_M:
			break
	file.close()
	print("[CAPTURE] last_in_bounds_station=%.1fm speed=%.2fkm/h gear=%d maximum_speed=%.2fkm/h at %.1fm" % [
		last_in_bounds_station, last_in_bounds_speed, last_in_bounds_gear, maximum_speed, maximum_speed_station
	])
	print("[CAPTURE] route_length=%.1fm reached=%.1fm output=%s" % [
		route_end_station, last_in_bounds_station, output_path
	])
	quit(0)

func _build_route(route_points: Array[Vector3], route_stations: Array[float]) -> bool:
	var file := FileAccess.open(RACING_LINE_PATH, FileAccess.READ)
	if file == null:
		return false
	var parsed: Variant = JSON.parse_string(file.get_as_text())
	file.close()
	if not parsed is Dictionary or str(parsed.get("track_id", "")) != EXPECTED_TRACK_ID:
		return false
	var waypoints: Variant = parsed.get("waypoints", [])
	if not waypoints is Array or waypoints.size() <= WRAP_END_WAYPOINT:
		return false
	var base_points: Array[Vector3] = []
	for waypoint_index in range(START_WAYPOINT, WRAP_END_WAYPOINT + 1):
		var waypoint: Variant = waypoints[waypoint_index]
		if not waypoint is Dictionary:
			return false
		var position: Variant = waypoint.get("pos", null)
		if not position is Array or position.size() < 3:
			return false
		base_points.append(Vector3(float(position[0]), float(position[1]), float(position[2])))
	for waypoint_index in range(0, EXIT_WAYPOINT + 1):
		var waypoint: Variant = waypoints[waypoint_index]
		if not waypoint is Dictionary:
			return false
		var position: Variant = waypoint.get("pos", null)
		if not position is Array or position.size() < 3:
			return false
		base_points.append(Vector3(float(position[0]), float(position[1]), float(position[2])))
	if base_points.size() < 3:
		return false
	route_points.clear()
	route_stations.clear()
	var station := 0.0
	for point_index in range(base_points.size()):
		if point_index > 0:
			var segment: Vector3 = base_points[point_index] - base_points[point_index - 1]
			station += Vector2(segment.x, segment.z).length()
		route_points.append(base_points[point_index])
		route_stations.append(station)
	return route_stations.back() > 1000.0

func _nearest_route_index(route_points: Array[Vector3], local_position: Vector3) -> int:
	var best_index := 0
	var best_distance_squared := INF
	for point_index in range(route_points.size()):
		var point: Vector3 = route_points[point_index]
		var delta := Vector2(local_position.x - point.x, local_position.z - point.z)
		var distance_squared := delta.length_squared()
		if distance_squared < best_distance_squared:
			best_distance_squared = distance_squared
			best_index = point_index
	return best_index

func _lateral_offset(route_points: Array[Vector3], point_index: int, local_position: Vector3) -> float:
	var previous: Vector3 = route_points[maxi(0, point_index - 1)]
	var next: Vector3 = route_points[mini(route_points.size() - 1, point_index + 1)]
	var tangent := Vector3(next.x - previous.x, 0.0, next.z - previous.z).normalized()
	if tangent.length_squared() < 0.5:
		return 0.0
	var right := Vector3.UP.cross(tangent).normalized()
	var to_position := local_position - route_points[point_index]
	return to_position.dot(right)

func _steering_for_route(route_points: Array[Vector3], route_stations: Array[float], point_index: int, vehicle: Node3D, active_track: Node3D, speed_kmh: float) -> float:
	var speed_ms := speed_kmh / 3.6
	var lookahead_m := clampf(5.0 + speed_ms * 0.38, 5.0, 17.0)
	var target_point := _route_position_at(route_points, route_stations, route_stations[point_index] + lookahead_m)
	var target_world := active_track.to_global(target_point)
	var local_target := vehicle.global_basis.inverse() * (target_world - vehicle.global_position)
	var distance := Vector2(local_target.x, local_target.z).length()
	if distance < 0.1:
		return 0.0
	var alpha := atan2(-local_target.x, -local_target.z)
	var steering_angle := atan((2.0 * WHEELBASE_M * sin(alpha)) / distance)
	return clampf(steering_angle / MAX_STEERING_ANGLE_RAD, -1.0, 1.0)

func _route_position_at(route_points: Array[Vector3], route_stations: Array[float], station: float) -> Vector3:
	if station <= 0.0:
		return route_points[0]
	if station >= route_stations.back():
		return route_points.back()
	var index := 0
	while index < route_stations.size() - 2 and route_stations[index + 1] < station:
		index += 1
	var span := maxf(0.001, route_stations[index + 1] - route_stations[index])
	var weight := clampf((station - route_stations[index]) / span, 0.0, 1.0)
	return route_points[index].lerp(route_points[index + 1], weight)
