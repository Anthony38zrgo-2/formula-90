extends RefCounted

static func resolve_profile(configuration_path: String, resolve_suspension_presentation := false) -> Dictionary:
	if configuration_path.is_empty():
		return {}
	var profile_interface: Object = ClassDB.instantiate("PhysicalVehicleWorldInterface")
	var build_configuration := "debug" if OS.has_feature("debug") else "release"
	var physics_library_path := "res://addons/formula90s/bin/vehicle_physics_engine.windows.template_%s.x86_64.dll" % build_configuration
	if profile_interface == null or not profile_interface.initialize_library(physics_library_path):
		push_error("Cannot initialize the authoritative vehicle profile resolver")
		return {}
	var resolution: Dictionary = profile_interface.execute_request({
		"operation": "resolve_vehicle_profile", "interface_version": 1,
		"repository_root": ProjectSettings.globalize_path("res://..").simplify_path(),
		"vehicle_profile_path": ProjectSettings.globalize_path(configuration_path),
		"resolve_suspension_presentation": resolve_suspension_presentation})
	profile_interface.close_library()
	if not resolution.get("success", false):
		push_error("Vehicle profile resolution failed: " + str(resolution.get("error")))
		return {}
	return resolution.get("result", {})

static func load_configuration(configuration_path: String) -> Dictionary:
	return resolve_profile(configuration_path).get("configuration", {})
