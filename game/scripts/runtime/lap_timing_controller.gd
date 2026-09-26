class_name LapTimingController
extends Node

signal lap_started(lap_number: int)
signal lap_completed(lap_number: int, lap_time_seconds: float, is_best: bool)

const DEFAULT_HALF_WIDTH_M := 10.0
const MINIMUM_PLAUSIBLE_STEP_M := 25.0
const MINIMUM_LAP_DISTANCE_RATIO := 0.5
const MINIMUM_LAP_DISTANCE_FLOOR_M := 200.0
const INSTANT_CONSUMPTION_TIME_CONSTANT_S := 2.0
const MINIMUM_INSTANT_RATE_FRACTION := 0.05
const MAXIMUM_LAPS_DELTA_MAGNITUDE := 99.0

var vehicle: Node3D
var is_configured := false
var lap_count := 0
var current_lap_number := 0
var current_lap_time_seconds := 0.0
var last_lap_time_seconds := -1.0
var best_lap_time_seconds := -1.0
var total_time_seconds := 0.0
var average_consumption_kg_per_lap := 0.0
var has_consumption_average := false
var instant_consumption_kg_per_lap := 0.0
var has_instant_consumption := false
var laps_delta := 0.0
var has_laps_delta := false
var planned_laps := 0.0

var _estimated_lap_consumption_kg := 0.0
var _reference_lap_time_seconds := 0.0
var _smoothed_consumption_rate_kg_per_s := 0.0
var _previous_frame_fuel_kg := -1.0
var _fuel_at_lap_start_kg := -1.0
var _previous_lap_fuel_kg := -1.0
var _total_consumed_kg := 0.0

var _start_finish_position := Vector3.ZERO
var _start_finish_forward := Vector3.FORWARD
var _start_finish_half_width_m := DEFAULT_HALF_WIDTH_M
var _minimum_lap_distance_m := MINIMUM_LAP_DISTANCE_FLOOR_M

var _has_previous_sample := false
var _previous_position := Vector3.ZERO
var _previous_signed_distance := 0.0
var _distance_since_crossing_m := 0.0
var _lap_started := false

func configure(
		next_vehicle: Node3D,
		track_definition: TrackDefinition,
		physics_config_path: String = "") -> bool:
	vehicle = next_vehicle
	is_configured = false
	if vehicle == null or track_definition == null:
		return false
	var fuel_plan := _load_fuel_plan(physics_config_path)
	_estimated_lap_consumption_kg = float(fuel_plan.get("estimated_lap_consumption_kg", 0.0))
	_reference_lap_time_seconds = float(fuel_plan.get("reference_lap_time_s", 0.0))
	var start_finish := track_definition.load_start_finish_data()
	if start_finish.is_empty():
		push_warning("LapTimingController: el circuito no declara start_finish.")
		return false
	var line_position: Variant = start_finish.get("position_m")
	var line_forward: Variant = start_finish.get("forward")
	if not (line_position is Array) or not (line_forward is Array):
		push_warning("LapTimingController: start_finish incompleto.")
		return false
	_start_finish_position = _to_vector3(line_position)
	_start_finish_forward = _to_vector3(line_forward).normalized()
	if _start_finish_forward.length_squared() <= 0.0:
		push_warning("LapTimingController: start_finish.forward es nulo.")
		return false
	_start_finish_half_width_m = maxf(float(start_finish.get("half_width_m", DEFAULT_HALF_WIDTH_M)), 0.0)
	var lap_length_m := track_definition.load_lap_length_m()
	_minimum_lap_distance_m = maxf(
		lap_length_m * MINIMUM_LAP_DISTANCE_RATIO,
		MINIMUM_LAP_DISTANCE_FLOOR_M)
	reset_timing()
	is_configured = true
	return true

func reset_timing() -> void:
	lap_count = 0
	current_lap_number = 0
	current_lap_time_seconds = 0.0
	last_lap_time_seconds = -1.0
	best_lap_time_seconds = -1.0
	total_time_seconds = 0.0
	average_consumption_kg_per_lap = 0.0
	has_consumption_average = false
	instant_consumption_kg_per_lap = 0.0
	has_instant_consumption = false
	laps_delta = 0.0
	has_laps_delta = false
	planned_laps = 0.0
	_smoothed_consumption_rate_kg_per_s = 0.0
	_previous_frame_fuel_kg = -1.0
	_fuel_at_lap_start_kg = -1.0
	_previous_lap_fuel_kg = -1.0
	_total_consumed_kg = 0.0
	_has_previous_sample = false
	_distance_since_crossing_m = 0.0
	_lap_started = false

func _physics_process(delta: float) -> void:
	if not is_configured or vehicle == null:
		return
	sample_position(vehicle.global_position, delta)

func sample_position(current_position: Vector3, delta: float) -> void:
	if not is_configured:
		return
	if not _has_previous_sample:
		_previous_position = current_position
		_previous_signed_distance = _signed_distance_to_line(current_position)
		_has_previous_sample = true
		return

	var step_vector := current_position - _previous_position
	var step_distance_m := step_vector.length()
	if step_distance_m <= MINIMUM_PLAUSIBLE_STEP_M:
		_distance_since_crossing_m += step_distance_m

	var current_signed_distance := _signed_distance_to_line(current_position)
	if _is_valid_crossing(current_position, current_signed_distance):
		if not _lap_started or _distance_since_crossing_m >= _minimum_lap_distance_m:
			_register_finish_line_crossing()

	if _lap_started:
		current_lap_time_seconds += delta
		total_time_seconds += delta
		var remaining_kg := _update_instant_consumption(delta)
		_update_laps_delta(remaining_kg)

	_previous_position = current_position
	_previous_signed_distance = current_signed_distance

func format_lap_time(lap_time_seconds: float) -> String:
	if lap_time_seconds < 0.0:
		return "--:--.---"
	var total_milliseconds := int(round(lap_time_seconds * 1000.0))
	var minutes := total_milliseconds / 60000
	var seconds := (total_milliseconds / 1000) % 60
	var milliseconds := total_milliseconds % 1000
	return "%d:%02d.%03d" % [minutes, seconds, milliseconds]

func _register_finish_line_crossing() -> void:
	if not _lap_started:
		_lap_started = true
		lap_count = 0
		current_lap_number = 1
		current_lap_time_seconds = 0.0
		total_time_seconds = 0.0
		_distance_since_crossing_m = 0.0
		_begin_fuel_accounting()
		lap_started.emit(current_lap_number)
		return
	var completed_lap_time := current_lap_time_seconds
	last_lap_time_seconds = completed_lap_time
	var is_best := best_lap_time_seconds < 0.0 or completed_lap_time < best_lap_time_seconds
	if is_best:
		best_lap_time_seconds = completed_lap_time
	lap_count += 1
	_update_consumption_after_lap()
	lap_completed.emit(lap_count, completed_lap_time, is_best)
	current_lap_number = lap_count + 1
	current_lap_time_seconds = 0.0
	_distance_since_crossing_m = 0.0
	lap_started.emit(current_lap_number)

func _begin_fuel_accounting() -> void:
	_fuel_at_lap_start_kg = _sample_remaining_fuel_kg()
	_previous_lap_fuel_kg = _fuel_at_lap_start_kg
	_previous_frame_fuel_kg = _fuel_at_lap_start_kg
	_total_consumed_kg = 0.0
	average_consumption_kg_per_lap = 0.0
	has_consumption_average = false
	_smoothed_consumption_rate_kg_per_s = 0.0
	instant_consumption_kg_per_lap = 0.0
	has_instant_consumption = false
	laps_delta = 0.0
	has_laps_delta = false
	planned_laps = 0.0
	if _fuel_at_lap_start_kg > 0.0 and _estimated_lap_consumption_kg > 0.0:
		planned_laps = _fuel_at_lap_start_kg / _estimated_lap_consumption_kg

func _update_consumption_after_lap() -> void:
	var remaining_kg := _sample_remaining_fuel_kg()
	if remaining_kg < 0.0 or _previous_lap_fuel_kg < 0.0:
		has_consumption_average = false
		return
	if remaining_kg > _previous_lap_fuel_kg:
		_begin_fuel_accounting()
		return
	_total_consumed_kg += _previous_lap_fuel_kg - remaining_kg
	_previous_lap_fuel_kg = remaining_kg
	if lap_count <= 0:
		return
	average_consumption_kg_per_lap = _total_consumed_kg / float(lap_count)
	has_consumption_average = true

func _update_instant_consumption(delta: float) -> float:
	var remaining_kg := _sample_remaining_fuel_kg()
	if remaining_kg < 0.0:
		_smoothed_consumption_rate_kg_per_s = 0.0
		_previous_frame_fuel_kg = -1.0
		has_instant_consumption = false
		return -1.0
	if _previous_frame_fuel_kg < 0.0 or remaining_kg > _previous_frame_fuel_kg:
		_previous_frame_fuel_kg = remaining_kg
		_smoothed_consumption_rate_kg_per_s = 0.0
		has_instant_consumption = false
		return remaining_kg
	var frame_rate_kg_per_s := (
		(_previous_frame_fuel_kg - remaining_kg) / maxf(delta, 0.0001))
	_previous_frame_fuel_kg = remaining_kg
	var blend := 1.0 - exp(-delta / INSTANT_CONSUMPTION_TIME_CONSTANT_S)
	_smoothed_consumption_rate_kg_per_s = lerpf(
		_smoothed_consumption_rate_kg_per_s, frame_rate_kg_per_s, blend)
	has_instant_consumption = (
		_smoothed_consumption_rate_kg_per_s > _minimum_instant_rate_kg_per_s())
	return remaining_kg

func _minimum_instant_rate_kg_per_s() -> float:
	if _estimated_lap_consumption_kg <= 0.0 or _reference_lap_time_seconds <= 0.0:
		return 0.0
	return (
		_estimated_lap_consumption_kg / _reference_lap_time_seconds
		* MINIMUM_INSTANT_RATE_FRACTION)

func _update_laps_delta(remaining_kg: float) -> void:
	has_laps_delta = false
	if remaining_kg < 0.0 or not has_instant_consumption or planned_laps <= 0.0:
		return
	var projected_lap_seconds := _projected_lap_seconds()
	if projected_lap_seconds <= 0.0:
		return
	instant_consumption_kg_per_lap = (
		_smoothed_consumption_rate_kg_per_s * projected_lap_seconds)
	if instant_consumption_kg_per_lap <= 0.0:
		return
	var laps_remaining_on_current_rate := remaining_kg / instant_consumption_kg_per_lap
	var current_lap_progress := clampf(
		current_lap_time_seconds / projected_lap_seconds, 0.0, 1.0)
	var planned_laps_remaining := planned_laps - float(lap_count) - current_lap_progress
	laps_delta = clampf(
		laps_remaining_on_current_rate - planned_laps_remaining,
		-MAXIMUM_LAPS_DELTA_MAGNITUDE,
		MAXIMUM_LAPS_DELTA_MAGNITUDE)
	has_laps_delta = true

func _projected_lap_seconds() -> float:
	if last_lap_time_seconds > 0.0:
		return last_lap_time_seconds
	return _reference_lap_time_seconds

func _sample_remaining_fuel_kg() -> float:
	if vehicle == null or not vehicle.has_method(&"get_fuel_state_snapshot"):
		return -1.0
	var fuel_state_value: Variant = vehicle.call(&"get_fuel_state_snapshot")
	if not (fuel_state_value is Dictionary):
		return -1.0
	var remaining_value: Variant = fuel_state_value.get("remaining_kg")
	if remaining_value == null:
		return -1.0
	var remaining_kg := float(remaining_value)
	return remaining_kg if is_finite(remaining_kg) else -1.0

func _load_fuel_plan(physics_config_path: String) -> Dictionary:
	if physics_config_path.is_empty():
		return {}
	var file := FileAccess.open(physics_config_path, FileAccess.READ)
	if file == null:
		return {}
	var parsed: Variant = JSON.parse_string(file.get_as_text())
	if not (parsed is Dictionary):
		return {}
	var fuel_data: Variant = parsed.get("fuel", {})
	if not (fuel_data is Dictionary):
		return {}
	return {
		"estimated_lap_consumption_kg": maxf(
			float(fuel_data.get("estimated_lap_consumption_kg", 0.0)), 0.0),
		"reference_lap_time_s": maxf(
			float(fuel_data.get("reference_lap_time_s", 0.0)), 0.0),
	}

func _is_valid_crossing(crossing_position: Vector3, current_signed_distance: float) -> bool:
	if not (_previous_signed_distance < 0.0 and current_signed_distance >= 0.0):
		return false
	var perpendicular := (
		crossing_position
		- _start_finish_position
		- _start_finish_forward * current_signed_distance)
	perpendicular.y = 0.0
	return perpendicular.length() <= _start_finish_half_width_m

func _signed_distance_to_line(world_position: Vector3) -> float:
	return (world_position - _start_finish_position).dot(_start_finish_forward)

func _to_vector3(values: Array) -> Vector3:
	if values.size() < 3:
		return Vector3.ZERO
	return Vector3(float(values[0]), float(values[1]), float(values[2]))
