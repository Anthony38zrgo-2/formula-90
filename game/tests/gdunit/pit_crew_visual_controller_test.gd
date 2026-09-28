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
	assert_int(visual.crew_members.size()).is_equal(11)
	assert_bool(visual.crew_root.visible).is_false()
	var initial_crew_root := visual.crew_root
	var initial_carrier := visual.crew_members["front_left_wheel_carrier"] as Node3D
	controller.set_selected_stop_lap(2)
	controller.confirm_selection()
	controller.on_lap_started(2)

	assert_int(visual.crew_members.size()).is_equal(11)
	assert_bool(visual.crew_root.visible).is_true()
	assert_object(visual.crew_root).is_same(initial_crew_root)
	assert_object(visual.crew_members["front_left_wheel_carrier"]).is_same(initial_carrier)
	assert_int(visual.crew_root.get_child_count()).is_equal(11)
	visual._on_crew_visibility_changed(false)
	assert_bool(visual.crew_root.visible).is_false()
	assert_int(visual.crew_members.size()).is_equal(11)
	visual._on_crew_visibility_changed(true)
	assert_bool(visual.crew_root.visible).is_true()
	assert_object(visual.crew_members["front_left_wheel_carrier"]).is_same(initial_carrier)
	for wheel_name in visual.WHEEL_NAMES:
		var carrier := visual.crew_members[wheel_name + "_wheel_carrier"] as Node3D
		assert_object(carrier.find_child("CarriedWheel", true, false)).is_not_null()
	visual._on_service_started({"tire_seconds": 3.0})
	assert_int(visual.wheel_exchanges.size()).is_equal(4)
	var fuel_operator := visual.crew_members["fuel_hose_operator"] as Node3D
	var fuel_operator_resting_position := visual.fuel_operator_resting_transform.origin
	visual._on_service_progress({"phase": PitStopController.SERVICE_PHASE_TIRES, "tire_seconds_remaining": 2.4})
	var mechanic := visual.crew_members["front_left_wheel_change_mechanic"] as Node3D
	var mechanic_resting_transform: Transform3D = visual.crew_member_service_transforms[mechanic.name]
	assert_float(mechanic.global_position.distance_to(mechanic_resting_transform.origin)).is_greater(0.2)
	var mechanic_tool := visual.wheel_change_tools[mechanic.name] as Node3D
	assert_float(absf(mechanic_tool.rotation.z)).is_greater(0.01)
	visual._on_service_progress({"phase": PitStopController.SERVICE_PHASE_TIRES, "tire_seconds_remaining": 1.74})
	var front_left_exchange: Dictionary = visual.wheel_exchanges[0]
	var front_left_carrier := front_left_exchange["carrier"] as Node3D
	var carrier_resting_transform: Transform3D = visual.crew_member_service_transforms[front_left_carrier.name]
	assert_float(front_left_carrier.global_position.distance_to(carrier_resting_transform.origin)).is_greater(0.5)
	var carried_replacement := front_left_exchange["replacement"] as Node3D
	var carried_wheel_transform: Transform3D = front_left_exchange["carrier_relative_replacement_transform"]
	assert_float(carried_replacement.global_position.distance_to(
		(front_left_carrier.global_transform * carried_wheel_transform).origin)).is_less(0.05)
	visual._on_service_progress({"phase": PitStopController.SERVICE_PHASE_TIRES, "tire_seconds_remaining": 0.12})
	var fuel_nozzle := fuel_operator.find_child("FuelNozzle", true, false) as Node3D
	var nozzle_to_port := fuel_nozzle.global_position - vehicle.global_transform * visual.FUEL_PORT_LOCAL_POSITION
	nozzle_to_port.y = 0.0
	assert_float(nozzle_to_port.length()).is_less(0.05)
	for exchange in visual.wheel_exchanges:
		var original_parts: Array[Node3D] = exchange["original_parts"]
		for original_part in original_parts:
			assert_bool(original_part.visible).is_false()
		var original_wheel := original_parts[0].get_parent().get_parent() as Node3D
		assert_bool((original_wheel.get_node("BrakeStatic") as Node3D).visible).is_true()
	visual._on_service_progress({"phase": PitStopController.SERVICE_PHASE_FUEL, "fuel_seconds_remaining": 1.0})
	assert_int(visual.wheel_exchanges.size()).is_equal(0)
	assert_int(visual.carried_removed_wheels.size()).is_equal(4)
	visual._on_service_completed()
	visual._process(1.2)
	assert_float(fuel_operator.global_position.distance_to(fuel_operator_resting_position)).is_less(0.01)
	for wheel_path in visual.VEHICLE_WHEEL_PATHS:
		for wheel_part in visual._find_transferable_wheel_parts(vehicle.get_node(wheel_path) as Node3D):
			assert_bool(wheel_part.visible).is_true()
	visual._on_service_started({"tire_seconds": 3.0})
	assert_int(visual.carried_removed_wheels.size()).is_equal(0)
	assert_int(visual.wheel_exchanges.size()).is_equal(4)
	visual._on_service_completed()
