extends GdUnitTestSuite

const LAP_TIMING_SCRIPT := preload("res://scripts/runtime/lap_timing_controller.gd")
const FUJI_TRACK_DEFINITION := "res://data/tracks/fuji76_77.tres"

var _crossing_position := Vector3(-294.278, 0.095, -275.19)
var _crossing_forward := Vector3(-0.0069, 0.0, -0.99998).normalized()


func test_fuji_metadata_declares_the_start_finish_line() -> void:
	var track_definition := load(FUJI_TRACK_DEFINITION) as TrackDefinition
	assert_object(track_definition).is_not_null()
	var start_finish := track_definition.load_start_finish_data()
	assert_bool(start_finish.is_empty()).is_false()
	var position: Array = start_finish.get("position_m", [])
	assert_float(position[2]).is_equal_approx(-275.19, 0.01)
	assert_float(float(start_finish.get("half_width_m", 0.0))).is_greater(1.0)
	assert_float(track_definition.load_lap_length_m()).is_equal_approx(4310.293945, 0.01)


func test_first_forward_crossing_starts_lap_one() -> void:
	var controller := _configured_controller()
	controller.sample_position(_behind_line(5.0), 0.1)
	controller.sample_position(_beyond_line(5.0), 0.1)

	assert_int(controller.lap_count).is_equal(0)
	assert_int(controller.current_lap_number).is_equal(1)
	assert_float(controller.last_lap_time_seconds).is_equal(-1.0)
	assert_float(controller.best_lap_time_seconds).is_equal(-1.0)


func test_completed_lap_records_time_and_best() -> void:
	var controller := _configured_controller()
	controller.sample_position(_behind_line(5.0), 0.1)
	controller.sample_position(_beyond_line(5.0), 0.1)
	_sample_accumulated_distance(controller, 2200.0)
	controller.sample_position(_behind_line(5.0), 0.1)
	controller.sample_position(_beyond_line(5.0), 0.1)

	assert_int(controller.lap_count).is_equal(1)
	assert_int(controller.current_lap_number).is_equal(2)
	assert_float(controller.last_lap_time_seconds).is_greater(0.0)
	assert_float(controller.best_lap_time_seconds).is_equal_approx(
		controller.last_lap_time_seconds, 0.0001)


func test_reverse_and_out_of_width_crossings_do_not_count() -> void:
	var controller := _configured_controller()
	controller.sample_position(_beyond_line(5.0), 0.1)
	controller.sample_position(_behind_line(5.0), 0.1)

	assert_int(controller.lap_count).is_equal(0)

	var wide_controller := _configured_controller()
	wide_controller.sample_position(_behind_line(5.0, 40.0), 0.1)
	wide_controller.sample_position(_beyond_line(5.0, 40.0), 0.1)

	assert_int(wide_controller.lap_count).is_equal(0)


func test_immediate_repeat_crossing_is_rejected() -> void:
	var controller := _configured_controller()
	controller.sample_position(_behind_line(5.0), 0.1)
	controller.sample_position(_beyond_line(5.0), 0.1)
	controller.sample_position(_behind_line(5.0), 0.1)
	controller.sample_position(_beyond_line(5.0), 0.1)

	assert_int(controller.lap_count).is_equal(0)
	assert_float(controller.last_lap_time_seconds).is_equal(-1.0)


func test_reset_clears_timing_state() -> void:
	var controller := _configured_controller()
	controller.sample_position(_behind_line(5.0), 0.1)
	controller.sample_position(_beyond_line(5.0), 0.1)
	controller.reset_timing()

	assert_int(controller.lap_count).is_equal(0)
	assert_int(controller.current_lap_number).is_equal(0)
	assert_float(controller.best_lap_time_seconds).is_equal(-1.0)


func test_format_lap_time_uses_minutes_seconds_milliseconds() -> void:
	var controller := _configured_controller()
	assert_str(controller.format_lap_time(83.456)).is_equal("1:23.456")
	assert_str(controller.format_lap_time(-1.0)).is_equal("--:--.---")


func _configured_controller() -> LapTimingController:
	var vehicle := auto_free(Node3D.new()) as Node3D
	var track_definition := load(FUJI_TRACK_DEFINITION) as TrackDefinition
	var controller := auto_free(LAP_TIMING_SCRIPT.new()) as LapTimingController
	assert_bool(controller.configure(vehicle, track_definition)).is_true()
	return controller


func _behind_line(distance_m: float, lateral_m: float = 0.0) -> Vector3:
	return _crossing_position - _crossing_forward * distance_m + Vector3(lateral_m, 0.0, 0.0)


func _beyond_line(distance_m: float, lateral_m: float = 0.0) -> Vector3:
	return _crossing_position + _crossing_forward * distance_m + Vector3(lateral_m, 0.0, 0.0)


func _sample_accumulated_distance(controller: LapTimingController, distance_m: float) -> void:
	var step_distance_m := 10.0
	var position := _beyond_line(5.0)
	var steps := int(distance_m / step_distance_m)
	for _index in range(steps):
		position += _crossing_forward * step_distance_m
		controller.sample_position(position, 0.1)
