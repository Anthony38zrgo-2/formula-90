extends SceneTree

const InputControllerScript = preload("res://scripts/input/vehicle_rust_input_controller.gd")

class ProfileVehicle:
	extends Node3D
	var profile_path: String
	var requested_gear: int = -2
	func get_physics_config_path() -> String: return profile_path
	func set_throttle_amount(_value: float) -> void: pass
	func set_steering_input(_value: float) -> void: pass
	func set_brake_amount(_value: float) -> void: pass
	func set_handbrake_amount(_value: float) -> void: pass
	func set_clutch_amount(_value: float) -> void: pass
	func set_gear_request(value: int) -> void: requested_gear = value
	func get_current_gear() -> int: return 6
	func get_speed_kmh() -> float: return 100.0

func _initialize() -> void:
	call_deferred("verify_profile_manual_gears")

func verify_profile_manual_gears() -> void:
	var vehicle := ProfileVehicle.new()
	root.add_child(vehicle)
	var controller := InputControllerScript.new()
	controller.vehicle_node = vehicle
	vehicle.profile_path = "res://data/vehicles/f1_2030/profiles/geometric/vehicle.json"
	root.add_child(controller)
	controller.set_physics_process(false)
	for profile_path in [
		"res://data/vehicles/f1_2030/profiles/geometric/vehicle.json",
		"res://data/vehicles/f1_2030/f1_2030_v10_geometric.json",
		"res://data/vehicles/f1_2030/profiles/legacy/vehicle.json"]:
		vehicle.profile_path = profile_path
		if not controller.resolve_maximum_forward_gear():
			quit(1)
			return
		var expected_gear_count := 6 if profile_path.contains("/legacy/") else 7
		if controller.maximum_forward_gear != expected_gear_count:
			push_error("Forward gear count disagrees with profile: " + profile_path)
			quit(1)
			return
		controller._has_required_interface = true
		var shift_up_event := InputEventAction.new()
		shift_up_event.action = controller.action_shift_up
		shift_up_event.pressed = true
		Input.parse_input_event(shift_up_event)
		Input.flush_buffered_events()
		await physics_frame
		controller._physics_process(0.016)
		Input.action_release(controller.action_shift_up)
		if vehicle.requested_gear != (7 if expected_gear_count == 7 else -2):
			push_error("Manual shift request disagrees with active profile: " + profile_path)
			quit(1)
			return
		vehicle.requested_gear = -2
	controller.free()
	vehicle.free()
	print("VEHICLE_PROFILE_MANUAL_GEARS_PASS")
	quit(0)
