extends GdUnitTestSuite

const PANEL_SCRIPT := preload("res://scripts/hud/engine_temperature_panel.gd")
const LAP_TIMING_SCRIPT := preload("res://scripts/runtime/lap_timing_controller.gd")


class FuelStubVehicle:
	extends Node

	var remaining_kg := 55.0
	var capacity_kg := 110.0

	func get_fuel_state_snapshot() -> Dictionary:
		return {
			"schema_version": 1,
			"remaining_kg": remaining_kg,
			"capacity_kg": capacity_kg,
		}


func test_fuel_label_shows_remaining_kilograms() -> void:
	var panel := _bound_panel(55.0)

	assert_str(panel._fuel_label.text).is_equal("FUEL  55.0 kg")
	assert_bool(panel._fuel_label.get_theme_color("font_color") == panel._settings.fuel_color).is_true()


func test_fuel_label_warns_when_low() -> void:
	var panel := _bound_panel(10.0)

	assert_str(panel._fuel_label.text).is_equal("FUEL  10.0 kg")
	assert_bool(panel._fuel_label.get_theme_color("font_color") == panel._settings.fuel_low_color).is_true()


func test_fuel_label_alerts_when_critical() -> void:
	var panel := _bound_panel(3.0)

	assert_str(panel._fuel_label.text).is_equal("FUEL  3.0 kg")
	assert_bool(panel._fuel_label.get_theme_color("font_color") == panel._settings.fuel_critical_color).is_true()


func test_fuel_label_shows_placeholder_without_vehicle() -> void:
	var panel := auto_free(PANEL_SCRIPT.new()) as EngineTemperaturePanel
	add_child(panel)
	panel._refresh_fuel()

	assert_str(panel._fuel_label.text).is_equal("FUEL  ---")


func test_consumption_rows_show_average_and_delta() -> void:
	var panel := _bound_panel(55.0)
	var lap_timing := auto_free(LAP_TIMING_SCRIPT.new()) as LapTimingController
	lap_timing.has_consumption_average = true
	lap_timing.average_consumption_kg_per_lap = 2.61
	lap_timing.has_laps_delta = true
	lap_timing.laps_delta = -0.5
	panel.bind_lap_timing(lap_timing)
	panel._refresh_consumption()

	assert_str(panel._average_consumption_label.text).is_equal("AVG  2.61 kg/l")
	assert_str(panel._laps_delta_label.text).is_equal("DELTA  -0.50 laps")
	assert_bool(
		panel._laps_delta_label.get_theme_color("font_color")
		== panel._settings.delta_warning_color).is_true()


func test_delta_on_plan_uses_the_neutral_color() -> void:
	var panel := _bound_panel(55.0)
	var lap_timing := auto_free(LAP_TIMING_SCRIPT.new()) as LapTimingController
	lap_timing.has_consumption_average = true
	lap_timing.average_consumption_kg_per_lap = 2.5
	lap_timing.has_laps_delta = true
	lap_timing.laps_delta = 0.12
	panel.bind_lap_timing(lap_timing)
	panel._refresh_consumption()

	assert_str(panel._laps_delta_label.text).is_equal("DELTA  +0.12 laps")
	assert_bool(
		panel._laps_delta_label.get_theme_color("font_color")
		== panel._settings.delta_neutral_color).is_true()


func test_delta_critical_uses_the_critical_color() -> void:
	var panel := _bound_panel(55.0)
	var lap_timing := auto_free(LAP_TIMING_SCRIPT.new()) as LapTimingController
	lap_timing.has_consumption_average = true
	lap_timing.average_consumption_kg_per_lap = 3.6
	lap_timing.has_laps_delta = true
	lap_timing.laps_delta = -1.4
	panel.bind_lap_timing(lap_timing)
	panel._refresh_consumption()

	assert_bool(
		panel._laps_delta_label.get_theme_color("font_color")
		== panel._settings.delta_critical_color).is_true()


func test_consumption_rows_show_placeholder_without_lap_timing() -> void:
	var panel := _bound_panel(55.0)
	panel._refresh_consumption()

	assert_str(panel._average_consumption_label.text).is_equal("AVG  --")
	assert_str(panel._laps_delta_label.text).is_equal("DELTA  --")


func _bound_panel(remaining_kg: float) -> EngineTemperaturePanel:
	var vehicle := auto_free(FuelStubVehicle.new()) as FuelStubVehicle
	vehicle.remaining_kg = remaining_kg
	var panel := auto_free(PANEL_SCRIPT.new()) as EngineTemperaturePanel
	add_child(panel)
	panel.bind_vehicle(vehicle)
	panel._refresh_fuel()
	return panel
