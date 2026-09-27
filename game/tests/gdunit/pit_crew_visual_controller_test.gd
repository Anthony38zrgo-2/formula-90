extends GdUnitTestSuite

const PIT_STOP_SCRIPT := preload("res://scripts/runtime/pit_stop_controller.gd")
const PIT_STOP_RULES_SCRIPT := preload("res://scripts/runtime/pit_stop_rules.gd")
const PIT_CREW_VISUAL_SCRIPT := preload("res://scripts/runtime/pit_crew_visual_controller.gd")
const WHEEL_FILES := [
	"f1_2030_v10_wheel_FL.glb",
	"f1_2030_v10_wheel_FR.glb",
	"f1_2030_v10_wheel_RL.glb",
	"f1_2030_v10_wheel_RR.glb",
]
const WHEEL_HUB_NAMES := ["FrontLeftWheel", "FrontRightWheel", "RearLeftWheel", "RearRightWheel"]


class VisualTestVehicle:
	extends Node3D

	var enable_player_input := true

	func get_fuel_state_snapshot() -> Dictionary:
		return {"remaining_kg": 40.0, "capacity_kg": 110.0}

	func get_speed_kmh() -> float:
		return 0.0


func test_fuji_crew_has_eleven_members_and_exchanges_four_wheels() -> void:
	var vehicle := auto_free(VisualTestVehicle.new()) as VisualTestVehicle
	add_child(vehicle)
	for wheel_index in WHEEL_HUB_NAMES.size():
		var hub := Node3D.new()
		hub.name = WHEEL_HUB_NAMES[wheel_index]
		vehicle.add_child(hub)
		var steering_pivot := Node3D.new()
		steering_pivot.name = "SteerPivot"
		hub.add_child(steering_pivot)
		var camber_pivot := Node3D.new()
		camber_pivot.name = "CamberPivot"
		steering_pivot.add_child(camber_pivot)
		var wheel_scene := load("res://assets/models/vehicles/f1-2030/" + WHEEL_FILES[wheel_index]) as PackedScene
		var wheel := wheel_scene.instantiate() as Node3D
		wheel.name = "Visual"
		camber_pivot.add_child(wheel)
	var controller := auto_free(PIT_STOP_SCRIPT.new()) as PitStopController
	add_child(controller)
	assert_bool(controller.configure(
		vehicle,
		load("res://data/tracks/fuji76_77.tres") as TrackDefinition,
		load("res://data/vehicles/f1_2030_v10.tres") as VehicleDefinition,
		PIT_STOP_RULES_SCRIPT.load_from_json())).is_true()
	var visual := auto_free(PIT_CREW_VISUAL_SCRIPT.new()) as PitCrewVisualController
	add_child(visual)
	visual.configure(controller, vehicle)
	controller.set_selected_stop_lap(2)
	controller.confirm_selection()
	controller.on_lap_started(2)

	assert_int(visual.crew_members.size()).is_equal(11)
	assert_int(visual.crew_root.get_child_count()).is_equal(11)
	for wheel_name in visual.WHEEL_NAMES:
		var carrier := visual.crew_members[wheel_name + "_wheel_carrier"] as Node3D
		assert_object(carrier.find_child("CarriedWheel", true, false)).is_not_null()
	visual._on_service_started({"tire_seconds": 3.0})
	assert_int(visual.wheel_exchanges.size()).is_equal(4)
	visual._on_service_progress({"phase": PitStopController.SERVICE_PHASE_TIRES, "tire_seconds_remaining": 1.5})
	for exchange in visual.wheel_exchanges:
		assert_bool((exchange["original"] as Node3D).visible).is_false()
	visual._on_service_completed()
	assert_int(visual.wheel_exchanges.size()).is_equal(0)
	for wheel_path in visual.VEHICLE_WHEEL_PATHS:
		assert_bool((vehicle.get_node(wheel_path) as Node3D).visible).is_true()
