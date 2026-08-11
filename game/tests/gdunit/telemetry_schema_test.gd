extends GdUnitTestSuite

const TELEMETRY_SCRIPT := preload("res://addons/formula90s/scripts/telemetry_manager.gd")
const VEHICLE_SCENE := "res://scenes/vehicles/jordan_191/jordan_191_phase_b.tscn"
const EXPECTED_COLUMNS := [
	"Time_ms", "Speed_kmh", "RPM", "Gear",
	"Throttle", "Brake", "Steering",
	"Lat_G", "Long_G",
	"FL_Comp", "FR_Comp", "RL_Comp", "RR_Comp",
	"Front_Slip", "Rear_Slip",
	"Session_Id", "Session_Timestamp_UTC", "Physics_Hz",
	"Test_Id", "Track_Scene", "Vehicle_Node_Path", "Vehicle_Scene",
	"Vehicle_Script", "Setup_Schema_Version", "Setup_JSON"
]


func test_csv_schema_keeps_setup_and_provenance_in_same_row() -> void:
	assert_array(TELEMETRY_SCRIPT.CSV_COLUMNS).is_equal(EXPECTED_COLUMNS)
	assert_int(TELEMETRY_SCRIPT.CSV_COLUMNS.size()).is_equal(25)
	assert_str(TELEMETRY_SCRIPT.CSV_COLUMNS[-1]).is_equal("Setup_JSON")


func test_formatted_row_matches_header_and_contains_valid_setup_json(_timeout := 10000) -> void:
	var runner := scene_runner(VEHICLE_SCENE)
	var vehicle := runner.scene().get_node("VehicleRigidBody")
	var manager: Variant = auto_free(TELEMETRY_SCRIPT.new())
	manager.enabled = false
	add_child(manager)
	manager.vehicle = vehicle
	manager._session_id = "session_test"
	manager._session_timestamp_utc = "2026-08-11T23:00:00Z"
	manager._setup_json = JSON.stringify(manager._build_setup_snapshot("telemetry_test.csv"))

	var row: String = manager._format_line(1000, Vector3.ZERO)
	var file := create_temp_file("telemetry_schema", "row.csv")
	var row_path := file.get_path()
	file.store_line(row)
	file.close()
	var reader := FileAccess.open(row_path, FileAccess.READ)
	var fields := reader.get_csv_line()
	reader.close()

	assert_int(fields.size()).is_equal(EXPECTED_COLUMNS.size())
	assert_str(fields[15]).is_equal("session_test")
	assert_str(fields[23]).is_equal("1")
	var setup: Variant = JSON.parse_string(fields[24])
	assert_bool(setup is Dictionary).is_true()
	assert_bool(setup.has("session")).is_true()
	assert_bool(setup.has("provenance")).is_true()
	assert_bool(setup.has("chassis")).is_true()
	assert_bool(setup.has("tires")).is_true()
	assert_bool(setup.has("suspension")).is_true()
	assert_bool(setup.has("engine")).is_true()
