extends SceneTree

const SESSION_SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"
const VERIFICATION_FILE_PATH := "user://engine_thermal_telemetry_verification.csv"

func _init() -> void:
	call_deferred("_verify")

func _verify() -> void:
	var telemetry_manager := root.get_node_or_null("TelemetryManager")
	if telemetry_manager == null:
		_fail("Telemetry manager is unavailable.")
		return
	telemetry_manager.enabled = false
	var session_resource := load(SESSION_SCENE_PATH) as PackedScene
	if session_resource == null:
		_fail("Vehicle session is unavailable.")
		return
	var session := session_resource.instantiate()
	root.add_child(session)
	for frame in 20:
		await physics_frame
	var vehicle := session.find_child("VehicleRigidBody", true, false)
	if vehicle == null:
		_fail("Vehicle is unavailable.")
		return
	var thermal_snapshot: Dictionary = vehicle.call("get_engine_thermal_state_snapshot")
	if thermal_snapshot.is_empty():
		_fail("Engine thermal state is unavailable.")
		return
	telemetry_manager.vehicle = vehicle
	telemetry_manager.set("_is_rust", true)
	var columns: Array = telemetry_manager.get_script().get_script_constant_map().get("CSV_COLUMNS", [])
	var sample: String = telemetry_manager.call("_format_line", Time.get_ticks_msec(), vehicle.linear_velocity)
	var output := FileAccess.open(VERIFICATION_FILE_PATH, FileAccess.WRITE)
	output.store_csv_line(PackedStringArray(columns))
	output.store_string(sample + "\n")
	output.close()
	var input := FileAccess.open(VERIFICATION_FILE_PATH, FileAccess.READ)
	var parsed_columns := input.get_csv_line()
	var parsed_sample := input.get_csv_line()
	input.close()
	DirAccess.remove_absolute(ProjectSettings.globalize_path(VERIFICATION_FILE_PATH))
	if parsed_columns.size() != parsed_sample.size() or parsed_columns.size() != columns.size():
		_fail("Engine thermal columns are not aligned with the CSV sample.")
		return
	var water: Dictionary = thermal_snapshot.get("water", {})
	var oil: Dictionary = thermal_snapshot.get("oil", {})
	var expected_values := {
		"Engine_Block_Temperature_Celsius": thermal_snapshot.get("engine_block_temperature_c"),
		"Water_Temperature_Celsius": water.get("temperature_c"),
		"Oil_Temperature_Celsius": oil.get("temperature_c"),
		"Water_Radiator_Opening": water.get("duct_opening"),
		"Oil_Radiator_Opening": oil.get("duct_opening"),
		"Water_Radiator_Mass_Flow_Kilograms_Per_Second": water.get("mass_flow_kg_s"),
		"Oil_Radiator_Mass_Flow_Kilograms_Per_Second": oil.get("mass_flow_kg_s"),
		"Water_Radiator_Drag_Newtons": water.get("drag_n"),
		"Oil_Radiator_Drag_Newtons": oil.get("drag_n"),
		"Total_Radiator_Drag_Newtons": thermal_snapshot.get("total_powertrain_cooling_drag_n"),
		"Engine_Available_Torque_Fraction": thermal_snapshot.get("available_engine_torque_fraction"),
		"Engine_Mechanical_Power_Watts": thermal_snapshot.get("engine_mechanical_power_w"),
		"Water_Rejected_Heat_Watts": thermal_snapshot.get("water_rejected_heat_w"),
		"Oil_Rejected_Heat_Watts": thermal_snapshot.get("oil_rejected_heat_w")
	}
	for column_name in expected_values:
		var column_index := columns.find(column_name)
		if column_index < 0 or expected_values[column_name] == null:
			_fail("Engine thermal column is unavailable: " + column_name)
			return
		if absf(float(parsed_sample[column_index]) - float(expected_values[column_name])) > 0.001:
			_fail("Engine thermal column differs from the native snapshot: " + column_name)
			return
	print("[PASS] Engine water and oil telemetry columns match the native snapshot.")
	quit(0)

func _fail(message: String) -> void:
	printerr("[FAIL] " + message)
	quit(1)
