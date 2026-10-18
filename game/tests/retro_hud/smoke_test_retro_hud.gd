extends SceneTree

class ProfileVehicle extends Node:
	var physics_config_path := "res://data/vehicles/f1_1997/williams/profiles/hockenheim/vehicle.json"


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var failures: Array[String] = []
	var display_scene := load("res://features/retro_hud/scenes/retro_hud_display.tscn") as PackedScene
	if display_scene == null:
		printerr("[FAIL] Retro HUD display scene could not load.")
		quit(1)
		return
	var display := display_scene.instantiate()
	root.add_child(display)
	await process_frame

	if display.get("state") == null:
		failures.append("procedural retro HUD is missing its state object")
	if not display.has_method("set_readout"):
		failures.append("procedural retro HUD is missing the decoupled readout API")
	if display.get("config") == null:
		failures.append("procedural retro HUD is missing its config")

	display.set_readout(254.4, 11950.0, "4", 0.7, 0.3)
	await process_frame
	var state: RefCounted = display.get("state")
	if not is_equal_approx(state.speed_kph, 254.4):
		failures.append("state did not store the speed readout")
	if not is_equal_approx(state.rpm, 11950.0):
		failures.append("state did not store the RPM readout")
	if state.gear_label != "4":
		failures.append("state did not store the gear readout")
	if not is_equal_approx(state.throttle, 0.7):
		failures.append("state did not store the throttle readout")
	if not is_equal_approx(state.brake, 0.3):
		failures.append("state did not store the brake readout")
	var peak_rpm: float = display.get("_peak_rpm")
	if peak_rpm < state.rpm:
		failures.append("peak-hold RPM did not advance to the live RPM")

	var config: RefCounted = display.get("config")
	if config.rpm_max <= config.rpm_min:
		failures.append("RPM scale is invalid (max <= min)")
	if config.rpm_redline < config.rpm_min or config.rpm_redline > config.rpm_max:
		failures.append("redline is outside the RPM scale")
	if config.speed_segments < 4 or config.speed_segments > 19:
		failures.append("speed segment count is outside the supported 4-19 range")
	if config.peak_activation_rpm < config.rpm_min or config.peak_activation_rpm > config.rpm_max:
		failures.append("peak activation RPM is outside the RPM scale")

	var adapter := CompetitionHudAdapter.new()
	var profile_vehicle := ProfileVehicle.new()
	adapter.retro_hud = display
	adapter.set("_vehicle", profile_vehicle)
	adapter.call("_synchronize_vehicle_powertrain_configuration")
	_check_vehicle_limits(display, 17500.0, failures)
	display.reload_config()
	_check_vehicle_limits(display, 17500.0, failures)
	profile_vehicle.physics_config_path = "res://data/vehicles/f1_2030/profiles/geometric/vehicle.json"
	adapter.call("_synchronize_vehicle_powertrain_configuration")
	_check_vehicle_limits(display, 18000.0, failures)
	adapter.set("_vehicle", null)
	adapter.call("_synchronize_vehicle_powertrain_configuration")
	if not is_equal_approx(display.config.revolutions_per_minute_maximum, 19000.0):
		failures.append("unbinding the vehicle did not restore HUD JSON defaults")
	adapter.free()
	profile_vehicle.free()
	display.queue_free()
	if failures.is_empty():
		print("[PASS] Retro HUD loads its procedural display, state, and validated config.")
	else:
		for failure in failures:
			printerr("[FAIL] " + failure)
	quit(failures.size())


func _check_vehicle_limits(display: RetroHudDisplay, engine_limit: float, failures: Array[String]) -> void:
	if not is_equal_approx(display.config.revolutions_per_minute_redline, engine_limit):
		failures.append("HUD redline does not match the resolved vehicle JSON")
	if not is_equal_approx(display.config.revolutions_per_minute_maximum, engine_limit * 19000.0 / 18000.0):
		failures.append("tachometer scale did not follow the vehicle limit")
	if not is_equal_approx(display.config.ideal_shift_revolutions_per_minute, engine_limit * 17400.0 / 18000.0):
		failures.append("ideal shift light did not retain the HUD JSON proportion")
	if not is_equal_approx(display.config.urgent_shift_revolutions_per_minute, engine_limit * 17800.0 / 18000.0):
		failures.append("urgent shift light did not retain the HUD JSON proportion")
	display.set_readout(200.0, display.config.urgent_shift_revolutions_per_minute - 1.0, "4")
	if display.engine_readout_color() != display.config.digital_color:
		failures.append("engine readout turns red before the configured warning")
	display.set_readout(200.0, display.config.urgent_shift_revolutions_per_minute, "4")
	if display.engine_readout_color() != display.config.alert_color:
		failures.append("engine readout does not turn red at the configured warning")
	display.set_readout(200.0, engine_limit, "4")
	if display.engine_readout_color() != display.config.alert_color:
		failures.append("engine readout does not turn red at the vehicle limiter")
