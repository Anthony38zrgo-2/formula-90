extends GdUnitTestSuite

const PIT_STOP_SCRIPT := preload("res://scripts/runtime/pit_stop_controller.gd")
const PIT_STOP_RULES_SCRIPT := preload("res://scripts/runtime/pit_stop_rules.gd")
const FUJI_TRACK_DEFINITION := "res://data/tracks/fuji76_77.tres"
const F1_2030_VEHICLE_DEFINITION := "res://data/vehicles/f1_2030_v10.tres"


class PitStopStubVehicle:
	extends Node3D

	var remaining_kg := 7.6
	var capacity_kg := 110.0
	var speed_kmh := 0.0
	var enable_player_input := true
	var throttle := 1.0
	var brake := 0.0
	var handbrake := 0.0
	var steering := 1.0
	var clutch := 1.0
	var gear_request := 5
	var tires_replaced_count := 0
	var fuel_target_received := -1.0

	func get_fuel_state_snapshot() -> Dictionary:
		return {
			"schema_version": 1,
			"remaining_kg": remaining_kg,
			"capacity_kg": capacity_kg,
		}

	func get_speed_kmh() -> float:
		return speed_kmh

	func set_fuel_kg(target_kg: float) -> void:
		fuel_target_received = target_kg
		remaining_kg = target_kg

	func replace_tires() -> void:
		tires_replaced_count += 1

	func set_throttle_amount(value: float) -> void:
		throttle = value

	func set_brake_amount(value: float) -> void:
		brake = value

	func set_handbrake_amount(value: float) -> void:
		handbrake = value

	func set_steering_input(value: float) -> void:
		steering = value

	func set_clutch_amount(value: float) -> void:
		clutch = value

	func set_gear_request(value: int) -> void:
		gear_request = value


func test_fuji_metadata_maps_every_painted_pit_box() -> void:
	var vehicle := auto_free(PitStopStubVehicle.new()) as PitStopStubVehicle
	var controller := _controller_for(vehicle)

	assert_int(controller.get_box_count()).is_equal(16)
	assert_int(controller.get_assigned_box_index()).is_equal(0)
	assert_bool(controller._is_inside_assigned_box(Vector3(-267.82, 0.0, 9.365))).is_true()
	assert_bool(controller._is_inside_assigned_box(Vector3(-267.82, 0.0, 20.0))).is_false()
	assert_bool(controller._is_inside_assigned_box(Vector3(-262.0, 0.0, 9.365))).is_false()
	assert_bool(controller._is_inside_strip(Vector3(-267.8, 0.0, -130.0))).is_true()
	assert_bool(controller._is_inside_strip(Vector3(-250.0, 0.0, -130.0))).is_false()


func test_default_selection_is_softs_and_fifteen_laps() -> void:
	var vehicle := auto_free(PitStopStubVehicle.new()) as PitStopStubVehicle
	var controller := _controller_for(vehicle)
	var selection := controller.get_selection()

	assert_str(str(selection.get("compound_label"))).is_equal("BLANDOS")
	assert_int(int(selection.get("fuel_laps"))).is_equal(15)
	assert_float(controller.get_fuel_target_kg()).is_equal_approx(37.95, 0.01)
	assert_int(controller.get_maximum_fuel_laps()).is_equal(43)


func test_fuel_selector_clamps_to_minimum_and_tank_capacity() -> void:
	var vehicle := auto_free(PitStopStubVehicle.new()) as PitStopStubVehicle
	var controller := _controller_for(vehicle)

	controller.set_fuel_target_laps(999)
	assert_int(controller.fuel_target_laps).is_equal(43)
	controller.set_fuel_target_laps(-5)
	assert_int(controller.fuel_target_laps).is_equal(1)
	controller.set_fuel_target_laps(15)
	assert_int(controller.fuel_target_laps).is_equal(15)


func test_service_runs_tires_then_fuel_and_releases_the_car() -> void:
	var vehicle := auto_free(PitStopStubVehicle.new()) as PitStopStubVehicle
	var controller := _controller_for(vehicle)

	controller._begin_service()
	assert_int(controller.service_phase).is_equal(PitStopController.SERVICE_PHASE_TIRES)
	assert_float(controller.tire_seconds_remaining).is_equal_approx(3.0, 0.0001)
	assert_float(controller.fuel_seconds_remaining).is_equal_approx(6.07, 0.01)
	assert_bool(vehicle.enable_player_input).is_false()
	assert_float(vehicle.brake).is_equal(1.0)
	assert_int(vehicle.gear_request).is_equal(0)

	controller._advance_service(2.5)
	assert_int(vehicle.tires_replaced_count).is_equal(0)
	controller._advance_service(0.6)
	assert_int(vehicle.tires_replaced_count).is_equal(1)
	assert_int(controller.service_phase).is_equal(PitStopController.SERVICE_PHASE_FUEL)
	controller._advance_service(6.0)
	assert_float(vehicle.fuel_target_received).is_equal(-1.0)
	controller._advance_service(0.1)
	assert_float(vehicle.fuel_target_received).is_equal_approx(37.95, 0.01)
	assert_float(vehicle.remaining_kg).is_equal_approx(37.95, 0.01)
	assert_int(controller.service_phase).is_equal(PitStopController.SERVICE_PHASE_NONE)
	assert_bool(vehicle.enable_player_input).is_true()
	assert_float(vehicle.brake).is_equal(0.0)
	assert_int(vehicle.gear_request).is_equal(0)


func test_tank_above_the_target_only_pays_the_tire_time() -> void:
	var vehicle := auto_free(PitStopStubVehicle.new()) as PitStopStubVehicle
	vehicle.remaining_kg = 60.0
	var controller := _controller_for(vehicle)

	controller._begin_service()
	controller._advance_service(3.1)

	assert_float(vehicle.fuel_target_received).is_equal(-1.0)
	assert_float(vehicle.remaining_kg).is_equal(60.0)
	assert_int(vehicle.tires_replaced_count).is_equal(1)
	assert_int(controller.service_phase).is_equal(PitStopController.SERVICE_PHASE_NONE)
	assert_bool(vehicle.enable_player_input).is_true()


func test_track_without_pit_lane_metadata_disables_the_controller() -> void:
	var vehicle := auto_free(PitStopStubVehicle.new()) as PitStopStubVehicle
	var controller := auto_free(PIT_STOP_SCRIPT.new()) as PitStopController
	var vehicle_definition := load(F1_2030_VEHICLE_DEFINITION) as VehicleDefinition

	assert_bool(controller.configure(
		vehicle,
		TrackDefinition.new(),
		vehicle_definition,
		PIT_STOP_RULES_SCRIPT.load_from_json())).is_false()
	assert_bool(controller.is_configured).is_false()


func test_pit_lane_selection_activity_tracks_the_zone() -> void:
	var vehicle := auto_free(PitStopStubVehicle.new()) as PitStopStubVehicle
	var controller := _controller_for(vehicle)

	assert_bool(controller.is_selection_active()).is_false()
	controller.in_pit_lane = true
	assert_bool(controller.is_selection_active()).is_true()
	controller.in_pit_lane = false
	controller.service_phase = PitStopController.SERVICE_PHASE_TIRES
	assert_bool(controller.is_selection_active()).is_true()


func _controller_for(vehicle: PitStopStubVehicle) -> PitStopController:
	var controller := auto_free(PIT_STOP_SCRIPT.new()) as PitStopController
	var track_definition := load(FUJI_TRACK_DEFINITION) as TrackDefinition
	var vehicle_definition := load(F1_2030_VEHICLE_DEFINITION) as VehicleDefinition
	var configured := controller.configure(
		vehicle,
		track_definition,
		vehicle_definition,
		PIT_STOP_RULES_SCRIPT.load_from_json())
	assert_bool(configured).is_true()
	return controller
