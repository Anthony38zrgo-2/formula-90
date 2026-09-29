extends GdUnitTestSuite

const INPUT_PROFILE_SCRIPT := preload("res://scripts/input/input_profile.gd")
const CANONICAL_PROFILE_PATH := "res://data/input/input_profile.json"


func test_canonical_profile_parses_without_errors() -> void:
	var profile: InputProfile = INPUT_PROFILE_SCRIPT.load_from_path(CANONICAL_PROFILE_PATH)
	assert_bool(profile.is_valid()).is_true()
	assert_int(profile.schema_version).is_equal(1)


func test_canonical_profile_covers_every_action_constant() -> void:
	var profile: InputProfile = INPUT_PROFILE_SCRIPT.load_from_path(CANONICAL_PROFILE_PATH)
	for action_name in [
		InputBindings.THROTTLE, InputBindings.BRAKES, InputBindings.STEER_LEFT, InputBindings.STEER_RIGHT,
		InputBindings.HANDBRAKE, InputBindings.CLUTCH, InputBindings.SHIFT_UP, InputBindings.SHIFT_DOWN,
		InputBindings.TOGGLE_TRANSMISSION, InputBindings.TOGGLE_TRACTION_CONTROL,
		InputBindings.AID_1, InputBindings.AID_2, InputBindings.AID_3, InputBindings.AID_4, InputBindings.AID_5,
		InputBindings.UI_BACK_TO_MENU, InputBindings.SHOW_DEBUG, InputBindings.DEBUG_NEXT, InputBindings.DEBUG_PREVIOUS,
		InputBindings.RESET_VEHICLE, InputBindings.TOGGLE_CAMERA,
		InputBindings.PIT_FIELD_UP, InputBindings.PIT_FIELD_DOWN, InputBindings.PIT_VALUE_LEFT,
		InputBindings.PIT_VALUE_RIGHT, InputBindings.PIT_CONFIRM, InputBindings.PIT_TOGGLE_MENU,
	]:
		assert_bool(profile.has_registered_action(action_name)).is_true()
	assert_int(profile.action_definitions.size()).is_equal(27)


func test_missing_file_is_reported_and_registers_nothing() -> void:
	var profile: InputProfile = INPUT_PROFILE_SCRIPT.load_from_path("res://data/input/does_not_exist.json")
	assert_bool(profile.is_valid()).is_false()
	assert_bool(profile.errors.is_empty())
	assert_int(profile.action_definitions.size()).is_equal(0)


func test_non_object_json_is_reported() -> void:
	var profile: InputProfile = INPUT_PROFILE_SCRIPT.new()
	profile.parse_json_text("[1, 2, 3]")
	assert_bool(profile.is_valid()).is_false()
	assert_int(profile.action_definitions.size()).is_equal(0)


func test_unsupported_schema_version_is_reported() -> void:
	var profile: InputProfile = INPUT_PROFILE_SCRIPT.new()
	profile.parse_json_text(JSON.stringify({"schema_version": 2}))
	assert_bool(profile.is_valid()).is_false()


func test_out_of_range_values_reject_their_entry_only() -> void:
	var document := {
		"schema_version": 1,
		"actions": [
			{"name": "Throttle", "deadzone": 1.4, "events": [{"type": "physical_key", "physical_keycode": 65}]},
			{"name": "Brakes", "deadzone": 0.5, "events": [{"type": "physical_key", "physical_keycode": 66}]},
			{"name": "Bad", "deadzone": 0.5, "events": [{"type": "telepathic_pulse"}]},
		],
		"axis_shaping": {
			"steering_axis_deadzone": 0.08,
			"steering_curve_exponent": 99.0,
			"throttle_curve_exponent": 1.0,
			"brake_curve_exponent": 1.0,
			"steering_return_speed_per_second": 6.85,
		},
		"vibration": {
			"minimum_speed_kmh": 15.0,
			"piano": {"weak_motor_magnitude": 0.5, "strong_motor_magnitude": 0.5, "pulses_per_second": 9.0},
			"off_asphalt": {"weak_motor_magnitude": 0.5, "strong_motor_magnitude": 0.5, "pulses_per_second": 5.0},
			"front_grip_limit": {"weak_motor_magnitude": 0.5, "strong_motor_magnitude": 0.5, "pulses_per_second": 12.0, "engage_slip_angle_rad": 0.1, "release_slip_angle_rad": 0.075},
			"rear_grip_limit": {"weak_motor_magnitude": 0.5, "strong_motor_magnitude": 0.5, "pulses_per_second": 12.0, "engage_slip_angle_rad": 0.1, "release_slip_angle_rad": 0.075},
		},
	}
	var profile: InputProfile = INPUT_PROFILE_SCRIPT.new()
	profile.parse_json_text(JSON.stringify(document))
	assert_bool(profile.is_valid()).is_false()
	assert_bool(profile.has_registered_action("Throttle"))
	assert_bool(profile.has_registered_action("Brakes"))
	assert_bool(profile.has_registered_action("Bad"))
	assert_bool(profile.has_registered_action("Aid"))


func test_signed_curve_boundaries_and_symmetry() -> void:
	assert_float(INPUT_PROFILE_SCRIPT.apply_signed_curve(1.0, 1.8)).is_equal(1.0)
	assert_float(INPUT_PROFILE_SCRIPT.apply_signed_curve(0.0, 1.8)).is_equal(0.0)
	assert_float(INPUT_PROFILE_SCRIPT.apply_signed_curve(-1.0, 1.8)).is_equal(-1.0)
	assert_float(absf(INPUT_PROFILE_SCRIPT.apply_signed_curve(0.6, 1.8))).is_equal(absf(INPUT_PROFILE_SCRIPT.apply_signed_curve(-0.6, 1.8)))
	assert_float(INPUT_PROFILE_SCRIPT.apply_signed_curve(0.5, 2.0)).is_between(0.24, 0.26)
	assert_float(INPUT_PROFILE_SCRIPT.apply_signed_curve(5.0, 1.8)).is_equal(1.0)


func test_unsigned_curve_output_stays_in_unit_range() -> void:
	assert_float(INPUT_PROFILE_SCRIPT.apply_unsigned_curve(0.0, 1.0)).is_equal(0.0)
	assert_float(INPUT_PROFILE_SCRIPT.apply_unsigned_curve(1.0, 3.0)).is_equal(1.0)
	assert_float(INPUT_PROFILE_SCRIPT.apply_unsigned_curve(-0.5, 2.0)).is_equal(0.0)
	assert_float(INPUT_PROFILE_SCRIPT.apply_unsigned_curve(0.5, 2.0)).is_equal(0.25)


func test_axis_deadzone_remaps_full_range_and_filters_inside() -> void:
	assert_float(INPUT_PROFILE_SCRIPT.remap_axis_deadzone(0.05, 0.08)).is_equal(0.0)
	assert_float(INPUT_PROFILE_SCRIPT.remap_axis_deadzone(-0.05, 0.08)).is_equal(0.0)
	assert_float(INPUT_PROFILE_SCRIPT.remap_axis_deadzone(1.0, 0.08)).is_equal(1.0)
	assert_float(INPUT_PROFILE_SCRIPT.remap_axis_deadzone(-1.0, 0.08)).is_equal(-1.0)
	assert_float(INPUT_PROFILE_SCRIPT.remap_axis_deadzone(0.54, 0.08)).is_between(0.499, 0.501)


func test_surface_band_selection_prefers_piano_and_ignores_walls_and_metal() -> void:
	var off_codes := [2, 3, 4, 5]
	assert_str(INPUT_PROFILE_SCRIPT.surface_band_for_wheel_codes([0, 0, 0, 0], 1, off_codes)).is_equal("")
	assert_str(INPUT_PROFILE_SCRIPT.surface_band_for_wheel_codes([0, 2, 0, 0], 1, off_codes)).is_equal("off_asphalt")
	assert_str(INPUT_PROFILE_SCRIPT.surface_band_for_wheel_codes([0, 3, 4, 5], 1, off_codes)).is_equal("off_asphalt")
	assert_str(INPUT_PROFILE_SCRIPT.surface_band_for_wheel_codes([0, 1, 2, 0], 1, off_codes)).is_equal("piano")
	assert_str(INPUT_PROFILE_SCRIPT.surface_band_for_wheel_codes([6, 7, 0, 6], 1, off_codes)).is_equal("")
