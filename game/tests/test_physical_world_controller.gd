extends SceneTree

const ControllerScript = preload("res://scripts/runtime/physical_vehicle_world_controller.gd")

class PresentationVehicle extends Node3D:
	var gear_request: int = -2
	var aids_enabled_mask: int = 255
	var steering_input: float = 0.1
	var throttle_amount: float = 0.4
	var brake_amount: float = 0.0
	var handbrake_amount: float = 0.0
	var clutch_amount: float = 0.0

class PresentationCore extends Node:
	var snapshots: Array = []
	func accept_physical_world_snapshot(_vehicle: Node, snapshot: Dictionary) -> bool:
		snapshots.append(snapshot)
		return true

class RecordedWorldInterface extends RefCounted:
	var requests: Array = []
	var time_seconds: float = 0.0
	func execute_request(request: Dictionary) -> Dictionary:
		requests.append(request.duplicate(true))
		var result: Dictionary = {}
		match request["operation"]:
			"advance":
				time_seconds += float(request["duration_seconds"])
				result = {"time_seconds": time_seconds, "unconsumed_host_time_seconds": 0.0001, "snapshots": [{"entity_identifier": 1}, {"entity_identifier": 2}], "events": []}
			"snapshots":
				result = {"snapshots": [{"entity_identifier": 1}, {"entity_identifier": 2}]}
		return {"success": true, "result": result}
	func close_library() -> void:
		requests.append({"operation": "close_library"})

var contract_failed: bool = false

func require_condition(condition: bool, message: String) -> void:
	if not condition:
		contract_failed = true
		push_error(message)

func _initialize() -> void:
	call_deferred("run_controller_contract")

func run_controller_contract() -> void:
	var controller := ControllerScript.new()
	root.add_child(controller)
	var world_interface := RecordedWorldInterface.new()
	controller.physical_world_interface = world_interface
	controller.world_identifier = 17
	for entity_identifier in [1, 2]:
		var vehicle := PresentationVehicle.new()
		var presentation_core := PresentationCore.new()
		root.add_child(vehicle)
		root.add_child(presentation_core)
		controller.registered_vehicles[entity_identifier] = {"vehicle": vehicle, "presentation_core": presentation_core}
	controller._physics_process(0.01)
	require_condition(world_interface.requests.size() == 3, "Two inputs must precede one shared world advance")
	require_condition(world_interface.requests[0]["operation"] == "submit_input" and world_interface.requests[1]["operation"] == "submit_input" and world_interface.requests[2]["operation"] == "advance", "Physical ownership order is invalid")
	require_condition(world_interface.requests[0]["sample"]["input"]["gear_request"] == null, "No-change gear sentinel was not translated")
	controller._physics_process(0.01)
	require_condition(is_equal_approx(float(world_interface.requests[3]["sample"]["time_seconds"]), 0.0101), "Input timestamp lost the unconsumed host fraction")
	paused = true
	controller._physics_process(0.1)
	require_condition(world_interface.requests.back()["operation"] == "set_paused", "Paused tree advanced physical time")
	paused = false
	controller._physics_process(0.01)
	controller.service_physical_vehicle("refuel", {"kilograms": 20.0}, 1)
	require_condition(world_interface.requests[-2]["operation"] == "refuel" and world_interface.requests[-1]["operation"] == "snapshots", "Refuel did not republish authoritative state")
	controller.service_physical_vehicle("reset_vehicle", {"position_world_metres": Vector3(1.0, 2.0, 3.0), "yaw_radians": 0.4}, 1)
	require_condition(world_interface.requests[-2]["position_world_metres"] == {"x": 1.0, "y": 2.0, "z": 3.0}, "Reset pose did not cross the document boundary")
	controller.remove_physical_vehicle(2)
	require_condition(not controller.registered_vehicles.has(2) and world_interface.requests.back()["operation"] == "remove_vehicle", "Vehicle removal retained a stale world binding")
	if not contract_failed:
		print("PHYSICAL_WORLD_CONTROLLER_CONTRACT_PASS")
	quit(1 if contract_failed else 0)
