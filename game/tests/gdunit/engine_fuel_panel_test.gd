extends GdUnitTestSuite

const PANEL_SCRIPT := preload("res://scripts/hud/engine_temperature_panel.gd")


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


func _bound_panel(remaining_kg: float) -> EngineTemperaturePanel:
	var vehicle := auto_free(FuelStubVehicle.new()) as FuelStubVehicle
	vehicle.remaining_kg = remaining_kg
	var panel := auto_free(PANEL_SCRIPT.new()) as EngineTemperaturePanel
	add_child(panel)
	panel.bind_vehicle(vehicle)
	panel._refresh_fuel()
	return panel
