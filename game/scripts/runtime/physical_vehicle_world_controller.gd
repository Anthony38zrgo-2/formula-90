class_name PhysicalVehicleWorldController
extends Node

signal physical_world_snapshots_published(snapshots: Array)
signal physical_world_collision_events_published(events: Array)

@export var enabled: bool = false
@export var physical_package_path: String = ""
@export var repository_root_path: String = ""
@export var native_library_path: String = "res://addons/formula90s/bin/vehicle_physics_engine.windows.template_debug.x86_64.dll"
@export var vehicle_profile_paths: Array[String] = []
@export var vehicle_paths: Array[NodePath] = []
@export var presentation_core_paths: Array[NodePath] = []

var physical_world_interface: RefCounted
var world_identifier: int = 0
var time_seconds: float = 0.0
var unconsumed_host_time_seconds: float = 0.0
var registered_vehicles: Dictionary = {}
var world_paused: bool = false
var failed: bool = false

func _ready() -> void:
	process_mode = Node.PROCESS_MODE_ALWAYS
	process_physics_priority = 1000
	set_physics_process(false)
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--physical-world-package="):
			physical_package_path = argument.trim_prefix("--physical-world-package=")
			enabled = true
	if enabled:
		call_deferred("initialize_physical_world")

func initialize_physical_world() -> bool:
	if world_identifier != 0 or failed:
		return false
	if physical_package_path.is_empty() or vehicle_paths.is_empty() or vehicle_paths.size() != presentation_core_paths.size() or vehicle_paths.size() != vehicle_profile_paths.size():
		return fail_physical_world("Explicit package, vehicle, profile and presentation core bindings are required")
	if not ClassDB.class_exists("PhysicalVehicleWorldInterface"):
		return fail_physical_world("Native physical world interface is unavailable")
	var profiles: Array[Dictionary] = []
	var presentation_cores: Array[Node] = []
	var vehicles: Array[Node3D] = []
	for binding_index in range(vehicle_paths.size()):
		var vehicle := get_node_or_null(vehicle_paths[binding_index]) as Node3D
		var presentation_core := get_node_or_null(presentation_core_paths[binding_index])
		var profile_document := FileAccess.get_file_as_string(vehicle_profile_paths[binding_index])
		var parsed_profile: Variant = JSON.parse_string(profile_document)
		if vehicle == null or presentation_core == null or not vehicle.has_method("set_physical_world_controlled") or not presentation_core.has_method("supports_physical_world_snapshots") or not presentation_core.supports_physical_world_snapshots() or not parsed_profile is Dictionary or not parsed_profile.has("coupled_world"):
			return fail_physical_world("Unsupported vehicle, presentation core or coupled profile")
		if presentation_core.config_json_path != vehicle_profile_paths[binding_index] or presentation_core.use_canonical_config or vehicle.physics_config_path != vehicle_profile_paths[binding_index]:
			return fail_physical_world("Presentation and vehicle profiles must match the physical profile")
		if vehicles.has(vehicle) or presentation_cores.has(presentation_core):
			return fail_physical_world("Each vehicle requires a unique presentation core")
		profiles.append(parsed_profile)
		presentation_core.target_vehicle_path = presentation_core.get_path_to(vehicle)
		vehicles.append(vehicle)
		presentation_cores.append(presentation_core)
	if has_unregistered_dynamic_body(get_tree().root, vehicles):
		return fail_physical_world("Moving Godot collision bodies must be registered with the Rust world")
	physical_world_interface = ClassDB.instantiate("PhysicalVehicleWorldInterface")
	if not physical_world_interface.initialize_library(ProjectSettings.globalize_path(native_library_path)):
		return fail_physical_world("Cannot initialize physical world library")
	var created := request_physical_world({"operation": "create_world", "interface_version": 1, "repository_root": ProjectSettings.globalize_path("res://..") if repository_root_path.is_empty() else ProjectSettings.globalize_path(repository_root_path), "physical_package_path": ProjectSettings.globalize_path(physical_package_path), "vehicle_profile": FileAccess.get_file_as_string(vehicle_profile_paths[0])})
	if failed:
		return false
	world_identifier = int(created["world_identifier"])
	for binding_index in range(vehicles.size()):
		if profiles[binding_index]["coupled_world"] != profiles[0]["coupled_world"]:
			return fail_physical_world("All vehicles must share one world configuration")
		var vehicle: Node3D = vehicles[binding_index]
		var registered := request_physical_world({"operation": "register_vehicle", "vehicle_profile": FileAccess.get_file_as_string(vehicle_profile_paths[binding_index]), "position_world_metres": vector_document(vehicle.global_position), "yaw_radians": vehicle.global_rotation.y})
		if failed:
			return false
		var entity_identifier := int(registered["entity_identifier"])
		registered_vehicles[entity_identifier] = {"vehicle": vehicle, "presentation_core": presentation_cores[binding_index], "last_impact_time_seconds": -INF}
		vehicle.tree_exiting.connect(remove_physical_vehicle.bind(entity_identifier))
		vehicle.set_physical_world_controlled(true)
		vehicle.physical_world_service_requested.connect(service_physical_vehicle.bind(entity_identifier))
	if not publish_physical_snapshots(request_physical_world({"operation": "snapshots"}).get("snapshots", [])):
		return false
	set_physics_process(true)
	return true

func _physics_process(duration_seconds: float) -> void:
	if failed or world_identifier == 0:
		return
	var paused := get_tree().paused
	if paused != world_paused:
		request_physical_world({"operation": "set_paused", "paused": paused})
		world_paused = paused
	if paused or failed:
		return
	for entity_identifier in registered_vehicles:
		var vehicle: Node3D = registered_vehicles[entity_identifier]["vehicle"]
		var gear_request: int = vehicle.gear_request
		request_physical_world({"operation": "submit_input", "entity_identifier": entity_identifier, "sample": {"time_seconds": time_seconds + unconsumed_host_time_seconds, "driving_aids_mask": vehicle.aids_enabled_mask, "input": {"steering": vehicle.steering_input, "throttle": vehicle.throttle_amount, "brake": vehicle.brake_amount, "handbrake": vehicle.handbrake_amount, "clutch": vehicle.clutch_amount, "gear_request": null if gear_request < -1 else gear_request}}})
		if failed:
			return
		vehicle.gear_request = -2
	var advanced := request_physical_world({"operation": "advance", "duration_seconds": duration_seconds})
	if failed:
		return
	time_seconds = float(advanced["time_seconds"])
	unconsumed_host_time_seconds = float(advanced["unconsumed_host_time_seconds"])
	if publish_physical_snapshots(advanced["snapshots"]):
		publish_physical_collision_audio(advanced["events"], advanced["snapshots"])
		physical_world_collision_events_published.emit(advanced["events"])

func remove_physical_vehicle(entity_identifier: int) -> void:
	if registered_vehicles.has(entity_identifier):
		var binding: Dictionary = registered_vehicles[entity_identifier]
		if is_instance_valid(binding["presentation_core"]) and binding["presentation_core"].has_method("clear_physical_world_vehicle"):
			binding["presentation_core"].clear_physical_world_vehicle(binding["vehicle"])
	if world_identifier != 0 and not failed:
		request_physical_world({"operation": "remove_vehicle", "entity_identifier": entity_identifier})
	registered_vehicles.erase(entity_identifier)

func service_physical_vehicle(operation: String, parameters: Dictionary, entity_identifier: int) -> void:
	var request := parameters.duplicate(true)
	if request.get("position_world_metres") is Vector3:
		request["position_world_metres"] = vector_document(request["position_world_metres"])
	request["operation"] = operation
	request["entity_identifier"] = entity_identifier
	request_physical_world(request)
	if not failed:
		publish_physical_snapshots(request_physical_world({"operation": "snapshots"}).get("snapshots", []))

func publish_physical_snapshots(snapshots: Array) -> bool:
	if failed:
		return false
	for snapshot in snapshots:
		var entity_identifier := int(snapshot["entity_identifier"])
		if not registered_vehicles.has(entity_identifier):
			return fail_physical_world("Snapshot references an unregistered vehicle")
		var binding: Dictionary = registered_vehicles[entity_identifier]
		if not binding["presentation_core"].accept_physical_world_snapshot(binding["vehicle"], snapshot):
			return fail_physical_world("Presentation core rejected authoritative snapshot")
	physical_world_snapshots_published.emit(snapshots)
	return true

func request_physical_world(request: Dictionary) -> Dictionary:
	if failed:
		return {}
	if world_identifier != 0:
		request["world_identifier"] = world_identifier
	var response: Dictionary = physical_world_interface.execute_request(request)
	if not response.get("success", false):
		fail_physical_world(str(response.get("error", "Physical world request failed")))
		return {}
	return response["result"]

func fail_physical_world(message: String) -> bool:
	failed = true
	for vehicle_path in vehicle_paths:
		var vehicle := get_node_or_null(vehicle_path)
		if vehicle is RigidBody3D:
			vehicle.freeze = true
	set_physics_process(false)
	push_error(message)
	return false

func vector_document(vector: Vector3) -> Dictionary:
	return {"x": vector.x, "y": vector.y, "z": vector.z}

func _exit_tree() -> void:
	if physical_world_interface != null:
		physical_world_interface.close_library()
	world_identifier = 0

func has_unregistered_dynamic_body(node: Node, vehicles: Array[Node3D]) -> bool:
	if (node is RigidBody3D or node is CharacterBody3D or node is AnimatableBody3D) and not vehicles.has(node):
		return true
	for child in node.get_children():
		if has_unregistered_dynamic_body(child, vehicles):
			return true
	return false

func publish_physical_collision_audio(events: Array, snapshots: Array) -> void:
	if events.is_empty():
		return
	var operating_masses: Dictionary = {}
	for snapshot in snapshots:
		operating_masses[int(snapshot["entity_identifier"])] = float(snapshot["operating_mass_kilograms"])
	for event in events:
		for affected_identifier in [event["first_entity_identifier"], event["second_entity_identifier"]]:
			if affected_identifier == null or not registered_vehicles.has(int(affected_identifier)):
				continue
			var entity_identifier := int(affected_identifier)
			var binding: Dictionary = registered_vehicles[entity_identifier]
			var event_time := float(event["time_seconds"])
			if event_time - float(binding.get("last_impact_time_seconds", -INF)) < 0.2:
				continue
			var velocity_change := float(event["normal_impulse_newton_seconds"]) / float(operating_masses[entity_identifier])
			if velocity_change <= 3.0:
				continue
			var impact_level := 1 if velocity_change < 8.0 else (2 if velocity_change < 15.0 else (3 if velocity_change < 25.0 else 4))
			binding["presentation_core"].trigger("impact_hit_" + str(impact_level))
			binding["last_impact_time_seconds"] = event_time
