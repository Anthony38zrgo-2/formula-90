class_name LapTimingController
extends Node

signal lap_started(lap_number: int)
signal lap_completed(lap_number: int, lap_time_seconds: float, is_best: bool)

const DEFAULT_HALF_WIDTH_M := 10.0
const MINIMUM_PLAUSIBLE_STEP_M := 25.0
const MINIMUM_LAP_DISTANCE_RATIO := 0.5
const MINIMUM_LAP_DISTANCE_FLOOR_M := 200.0

var vehicle: Node3D
var is_configured := false
var lap_count := 0
var current_lap_number := 0
var current_lap_time_seconds := 0.0
var last_lap_time_seconds := -1.0
var best_lap_time_seconds := -1.0
var total_time_seconds := 0.0

var _start_finish_position := Vector3.ZERO
var _start_finish_forward := Vector3.FORWARD
var _start_finish_half_width_m := DEFAULT_HALF_WIDTH_M
var _minimum_lap_distance_m := MINIMUM_LAP_DISTANCE_FLOOR_M

var _has_previous_sample := false
var _previous_position := Vector3.ZERO
var _previous_signed_distance := 0.0
var _distance_since_crossing_m := 0.0
var _lap_started := false

func configure(next_vehicle: Node3D, track_definition: TrackDefinition) -> bool:
	vehicle = next_vehicle
	is_configured = false
	if vehicle == null or track_definition == null:
		return false
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
		lap_started.emit(current_lap_number)
		return
	var completed_lap_time := current_lap_time_seconds
	last_lap_time_seconds = completed_lap_time
	var is_best := best_lap_time_seconds < 0.0 or completed_lap_time < best_lap_time_seconds
	if is_best:
		best_lap_time_seconds = completed_lap_time
	lap_count += 1
	lap_completed.emit(lap_count, completed_lap_time, is_best)
	current_lap_number = lap_count + 1
	current_lap_time_seconds = 0.0
	_distance_since_crossing_m = 0.0
	lap_started.emit(current_lap_number)

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
