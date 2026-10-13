extends GdUnitTestSuite

const PROFILE_CONFIGURATION = preload("res://scripts/vehicle/vehicle_profile_configuration.gd")
const GEOMETRIC_PROFILE = "res://data/vehicles/f1_2030/profiles/geometric/vehicle.json"
const FLAT_PROFILE = "res://data/vehicles/f1_2030/f1_2030_v10_geometric.json"

func test_both_vehicle_definitions_select_the_manifest_and_load_fuel() -> void:
	for definition_path in ["res://data/vehicles/f1_2030_v10.tres", "res://data/vehicles/liveries/f1_2030_v10_mp4_6_senna_1.tres"]:
		var definition := load(definition_path) as VehicleDefinition
		assert_str(definition.physics_config_path).is_equal(GEOMETRIC_PROFILE)
		var fuel_plan := definition.load_fuel_plan()
		assert_float(float(fuel_plan.get("capacity_kg"))).is_equal_approx(110.0, 0.000001)
		assert_float(float(fuel_plan.get("initial_kg"))).is_equal_approx(7.6, 0.000001)
		assert_float(float(fuel_plan.get("estimated_lap_consumption_kg"))).is_equal_approx(2.53, 0.000001)
		assert_float(float(fuel_plan.get("reference_lap_time_s"))).is_equal_approx(90.0, 0.000001)

func test_manifest_and_flat_profile_preserve_declared_chassis_and_assistance() -> void:
	for profile_path in [GEOMETRIC_PROFILE, FLAT_PROFILE]:
		var configuration: Dictionary = PROFILE_CONFIGURATION.load_configuration(profile_path)
		assert_float(float(configuration["chassis"]["vehicle_mass"])).is_equal_approx(597.7950900124142, 0.000001)
		assert_float(float(configuration["chassis"]["front_weight_distribution"])).is_equal_approx(0.45, 0.000001)
		assert_float(float(configuration["aids"]["stability_yaw_strength"])).is_equal_approx(3.0, 0.000001)
		assert_float(float(configuration["aids"]["stability_grounded_multiplier"])).is_equal_approx(2.0, 0.000001)

func test_resolver_preserves_seven_gears_and_original_shift_inertia() -> void:
	var configuration: Dictionary = PROFILE_CONFIGURATION.load_configuration(GEOMETRIC_PROFILE)
	assert_int(configuration["powertrain"]["gear_ratios"].size()).is_equal(7)
	assert_float(float(configuration["powertrain"]["gear_inertia"])).is_equal_approx(0.014, 0.000001)
	assert_float(float(configuration["powertrain"]["shift_time"]) + float(configuration["powertrain"]["gear_inertia"])).is_equal_approx(0.064, 0.000001)

func test_presentation_resolution_is_explicit_and_returns_a_readable_document() -> void:
	var physical_resolution: Dictionary = PROFILE_CONFIGURATION.resolve_profile(GEOMETRIC_PROFILE)
	assert_bool(physical_resolution.has("suspension_presentation_path")).is_false()
	var presentation_resolution: Dictionary = PROFILE_CONFIGURATION.resolve_profile(GEOMETRIC_PROFILE, true)
	var presentation_path := str(presentation_resolution["suspension_presentation_path"]).replace("\\", "/").trim_prefix("//?/")
	assert_bool(FileAccess.file_exists(presentation_path)).is_true()
	var presentation: Variant = JSON.parse_string(FileAccess.get_file_as_string(presentation_path))
	assert_bool(presentation is Dictionary).is_true()
