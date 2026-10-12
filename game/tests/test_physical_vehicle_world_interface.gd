extends SceneTree

func _require(condition: bool, message: String) -> bool:
	if condition:
		return true
	push_error(message)
	quit(1)
	return false

func _initialize() -> void:
	if not _require(ClassDB.class_exists("PhysicalVehicleWorldInterface"), "PhysicalVehicleWorldInterface is not registered in the native build"):
		return
	var world_interface: Object = ClassDB.instantiate("PhysicalVehicleWorldInterface")
	for method_name in ["initialize_library", "execute_request", "close_library"]:
		if not _require(world_interface.has_method(method_name), "Missing world interface method: " + method_name):
			return
	var arguments := OS.get_cmdline_user_args()
	if not _require(arguments.size() == 2, "Expected physical library and source-verified physical package paths"):
		return
	if not _require(world_interface.call("initialize_library", arguments[0]), "Physical world library failed version or symbol validation"):
		return
	var repository_root: String = ProjectSettings.globalize_path("res://").path_join("..").simplify_path()
	var resolution: Dictionary = world_interface.call("execute_request", {
		"operation": "resolve_vehicle_profile", "interface_version": 1,
		"repository_root": repository_root,
		"vehicle_profile_path": ProjectSettings.globalize_path("res://data/vehicles/f1_2030/profiles/geometric/vehicle.json")})
	if not _require(resolution.get("success", false), "Profile resolution failed: " + str(resolution.get("error"))):
		return
	var resolved: Variant = resolution["result"]["resolved_vehicle_configuration"]
	if not _require(resolved is Dictionary and resolved["configuration"].has("coupled_world"), "Candidate world settings are missing"):
		return
	for profile_document in [resolved["configuration"], JSON.stringify(resolved["configuration"])]:
		var profile_creation: Dictionary = world_interface.call("execute_request", {
			"operation": "create_world", "interface_version": 1,
			"repository_root": repository_root, "physical_package_path": arguments[1],
			"vehicle_profile": profile_document})
		if not _require(profile_creation.get("success", false), "Serialized profile creation failed: " + str(profile_creation.get("error"))):
			return
		var profile_destruction: Dictionary = world_interface.call("execute_request", {
			"operation": "destroy_world", "world_identifier": profile_creation["result"]["world_identifier"]})
		if not _require(profile_destruction.get("success", false), "Profile world destruction failed"):
			return
	var fractional_profile: Dictionary = resolved["configuration"].duplicate(true)
	fractional_profile["coupled_world"]["contacts"]["maximum_events_per_step"] = 128.5
	var rejected_profile: Dictionary = world_interface.call("execute_request", {
		"operation": "create_world", "interface_version": 1,
		"repository_root": repository_root, "physical_package_path": arguments[1],
		"vehicle_profile": fractional_profile})
	if not _require(not rejected_profile.get("success", true), "Fractional integer setting was silently accepted"):
		return
	var creation: Dictionary = world_interface.call("execute_request", {
		"operation": "create_world", "interface_version": 1,
		"repository_root": repository_root,
		"physical_package_path": arguments[1], "configuration": resolved["configuration"]["coupled_world"]})
	if not _require(creation.get("success", false), "World creation failed: " + str(creation.get("error"))):
		return
	var world_identifier: int = creation["result"]["world_identifier"]
	var snapshot: Dictionary = world_interface.call("execute_request", {"operation": "snapshots", "world_identifier": world_identifier})
	if not _require(snapshot.get("success", false) and snapshot["result"]["snapshots"].is_empty(), "New world has unexpected vehicles"):
		return
	var pause: Dictionary = world_interface.call("execute_request", {"operation": "set_paused", "world_identifier": world_identifier, "paused": true})
	if not _require(pause.get("success", false), "World pause failed"):
		return
	var advance: Dictionary = world_interface.call("execute_request", {"operation": "advance", "world_identifier": world_identifier, "duration_seconds": 0.1})
	if not _require(advance.get("success", false) and advance["result"]["time_seconds"] == 0.0, "Paused world consumed simulation time"):
		return
	var destruction: Dictionary = world_interface.call("execute_request", {"operation": "destroy_world", "world_identifier": world_identifier})
	if not _require(destruction.get("success", false), "World destruction failed"):
		return
	var stale: Dictionary = world_interface.call("execute_request", {"operation": "snapshots", "world_identifier": world_identifier})
	if not _require(not stale.get("success", true), "Destroyed world accepted a stale handle"):
		return
	world_interface.call("close_library")
	print("Physical vehicle world registration, native version, pause and lifecycle checks passed")
	quit(0)
