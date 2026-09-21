extends Control

const RACING_LINE_PATH := "res://tracks/fuji76_77/metadata/racing_line.json"
const EXPECTED_TRACK_ID := "fuji76_77"
const START_WAYPOINT := 650
const WRAP_END_WAYPOINT := 818
const EXIT_WAYPOINT := 75

const CLIP_DURATION_S := 30.0
const IDLE_DURATION_S := 3.0
const SLALOM_START_M := 80.0
const SLALOM_LENGTH_M := 260.0
const SLALOM_AMPLITUDE_M := 3.2
const CAR_HALF_WIDTH_M := 1.0
const EDGE_MARGIN_M := 0.75
const SPAWN_HEIGHT_M := 0.35
const WHEELBASE_M := 2.95
const MAX_STEERING_ANGLE_RAD := 0.48
const UPSHIFT_RPM := 14200.0
const DOWNSHIFT_START_S := 22.5
const DOWNSHIFT_MAX_SPEED_KMH := [145.0, 180.0, 220.0, 255.0, 290.0]
const DOWNSHIFT_INTERVAL_S := 0.9

@onready var _session_ui: Control = $VehicleTestSession

var _race_session: RaceSession
var _active_track: Node3D
var _active_vehicle: Node3D
var _camera: Camera3D
var _route_points: Array[Vector3] = []
var _route_stations: Array[float] = []
var _route_cursor := 0
var _elapsed_s := 0.0
var _commanded_gear := 1
var _last_observed_gear := -99
var _next_downshift_s := DOWNSHIFT_START_S
var _is_ready := false
var _completion_logged := false


func _ready() -> void:
	var hud := _session_ui.get_node_or_null("DisplayAspect/DisplayStage/HudLayer/DebugHud") as Control
	if hud != null:
		hud.visible = false

	var world_viewport := _session_ui.get_node_or_null("WorldViewport") as SubViewport
	if world_viewport == null:
		push_error("Fuji showcase: WorldViewport was not found in the base session.")
		set_physics_process(false)
		return
	_race_session = world_viewport.get_node_or_null("RaceSession") as RaceSession
	if _race_session == null or _race_session.active_track == null or _race_session.active_vehicle == null:
		push_error("Fuji showcase: the base race session did not compose Fuji and the F1 2030 vehicle.")
		set_physics_process(false)
		return

	_active_track = _race_session.active_track
	_active_vehicle = _race_session.active_vehicle as Node3D
	if _active_vehicle == null:
		push_error("Fuji showcase: the active vehicle is not a Node3D.")
		set_physics_process(false)
		return

	var line_data := _load_racing_line()
	if line_data.is_empty() or not _build_route(line_data.get("waypoints", [])):
		push_error("Fuji showcase: failed to build a valid main-straight route from Fuji metadata.")
		set_physics_process(false)
		return

	_prepare_vehicle()
	_add_slalom_gates()
	_add_front_three_quarter_camera()
	_is_ready = true
	print("[FUJI_SHOWCASE] route=%.1fm, slalom=%.1f-%.1fm, duration=%.1fs" % [
		_route_stations.back(), SLALOM_START_M, SLALOM_START_M + SLALOM_LENGTH_M, CLIP_DURATION_S
	])


func _load_racing_line() -> Dictionary:
	var file := FileAccess.open(RACING_LINE_PATH, FileAccess.READ)
	if file == null:
		push_error("Fuji showcase: metadata not found: %s" % RACING_LINE_PATH)
		return {}
	var parsed: Variant = JSON.parse_string(file.get_as_text())
	file.close()
	if not parsed is Dictionary or str(parsed.get("track_id", "")) != EXPECTED_TRACK_ID:
		push_error("Fuji showcase: racing-line metadata has the wrong track identity or format.")
		return {}
	return parsed as Dictionary


func _build_route(waypoints: Variant) -> bool:
	if not waypoints is Array or waypoints.size() <= WRAP_END_WAYPOINT:
		return false

	var indices: Array[int] = []
	for index in range(START_WAYPOINT, WRAP_END_WAYPOINT + 1):
		indices.append(index)
	for index in range(0, EXIT_WAYPOINT + 1):
		indices.append(index)

	var base_points: Array[Vector3] = []
	var widths_left: Array[float] = []
	var widths_right: Array[float] = []
	for index in indices:
		var waypoint: Variant = waypoints[index]
		if not waypoint is Dictionary:
			return false
		var position: Variant = waypoint.get("pos", null)
		if not position is Array or position.size() < 3:
			return false
		base_points.append(Vector3(float(position[0]), float(position[1]), float(position[2])))
		widths_left.append(float(waypoint.get("width_left", 0.0)))
		widths_right.append(float(waypoint.get("width_right", 0.0)))

	if base_points.size() < 3:
		return false

	_route_stations.clear()
	_route_points.clear()
	var station := 0.0
	for index in range(base_points.size()):
		if index > 0:
			var segment := base_points[index] - base_points[index - 1]
			station += Vector2(segment.x, segment.z).length()
		_route_stations.append(station)

	for index in range(base_points.size()):
		var previous := base_points[maxi(0, index - 1)]
		var next := base_points[mini(base_points.size() - 1, index + 1)]
		var tangent := Vector3(next.x - previous.x, 0.0, next.z - previous.z).normalized()
		if tangent.length_squared() < 0.5:
			return false
		var right := Vector3.UP.cross(tangent).normalized()
		var offset := _slalom_offset(_route_stations[index])
		var available_right := maxf(0.0, widths_right[index] - CAR_HALF_WIDTH_M - EDGE_MARGIN_M)
		var available_left := maxf(0.0, widths_left[index] - CAR_HALF_WIDTH_M - EDGE_MARGIN_M)
		if offset >= 0.0:
			offset = minf(offset, available_right)
		else:
			offset = maxf(offset, -available_left)
		_route_points.append(base_points[index] + right * offset)

	return _route_stations.back() > 1000.0


func _slalom_offset(station: float) -> float:
	var progress := (station - SLALOM_START_M) / SLALOM_LENGTH_M
	if progress < 0.0 or progress > 1.0:
		return 0.0
	var edge := clampf(minf(progress, 1.0 - progress) * 8.0, 0.0, 1.0)
	var envelope := edge * edge * (3.0 - 2.0 * edge)
	return SLALOM_AMPLITUDE_M * envelope * sin(4.0 * PI * progress)


func _prepare_vehicle() -> void:
	if "enable_player_input" in _active_vehicle:
		_active_vehicle.set("enable_player_input", false)
	var local_start := _route_points[0]
	var local_next := _route_points[1]
	var world_start := _active_track.to_global(local_start)
	var world_next := _active_track.to_global(local_next)
	var direction := world_next - world_start
	var yaw := atan2(-direction.x, -direction.z)
	if _active_vehicle.has_method("reset_vehicle"):
		_active_vehicle.call("reset_vehicle", world_start + Vector3.UP * SPAWN_HEIGHT_M, yaw)
	else:
		_active_vehicle.global_position = world_start + Vector3.UP * SPAWN_HEIGHT_M
		_active_vehicle.rotation.y = yaw
	if _active_vehicle.has_method("set_automatic_transmission"):
		# The tuned F1 2030 profile deliberately defaults to manual; issue real
		# gear requests on a fixed showcase timeline to guarantee audible shifts.
		_active_vehicle.call("set_automatic_transmission", false)
	else:
		push_warning("Fuji showcase: vehicle has no transmission setter; gear requests may be unavailable.")
	_active_vehicle.call("set_throttle_amount", 0.0)
	_active_vehicle.call("set_brake_amount", 0.16)
	_active_vehicle.call("set_steering_input", 0.0)


func _add_slalom_gates() -> void:
	var gate_root := Node3D.new()
	gate_root.name = "FujiShowcaseSlalomGates"
	_race_session.add_child(gate_root)

	var cone_mesh := CylinderMesh.new()
	cone_mesh.top_radius = 0.025
	cone_mesh.bottom_radius = 0.22
	cone_mesh.height = 0.55
	cone_mesh.radial_segments = 12
	var orange := StandardMaterial3D.new()
	orange.albedo_color = Color(1.0, 0.24, 0.035)
	orange.roughness = 0.8
	var white := StandardMaterial3D.new()
	white.albedo_color = Color(0.92, 0.94, 0.96)
	white.roughness = 0.8

	for gate_index in range(4):
		var fraction := 0.125 + 0.25 * float(gate_index)
		var station := SLALOM_START_M + SLALOM_LENGTH_M * fraction
		var center := _route_position_at(station)
		var tangent := _route_tangent_at(station)
		var right := Vector3.UP.cross(tangent).normalized()
		for side_index in range(2):
			var cone := MeshInstance3D.new()
			cone.name = "Gate%02dCone%d" % [gate_index + 1, side_index + 1]
			cone.mesh = cone_mesh
			cone.material_override = orange if (gate_index + side_index) % 2 == 0 else white
			gate_root.add_child(cone)
			var local_position := center + right * (2.0 if side_index == 1 else -2.0) + Vector3.UP * 0.275
			cone.global_position = _active_track.to_global(local_position)


func _add_front_three_quarter_camera() -> void:
	_camera = Camera3D.new()
	_camera.name = "ShowcaseFrontThreeQuarterCamera"
	_camera.top_level = true
	_camera.fov = 54.0
	_camera.near = 0.1
	_camera.far = 2000.0
	_race_session.add_child(_camera)
	_camera.current = true
	_update_camera(1.0)


func _physics_process(delta: float) -> void:
	if not _is_ready:
		return
	_elapsed_s += delta
	_route_cursor = _find_nearest_route_index(_active_vehicle.global_position)
	var remaining_route_m: float = float(_route_stations.back()) - _route_stations[_route_cursor]
	var brakeable_route_m: float = maxf(0.0, remaining_route_m - 18.0)
	var route_speed_cap_kmh: float = sqrt(2.0 * 5.0 * brakeable_route_m) * 3.6
	var target_speed_kmh: float = minf(_target_speed_kmh(_elapsed_s), route_speed_cap_kmh)
	var speed_kmh := absf(float(_active_vehicle.call("get_speed_kmh")))
	var speed_error := target_speed_kmh - speed_kmh
	var throttle := clampf(speed_error / 72.0, 0.0, 1.0)
	var brake := clampf(-speed_error / 75.0, 0.0, 0.70)
	if _elapsed_s < IDLE_DURATION_S:
		throttle = 0.0
		brake = 0.16
	elif _elapsed_s >= CLIP_DURATION_S:
		throttle = 0.0
		brake = 0.48

	_update_showcase_gear(throttle, brake, speed_kmh)
	_active_vehicle.call("set_throttle_amount", throttle)
	_active_vehicle.call("set_brake_amount", brake)
	_active_vehicle.call("set_steering_input", _calculate_steering())
	if _elapsed_s >= CLIP_DURATION_S and not _completion_logged:
		_completion_logged = true
		print("[FUJI_SHOWCASE] 30-second replay reached; user visual/audio review pending.")


func _update_showcase_gear(throttle: float, brake: float, speed_kmh: float) -> void:
	if not _active_vehicle.has_method("get_current_gear"):
		return
	var actual_gear := int(_active_vehicle.call("get_current_gear"))
	if actual_gear != _last_observed_gear:
		_last_observed_gear = actual_gear
		print("[FUJI_SHOWCASE] actual_gear=%d at %.2fs speed=%.1fkm/h" % [
			actual_gear, _elapsed_s, speed_kmh
		])
	if actual_gear != _commanded_gear:
		return

	if _elapsed_s < DOWNSHIFT_START_S:
		if actual_gear < 6 and throttle > 0.55 and _active_vehicle.has_method("get_motor_rpm"):
			var rpm := float(_active_vehicle.call("get_motor_rpm"))
			if rpm >= UPSHIFT_RPM:
				_request_showcase_gear(actual_gear + 1)
		return

	if brake < 0.12 or _elapsed_s < _next_downshift_s or actual_gear <= 1:
		return
	var target_gear := actual_gear - 1
	var safe_downshift_speed := float(DOWNSHIFT_MAX_SPEED_KMH[target_gear - 1])
	if speed_kmh <= safe_downshift_speed:
		_request_showcase_gear(target_gear)
		_next_downshift_s = _elapsed_s + DOWNSHIFT_INTERVAL_S


func _request_showcase_gear(target_gear: int) -> void:
	_commanded_gear = target_gear
	_active_vehicle.call("set_gear_request", target_gear)
	print("[FUJI_SHOWCASE] gear_request=%d at %.2fs" % [target_gear, _elapsed_s])


func _process(delta: float) -> void:
	if _is_ready and _camera != null and is_instance_valid(_active_vehicle):
		_update_camera(delta)


func _target_speed_kmh(time_s: float) -> float:
	if time_s < IDLE_DURATION_S:
		return 0.0
	if time_s < 6.0:
		return lerpf(0.0, 90.0, inverse_lerp(IDLE_DURATION_S, 6.0, time_s))
	if time_s < 15.0:
		return 90.0
	if time_s < 22.0:
		return lerpf(90.0, 280.0, inverse_lerp(15.0, 22.0, time_s))
	return lerpf(280.0, 60.0, inverse_lerp(22.0, CLIP_DURATION_S, minf(time_s, CLIP_DURATION_S)))


func _calculate_steering() -> float:
	var speed_ms := absf(float(_active_vehicle.call("get_speed_kmh"))) / 3.6
	var lookahead_m := clampf(5.0 + speed_ms * 0.38, 5.0, 17.0)
	var station := _route_stations[_route_cursor] + lookahead_m
	var target_world := _active_track.to_global(_route_position_at(station))
	var local_target := _active_vehicle.global_basis.inverse() * (target_world - _active_vehicle.global_position)
	var distance := Vector2(local_target.x, local_target.z).length()
	if distance < 0.1:
		return 0.0
	var alpha := atan2(-local_target.x, -local_target.z)
	var steering_angle := atan((2.0 * WHEELBASE_M * sin(alpha)) / distance)
	return clampf(steering_angle / MAX_STEERING_ANGLE_RAD, -1.0, 1.0)


func _find_nearest_route_index(world_position: Vector3) -> int:
	var local_position := _active_track.to_local(world_position)
	var best_index := _route_cursor
	var best_distance_sq := INF
	var last_index := mini(_route_points.size() - 1, _route_cursor + 24)
	for index in range(_route_cursor, last_index + 1):
		var point := _route_points[index]
		var delta := Vector2(local_position.x - point.x, local_position.z - point.z)
		var distance_sq := delta.length_squared()
		if distance_sq < best_distance_sq:
			best_distance_sq = distance_sq
			best_index = index
	return best_index


func _route_position_at(station: float) -> Vector3:
	if station <= 0.0:
		return _route_points[0]
	if station >= _route_stations.back():
		return _route_points.back()
	var index := _segment_index_at(station)
	var span := maxf(0.001, _route_stations[index + 1] - _route_stations[index])
	var weight := clampf((station - _route_stations[index]) / span, 0.0, 1.0)
	return _route_points[index].lerp(_route_points[index + 1], weight)


func _route_tangent_at(station: float) -> Vector3:
	var index := _segment_index_at(station)
	return (_route_points[index + 1] - _route_points[index]).normalized()


func _segment_index_at(station: float) -> int:
	var index := 0
	while index < _route_stations.size() - 2 and _route_stations[index + 1] < station:
		index += 1
	return index


func _update_camera(delta: float) -> void:
	var basis := _active_vehicle.global_basis
	var desired_position := _active_vehicle.global_position + basis * Vector3(3.8, 2.0, -8.5)
	var focus := _active_vehicle.global_position + Vector3.UP * 0.75
	if delta > 0.0:
		_camera.global_position = _camera.global_position.lerp(desired_position, 1.0 - exp(-7.0 * delta))
	else:
		_camera.global_position = desired_position
	_camera.look_at(focus, Vector3.UP)
