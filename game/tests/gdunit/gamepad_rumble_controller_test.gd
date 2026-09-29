extends GdUnitTestSuite

const RUMBLE_SCRIPT := preload("res://scripts/input/gamepad_rumble_controller.gd")
const INPUT_PROFILE_SCRIPT := preload("res://scripts/input/input_profile.gd")

const FRONT_AXLE_DISTANCE_M := 1.6
const REAR_AXLE_DISTANCE_M := 1.4
const FORWARD_SPEED_MS := 20.0


func test_hysteresis_engages_above_and_releases_below_its_own_threshold() -> void:
	assert_bool(RUMBLE_SCRIPT._apply_hysteresis(false, 0.09, 0.10, 0.075)).is_false()
	assert_bool(RUMBLE_SCRIPT._apply_hysteresis(false, 0.11, 0.10, 0.075)).is_true()
	assert_bool(RUMBLE_SCRIPT._apply_hysteresis(true, 0.08, 0.10, 0.075)).is_true()
	assert_bool(RUMBLE_SCRIPT._apply_hysteresis(true, 0.07, 0.10, 0.075)).is_false()


func test_slip_estimates_are_both_zero_in_a_perfect_kinematic_corner() -> void:
	var velocity := Vector3(-0.35, 0.0, -FORWARD_SPEED_MS)
	var yaw_rate := 0.25
	var steer := atan2(yaw_rate * (FRONT_AXLE_DISTANCE_M + REAR_AXLE_DISTANCE_M), FORWARD_SPEED_MS)
	var front := RUMBLE_SCRIPT.front_slip_angle_rad(velocity, yaw_rate, FRONT_AXLE_DISTANCE_M, REAR_AXLE_DISTANCE_M, steer)
	var rear := RUMBLE_SCRIPT.rear_slip_angle_rad(velocity, yaw_rate, REAR_AXLE_DISTANCE_M)
	assert_float(front).is_between(-0.001, 0.001)
	assert_float(rear).is_between(-0.001, 0.001)


func test_plowing_straight_ahead_reports_front_limit_not_rear() -> void:
	var velocity := Vector3(0.0, 0.0, -FORWARD_SPEED_MS)
	var steer := 0.075
	var front := RUMBLE_SCRIPT.front_slip_angle_rad(velocity, 0.0, FRONT_AXLE_DISTANCE_M, REAR_AXLE_DISTANCE_M, steer)
	var rear := RUMBLE_SCRIPT.rear_slip_angle_rad(velocity, 0.0, REAR_AXLE_DISTANCE_M)
	assert_float(absf(front)).is_between(0.074, 0.076)
	assert_float(absf(rear)).is_between(-0.0001, 0.0001)


func test_spun_rear_reports_rear_limit_ahead_of_front() -> void:
	var velocity := Vector3(0.8, 0.0, -FORWARD_SPEED_MS)
	var yaw_rate := 0.65
	var steer := 0.075
	var front := RUMBLE_SCRIPT.front_slip_angle_rad(velocity, yaw_rate, FRONT_AXLE_DISTANCE_M, REAR_AXLE_DISTANCE_M, steer)
	var rear := RUMBLE_SCRIPT.rear_slip_angle_rad(velocity, yaw_rate, REAR_AXLE_DISTANCE_M)
	assert_bool(absf(rear) > absf(front)).is_true()


func test_slip_estimates_return_zero_when_not_driving_forward() -> void:
	var stationary := Vector3.ZERO
	assert_float(RUMBLE_SCRIPT.front_slip_angle_rad(stationary, 0.0, FRONT_AXLE_DISTANCE_M, REAR_AXLE_DISTANCE_M, 0.05)).is_equal(0.0)
	assert_float(RUMBLE_SCRIPT.rear_slip_angle_rad(Vector3(0.0, 0.0, 10.0), 0.5, REAR_AXLE_DISTANCE_M)).is_equal(0.0)


func test_canonical_vibration_bands_distinguish_surface_responses() -> void:
	var profile: InputProfile = INPUT_PROFILE_SCRIPT.load_from_path("res://data/input/input_profile.json")
	assert_bool(profile.is_valid()).is_true()
	var piano: Dictionary = profile.vibration_settings["piano"]
	var off_asphalt: Dictionary = profile.vibration_settings["off_asphalt"]
	var front_band: Dictionary = profile.vibration_settings["front_grip_limit"]
	var rear_band: Dictionary = profile.vibration_settings["rear_grip_limit"]
	assert_bool(absf(float(piano["strong_motor_magnitude"]) - float(off_asphalt["strong_motor_magnitude"])) > 0.1).is_true()
	assert_bool(absf(float(piano["pulses_per_second"]) - float(off_asphalt["pulses_per_second"])) > 1.0).is_true()
	assert_bool(float(off_asphalt["weak_motor_magnitude"]) < 0.5).is_true()
	assert_bool(float(off_asphalt["strong_motor_magnitude"]) < 0.5).is_true()
	assert_bool(float(front_band["strong_motor_magnitude"]) > float(front_band["weak_motor_magnitude"])
		and float(rear_band["weak_motor_magnitude"]) > float(rear_band["strong_motor_magnitude"])).is_true()
	assert_bool(absf(float(front_band["weak_motor_magnitude"]) - float(rear_band["weak_motor_magnitude"])) > 0.1).is_true()
