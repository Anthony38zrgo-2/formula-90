extends GdUnitTestSuite

const LAP_TIMING_SCRIPT := preload("res://scripts/runtime/lap_timing_controller.gd")
const FUJI_TRACK_DEFINITION := "res://data/tracks/fuji76_77.tres"
const F1_2030_PROFILE := "res://data/vehicles/f1_2030/f1_2030_v10_geometric.json"

var _crossing_position := Vector3(-294.278, 0.095, -275.19)
var _crossing_forward := Vector3(-0.0069, 0.0, -0.99998).normalized()


class FuelStubVehicle:
	extends Node3D

	var remaining_kg := 7.6
	var capacity_kg := 110.0
	var burn_per_snapshot_kg := 0.0

	func get_fuel_state_snapshot() -> Dictionary:
		remaining_kg = maxf(remaining_kg - burn_per_snapshot_kg, 0.0)
		return {
			"schema_version": 1,
			"remaining_kg": remaining_kg,
			"capacity_kg": capacity_kg,
		}


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


func test_average_consumption_tracks_completed_laps() -> void:
	var vehicle := auto_free(FuelStubVehicle.new()) as FuelStubVehicle
	var controller := _configured_fueled_controller(vehicle, 7.6)
	_start_first_lap(controller)
	vehicle.remaining_kg = 5.0
	_complete_lap(controller)

	assert_bool(controller.has_consumption_average).is_true()
	assert_float(controller.average_consumption_kg_per_lap).is_equal_approx(2.6, 0.0001)


func test_instant_delta_is_near_zero_on_the_reference_pace() -> void:
	var vehicle := auto_free(FuelStubVehicle.new()) as FuelStubVehicle
	var controller := _configured_fueled_controller(vehicle, 7.6)
	vehicle.burn_per_snapshot_kg = 2.53 / 900.0
	_start_first_lap(controller)
	_run_reference_lap(controller)

	assert_bool(controller.has_instant_consumption).is_true()
	assert_float(controller.instant_consumption_kg_per_lap).is_equal_approx(2.53, 0.05)
	assert_bool(controller.has_laps_delta).is_true()
	assert_float(controller.laps_delta).is_equal_approx(0.0, 0.15)


func test_instant_delta_turns_negative_when_consumption_rises() -> void:
	var vehicle := auto_free(FuelStubVehicle.new()) as FuelStubVehicle
	var controller := _configured_fueled_controller(vehicle, 7.6)
	vehicle.burn_per_snapshot_kg = 2.53 / 900.0
	_start_first_lap(controller)
	_run_steps(controller, 300)
	var reference_delta := controller.laps_delta

	vehicle.burn_per_snapshot_kg = 2.53 * 1.4 / 900.0
	_run_steps(controller, 300)

	assert_bool(controller.has_laps_delta).is_true()
	assert_float(controller.laps_delta).is_less(reference_delta)
	assert_float(controller.laps_delta).is_less(-0.25)


func test_instant_delta_turns_positive_when_consumption_drops() -> void:
	var vehicle := auto_free(FuelStubVehicle.new()) as FuelStubVehicle
	var controller := _configured_fueled_controller(vehicle, 7.6)
	vehicle.burn_per_snapshot_kg = 2.53 * 1.4 / 900.0
	_start_first_lap(controller)
	_run_steps(controller, 300)
	var high_delta := controller.laps_delta

	vehicle.burn_per_snapshot_kg = 2.53 * 0.5 / 900.0
	_run_steps(controller, 300)

	assert_bool(controller.has_laps_delta).is_true()
	assert_float(controller.laps_delta).is_greater(high_delta)


func test_refuel_restarts_consumption_accounting() -> void:
	var vehicle := auto_free(FuelStubVehicle.new()) as FuelStubVehicle
	var controller := _configured_fueled_controller(vehicle, 7.6)
	_start_first_lap(controller)
	vehicle.remaining_kg = 2.0
	_complete_lap(controller)

	assert_bool(controller.has_consumption_average).is_true()

	vehicle.remaining_kg = 7.6
	_complete_lap(controller)

	assert_bool(controller.has_consumption_average).is_false()
	assert_float(controller.average_consumption_kg_per_lap).is_equal(0.0)


func _configured_controller() -> LapTimingController:
	var vehicle := auto_free(Node3D.new()) as Node3D
	var track_definition := load(FUJI_TRACK_DEFINITION) as TrackDefinition
	var controller := auto_free(LAP_TIMING_SCRIPT.new()) as LapTimingController
	assert_bool(controller.configure(vehicle, track_definition)).is_true()
	return controller


func _configured_fueled_controller(
		vehicle: FuelStubVehicle,
		initial_fuel_kg: float) -> LapTimingController:
	vehicle.remaining_kg = initial_fuel_kg
	var track_definition := load(FUJI_TRACK_DEFINITION) as TrackDefinition
	var controller := auto_free(LAP_TIMING_SCRIPT.new()) as LapTimingController
	assert_bool(controller.configure(vehicle, track_definition, F1_2030_PROFILE)).is_true()
	return controller


func _start_first_lap(controller: LapTimingController) -> void:
	controller.sample_position(_behind_line(5.0), 0.1)
	controller.sample_position(_beyond_line(5.0), 0.1)


func _complete_lap(controller: LapTimingController) -> void:
	_sample_accumulated_distance(controller, 2200.0)
	controller.sample_position(_behind_line(5.0), 0.1)
	controller.sample_position(_beyond_line(5.0), 0.1)


func _run_reference_lap(controller: LapTimingController) -> void:
	_run_steps(controller, 900)


func _run_steps(controller: LapTimingController, step_count: int) -> void:
	_sample_accumulated_distance(controller, float(step_count) * 10.0)


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
