extends SceneTree

var failed: bool = false

func _initialize() -> void:
	call_deferred("verify_aerodynamics_backend")

func require(condition: bool, message: String) -> void:
	if not condition:
		failed = true
		push_error(message)

func verify_aerodynamics_backend() -> void:
	var scene_path := "res://scenes/runtime/vehicle_test_session.tscn"
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--scene="):
			scene_path = argument.trim_prefix("--scene=")
	var session: Node = load(scene_path).instantiate()
	root.add_child(session)
	for frame_index in 5:
		await physics_frame
	var vehicle: Node = session.get_node("WorldViewport/RaceSession/VehicleContainer/ActiveVehicle/VehicleRigidBody")
	var backend := OS.get_environment("FORMULA90_AERODYNAMICS_BACKEND")
	var simplified := backend == "simplified"
	var enabled_floor_sensors := 0
	var enabled_suspension_sensors := 0
	for child in vehicle.get_children():
		if child is RayCast3D and str(child.name).begins_with("RayCast_") and child.enabled:
			enabled_suspension_sensors += 1
		if child is RayCast3D and (str(child.name).begins_with("Underfloor") or str(child.name).begins_with("Diffuser")):
			if child.enabled:
				enabled_floor_sensors += 1
	require(enabled_floor_sensors == (2 if simplified else 5), "Unexpected enabled floor sensor count")
	require(enabled_suspension_sensors == 12, "Suspension must retain twelve enabled contact rays")
	if simplified:
		var front_sensor := vehicle.get_node("UnderfloorFrontCenter") as RayCast3D
		require(is_zero_approx(front_sensor.position.x), "Front floor sensor must be centered")
		require(vehicle.get_node("DiffuserThroat").enabled, "Diffuser throat sensor must remain enabled")
	var tire_snapshot: Dictionary = vehicle.get_tire_state_snapshot()
	for wheel_identifier in ["FL", "FR", "RL", "RR"]:
		require(tire_snapshot.has(wheel_identifier), "Missing tire state for " + wheel_identifier)
		if tire_snapshot.has(wheel_identifier):
			var wheel_state: Dictionary = tire_snapshot[wheel_identifier]
			require(is_finite(float(wheel_state.pressure_kpa)) and float(wheel_state.pressure_kpa) > 0.0, "Invalid tire pressure for " + wheel_identifier)
	var snapshot: Dictionary = vehicle.get_underfloor_state_snapshot()
	require(snapshot.probe_layout == ("front_center_and_diffuser_throat" if simplified else "five_probes"), "Native floor layout disagrees with selected backend")
	var vehicle_container := session.get_node("WorldViewport/RaceSession/VehicleContainer")
	require(vehicle_container.get_child_count() == 1, "Comparison must retain only the player vehicle")
	if not failed:
		print("AERODYNAMICS_BACKEND_SMOKE_PASS backend=%s suspension=%s vehicles=1 floor_sensors=%d suspension_sensors=%d" % [backend, OS.get_environment("FORMULA90_SUSPENSION_BACKEND"), enabled_floor_sensors, enabled_suspension_sensors])
		print("TIRE_BACKEND_SMOKE_PASS backend=%s vehicles=1 suspension_sensors=%d" % [OS.get_environment("FORMULA90_TIRE_BACKEND"), enabled_suspension_sensors])
	quit(1 if failed else 0)
