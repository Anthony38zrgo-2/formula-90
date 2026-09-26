class_name PitStopController
extends Node3D

signal pit_lane_entered
signal pit_lane_exited
signal selection_changed(selection: Dictionary)
signal service_started(plan: Dictionary)
signal service_progress(status: Dictionary)
signal service_completed

const PIT_STOP_CONTROLLER_GROUP := "pit_stop_controller"
const MARKER_RENDER_HEIGHT_M := 0.02

const SELECTION_FIELD_COMPOUND := 0
const SELECTION_FIELD_FUEL := 1

const SERVICE_PHASE_NONE := 0
const SERVICE_PHASE_TIRES := 1
const SERVICE_PHASE_FUEL := 2

var vehicle: Node3D
var track_definition: TrackDefinition
var vehicle_definition: VehicleDefinition
var rules: PitStopRules

var is_configured := false
var in_pit_lane := false
var service_phase := SERVICE_PHASE_NONE
var selection_field := SELECTION_FIELD_FUEL
var selection_confirmed := false
var assigned_box_index := 0
var selected_compound_index := 0
var fuel_target_laps := 15
var tire_seconds_remaining := 0.0
var fuel_seconds_remaining := 0.0
var service_fill_kg := 0.0
var service_target_kg := 0.0

var _compound_labels: Array[String] = []
var _boxes: Array = []
var _estimated_lap_consumption_kg := 0.0
var _fuel_capacity_kg := 0.0
var _maximum_fuel_laps := 1
var _minimum_fuel_laps := 1
var _pit_forward := Vector3.FORWARD
var _pit_right := Vector3.RIGHT
var _strip_center := Vector3.ZERO
var _strip_half_extents := Vector3.ZERO
var _stop_hold_elapsed := 0.0
var _requires_box_exit := false
var _markers_root: Node3D
var _marker_materials: Array[StandardMaterial3D] = []


func _ready() -> void:
	add_to_group(PIT_STOP_CONTROLLER_GROUP)


func configure(
		next_vehicle: Node3D,
		next_track_definition: TrackDefinition,
		next_vehicle_definition: VehicleDefinition,
		next_rules: PitStopRules) -> bool:
	is_configured = false
	_clear_markers()
	vehicle = next_vehicle
	track_definition = next_track_definition
	vehicle_definition = next_vehicle_definition
	rules = next_rules if next_rules != null else PitStopRules.new()
	if vehicle == null or track_definition == null or not track_definition.has_pit_lane():
		return false
	var pit_lane := track_definition.load_pit_lane_data()
	_pit_forward = _to_vector3(pit_lane.get("forward", [0.0, 0.0, -1.0]), Vector3.FORWARD)
	if _pit_forward.length_squared() <= 0.0:
		_pit_forward = Vector3.FORWARD
	_pit_forward = _pit_forward.normalized()
	_pit_right = _pit_forward.cross(Vector3.UP).normalized()
	if _pit_right.length_squared() <= 0.0:
		_pit_right = Vector3.RIGHT
	_boxes = _parse_boxes(pit_lane.get("boxes", []))
	if _boxes.is_empty():
		return false
	_strip_center = _to_vector3(pit_lane.get("strip_center_m", [0.0, 0.0, 0.0]), Vector3.ZERO)
	var strip_extents := _to_vector3(pit_lane.get("strip_half_extents_m", [0.0, 0.0, 0.0]), Vector3.ZERO)
	_strip_half_extents = Vector3(
		maxf(absf(strip_extents.x), 0.0),
		maxf(absf(strip_extents.y), 0.0),
		maxf(absf(strip_extents.z), 0.0))
	assigned_box_index = clampi(int(pit_lane.get("player_box_index", 0)), 0, _boxes.size() - 1)
	_compound_labels = _parse_compounds()
	selected_compound_index = 0
	var fuel_plan := vehicle_definition.load_fuel_plan() if vehicle_definition != null else {}
	_estimated_lap_consumption_kg = float(fuel_plan.get("estimated_lap_consumption_kg", 0.0))
	_fuel_capacity_kg = _read_fuel_capacity(fuel_plan)
	_minimum_fuel_laps = maxi(rules.minimum_fuel_laps, 1)
	_maximum_fuel_laps = _minimum_fuel_laps
	if _estimated_lap_consumption_kg > 0.0 and _fuel_capacity_kg > 0.0:
		_maximum_fuel_laps = maxi(int(floor(_fuel_capacity_kg / _estimated_lap_consumption_kg)), _minimum_fuel_laps)
	fuel_target_laps = clampi(rules.default_fuel_laps, _minimum_fuel_laps, _maximum_fuel_laps)
	_build_box_markers()
	is_configured = true
	return true


func _physics_process(delta: float) -> void:
	if not is_configured or vehicle == null:
		return
	var vehicle_position := vehicle.global_position
	var now_in_pit_lane := _is_inside_strip(vehicle_position)
	if now_in_pit_lane != in_pit_lane:
		in_pit_lane = now_in_pit_lane
		if in_pit_lane:
			pit_lane_entered.emit()
		else:
			pit_lane_exited.emit()
	if service_phase != SERVICE_PHASE_NONE:
		_advance_service(delta)
		service_progress.emit(get_service_status())
		return
	_handle_selection_input()
	var inside_assigned_box := _is_inside_assigned_box(vehicle_position)
	if _requires_box_exit:
		if not inside_assigned_box:
			_requires_box_exit = false
		_stop_hold_elapsed = 0.0
		return
	if inside_assigned_box and absf(_vehicle_speed_kmh()) <= rules.stop_speed_threshold_kmh:
		_stop_hold_elapsed += delta
		if _stop_hold_elapsed >= rules.stop_hold_seconds:
			_begin_service()
	else:
		_stop_hold_elapsed = 0.0


func is_selection_active() -> bool:
	return is_configured and (in_pit_lane or service_phase != SERVICE_PHASE_NONE)


func is_servicing() -> bool:
	return service_phase != SERVICE_PHASE_NONE


func get_assigned_box_index() -> int:
	return assigned_box_index


func get_box_count() -> int:
	return _boxes.size()


func get_minimum_fuel_laps() -> int:
	return _minimum_fuel_laps


func get_maximum_fuel_laps() -> int:
	return _maximum_fuel_laps


func get_fuel_target_kg() -> float:
	return minf(float(fuel_target_laps) * _estimated_lap_consumption_kg, _fuel_capacity_kg)


func get_compound_labels() -> Array[String]:
	return _compound_labels.duplicate()


func get_selected_compound_label() -> String:
	if _compound_labels.is_empty():
		return ""
	return _compound_labels[clampi(selected_compound_index, 0, _compound_labels.size() - 1)]


func get_selection() -> Dictionary:
	return {
		"field": selection_field,
		"confirmed": selection_confirmed,
		"compound_index": selected_compound_index,
		"compound_label": get_selected_compound_label(),
		"compound_labels": _compound_labels.duplicate(),
		"fuel_laps": fuel_target_laps,
		"fuel_target_kg": get_fuel_target_kg(),
		"minimum_fuel_laps": _minimum_fuel_laps,
		"maximum_fuel_laps": _maximum_fuel_laps,
	}


func get_service_status() -> Dictionary:
	return {
		"phase": service_phase,
		"tire_seconds_remaining": tire_seconds_remaining,
		"fuel_seconds_remaining": fuel_seconds_remaining,
		"fill_kg": service_fill_kg,
		"target_kg": service_target_kg,
		"target_laps": fuel_target_laps,
		"compound_label": get_selected_compound_label(),
	}


func set_selection_field(next_field: int) -> void:
	selection_field = SELECTION_FIELD_COMPOUND if next_field == SELECTION_FIELD_COMPOUND else SELECTION_FIELD_FUEL
	selection_confirmed = false
	selection_changed.emit(get_selection())


func set_fuel_target_laps(next_laps: int) -> void:
	fuel_target_laps = clampi(next_laps, _minimum_fuel_laps, _maximum_fuel_laps)
	selection_confirmed = false
	selection_changed.emit(get_selection())


func set_compound_index(next_index: int) -> void:
	if _compound_labels.is_empty():
		return
	selected_compound_index = clampi(next_index, 0, _compound_labels.size() - 1)
	selection_confirmed = false
	selection_changed.emit(get_selection())


func confirm_selection() -> void:
	selection_confirmed = true
	selection_changed.emit(get_selection())


func _handle_selection_input() -> void:
	if not in_pit_lane or not _is_inside_strip(vehicle.global_position):
		return
	if InputMap.has_action(InputBindings.PIT_FIELD_UP) and Input.is_action_just_pressed(InputBindings.PIT_FIELD_UP):
		set_selection_field(SELECTION_FIELD_COMPOUND)
	if InputMap.has_action(InputBindings.PIT_FIELD_DOWN) and Input.is_action_just_pressed(InputBindings.PIT_FIELD_DOWN):
		set_selection_field(SELECTION_FIELD_FUEL)
	if InputMap.has_action(InputBindings.PIT_VALUE_LEFT) and Input.is_action_just_pressed(InputBindings.PIT_VALUE_LEFT):
		_adjust_selected_value(-1)
	if InputMap.has_action(InputBindings.PIT_VALUE_RIGHT) and Input.is_action_just_pressed(InputBindings.PIT_VALUE_RIGHT):
		_adjust_selected_value(1)
	if InputMap.has_action(InputBindings.PIT_CONFIRM) and Input.is_action_just_pressed(InputBindings.PIT_CONFIRM):
		confirm_selection()


func _adjust_selected_value(direction: int) -> void:
	if selection_field == SELECTION_FIELD_FUEL:
		set_fuel_target_laps(fuel_target_laps + direction)
		return
	if _compound_labels.size() > 1:
		var next_index := posmod(selected_compound_index + direction, _compound_labels.size())
		set_compound_index(next_index)


func _begin_service() -> void:
	var current_fuel_kg := _read_fuel_state_kg()
	service_target_kg = minf(float(fuel_target_laps) * _estimated_lap_consumption_kg, _fuel_capacity_kg)
	service_fill_kg = maxf(service_target_kg - current_fuel_kg, 0.0)
	tire_seconds_remaining = rules.tire_change_seconds
	fuel_seconds_remaining = service_fill_kg / rules.refuel_rate_kg_per_s
	service_phase = SERVICE_PHASE_TIRES
	_stop_hold_elapsed = 0.0
	_lock_vehicle()
	service_started.emit({
		"compound_label": get_selected_compound_label(),
		"target_laps": fuel_target_laps,
		"target_kg": service_target_kg,
		"fill_kg": service_fill_kg,
		"tire_seconds": tire_seconds_remaining,
		"fuel_seconds": fuel_seconds_remaining,
		"assigned_box_index": assigned_box_index,
	})


func _advance_service(delta: float) -> void:
	if service_phase == SERVICE_PHASE_TIRES:
		tire_seconds_remaining = maxf(tire_seconds_remaining - delta, 0.0)
		if tire_seconds_remaining <= 0.0:
			if vehicle.has_method("replace_tires"):
				vehicle.call("replace_tires")
			service_phase = SERVICE_PHASE_FUEL
			if fuel_seconds_remaining <= 0.0:
				_apply_fuel_target()
		return
	if service_phase == SERVICE_PHASE_FUEL:
		fuel_seconds_remaining = maxf(fuel_seconds_remaining - delta, 0.0)
		if fuel_seconds_remaining <= 0.0:
			_apply_fuel_target()


func _apply_fuel_target() -> void:
	if service_fill_kg > 0.0 and vehicle.has_method("set_fuel_kg"):
		vehicle.call("set_fuel_kg", service_target_kg)
	service_phase = SERVICE_PHASE_NONE
	_requires_box_exit = true
	_release_vehicle()
	service_completed.emit()


func _lock_vehicle() -> void:
	if "enable_player_input" in vehicle:
		vehicle.set("enable_player_input", false)
	if vehicle.has_method("set_throttle_amount"):
		vehicle.call("set_throttle_amount", 0.0)
	if vehicle.has_method("set_steering_input"):
		vehicle.call("set_steering_input", 0.0)
	if vehicle.has_method("set_brake_amount"):
		vehicle.call("set_brake_amount", 1.0)
	if vehicle.has_method("set_handbrake_amount"):
		vehicle.call("set_handbrake_amount", 1.0)
	if vehicle.has_method("set_clutch_amount"):
		vehicle.call("set_clutch_amount", 0.0)
	if vehicle.has_method("set_gear_request"):
		vehicle.call("set_gear_request", 0)
	if vehicle is RigidBody3D:
		vehicle.linear_velocity = Vector3.ZERO
		vehicle.angular_velocity = Vector3.ZERO


func _release_vehicle() -> void:
	if vehicle.has_method("set_throttle_amount"):
		vehicle.call("set_throttle_amount", 0.0)
	if vehicle.has_method("set_steering_input"):
		vehicle.call("set_steering_input", 0.0)
	if vehicle.has_method("set_brake_amount"):
		vehicle.call("set_brake_amount", 0.0)
	if vehicle.has_method("set_handbrake_amount"):
		vehicle.call("set_handbrake_amount", 0.0)
	if vehicle.has_method("set_clutch_amount"):
		vehicle.call("set_clutch_amount", 0.0)
	if vehicle.has_method("set_gear_request"):
		vehicle.call("set_gear_request", 0)
	if "enable_player_input" in vehicle:
		vehicle.set("enable_player_input", true)


func _read_fuel_state_kg() -> float:
	if vehicle == null or not vehicle.has_method("get_fuel_state_snapshot"):
		return 0.0
	var snapshot_value: Variant = vehicle.call("get_fuel_state_snapshot")
	if not (snapshot_value is Dictionary):
		return 0.0
	var remaining := float((snapshot_value as Dictionary).get("remaining_kg", 0.0))
	return remaining if is_finite(remaining) else 0.0


func _read_fuel_capacity(fuel_plan: Dictionary) -> float:
	var capacity := float(fuel_plan.get("capacity_kg", 0.0))
	if capacity > 0.0:
		return capacity
	if vehicle != null and vehicle.has_method("get_fuel_state_snapshot"):
		var snapshot_value: Variant = vehicle.call("get_fuel_state_snapshot")
		if snapshot_value is Dictionary:
			capacity = float((snapshot_value as Dictionary).get("capacity_kg", 0.0))
	return maxf(capacity, 0.0)


func _vehicle_speed_kmh() -> float:
	if vehicle == null:
		return 0.0
	if vehicle.has_method("get_speed_kmh"):
		return float(vehicle.call("get_speed_kmh"))
	if "speed_kmh" in vehicle:
		return float(vehicle.get("speed_kmh"))
	return 0.0


func _parse_boxes(raw_boxes: Array) -> Array:
	var parsed: Array = []
	for raw_box in raw_boxes:
		if not (raw_box is Dictionary):
			continue
		var box: Dictionary = raw_box
		var center := _to_vector3(box.get("center_m", []), Vector3.ZERO)
		var half_width := maxf(float(box.get("half_width_m", 0.0)), 0.0)
		var half_length := maxf(float(box.get("half_length_m", 0.0)), 0.0)
		if half_width <= 0.0 or half_length <= 0.0:
			continue
		parsed.append({
			"index": int(box.get("index", parsed.size())),
			"center": center,
			"half_width": half_width,
			"half_length": half_length,
		})
	return parsed


func _parse_compounds() -> Array[String]:
	var labels: Array[String] = []
	for raw_label in rules.compound_labels:
		var label := str(raw_label).strip_edges()
		if not label.is_empty():
			labels.append(label)
	if labels.is_empty():
		labels.append("BLANDOS")
	return labels


func _is_inside_strip(world_position: Vector3) -> bool:
	var local := _to_pit_local(world_position, _strip_center)
	return absf(local.x) <= _strip_half_extents.x and absf(local.y) <= _strip_half_extents.z


func _is_inside_assigned_box(world_position: Vector3) -> bool:
	if _boxes.is_empty():
		return false
	var box: Dictionary = _boxes[clampi(assigned_box_index, 0, _boxes.size() - 1)]
	var local := _to_pit_local(world_position, box.get("center", Vector3.ZERO))
	return absf(local.x) <= float(box.get("half_width", 0.0)) \
		and absf(local.y) <= float(box.get("half_length", 0.0))


func _to_pit_local(world_position: Vector3, origin: Vector3) -> Vector2:
	var offset := world_position - origin
	return Vector2(offset.dot(_pit_right), offset.dot(_pit_forward))


func _to_vector3(raw_value: Variant, fallback: Vector3) -> Vector3:
	if raw_value is Array and (raw_value as Array).size() >= 3:
		var values: Array = raw_value
		return Vector3(float(values[0]), float(values[1]), float(values[2]))
	if raw_value is Vector3:
		return raw_value
	return fallback


func _build_box_markers() -> void:
	_markers_root = Node3D.new()
	_markers_root.name = "PitBoxMarkers"
	add_child(_markers_root)
	var yaw := atan2(_pit_forward.x, -_pit_forward.z)
	for box in _boxes:
		var half_width := float(box.get("half_width", 0.0))
		var half_length := float(box.get("half_length", 0.0))
		var mesh := PlaneMesh.new()
		mesh.size = Vector2(half_width * 2.0, half_length * 2.0)
		var material := StandardMaterial3D.new()
		material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
		material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		material.cull_mode = BaseMaterial3D.CULL_DISABLED
		material.albedo_color = Color(0.95, 0.95, 0.95, 0.22)
		var marker := MeshInstance3D.new()
		marker.name = "PitBoxMarker%02d" % int(box.get("index", 0))
		marker.mesh = mesh
		marker.material_override = material
		marker.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		marker.position = (box.get("center", Vector3.ZERO) as Vector3) + Vector3(0.0, MARKER_RENDER_HEIGHT_M, 0.0)
		marker.rotation.y = yaw
		_markers_root.add_child(marker)
		_marker_materials.append(material)
	_highlight_assigned_box_marker()


func _highlight_assigned_box_marker() -> void:
	if assigned_box_index < _marker_materials.size():
		_marker_materials[assigned_box_index].albedo_color = Color(1.0, 0.82, 0.16, 0.55)


func _clear_markers() -> void:
	if _markers_root != null:
		remove_child(_markers_root)
		_markers_root.queue_free()
	_markers_root = null
	_marker_materials.clear()
