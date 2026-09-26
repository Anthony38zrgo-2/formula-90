extends GdUnitTestSuite

const PIT_STOP_PANEL_SCRIPT := preload("res://scripts/hud/pit_stop_panel.gd")
const PIT_STOP_SCRIPT := preload("res://scripts/runtime/pit_stop_controller.gd")
const PIT_STOP_RULES_SCRIPT := preload("res://scripts/runtime/pit_stop_rules.gd")
const FUJI_TRACK_DEFINITION := "res://data/tracks/fuji76_77.tres"
const F1_2030_VEHICLE_DEFINITION := "res://data/vehicles/f1_2030_v10.tres"


class PanelStubVehicle:
	extends Node3D

	var remaining_kg := 7.6

	func get_fuel_state_snapshot() -> Dictionary:
		return {
			"schema_version": 1,
			"remaining_kg": remaining_kg,
			"capacity_kg": 110.0,
		}


func test_panel_shows_soft_compound_and_fifteen_laps_on_entry() -> void:
	var controller := _controller_with_selection()
	var panel := _bound_panel(controller)
	panel.call("_refresh")

	assert_bool(panel.visible).is_true()
	assert_str(panel._compound_label.text).contains("BLANDOS")
	assert_str(panel._fuel_label.text).contains("15 VUELTAS")
	assert_str(panel._fuel_label.text).contains("37.9 kg")
	assert_str(panel._hint_label.text).contains("BOX 1")


func test_panel_hides_outside_the_pit_lane() -> void:
	var controller := _controller_with_selection()
	controller.in_pit_lane = false
	var panel := _bound_panel(controller)
	panel.call("_refresh")

	assert_bool(panel.visible).is_false()


func test_panel_reports_the_tire_phase_progress() -> void:
	var controller := _controller_with_selection()
	var panel := _bound_panel(controller)
	controller._begin_service()
	controller._advance_service(1.2)
	panel.call("_refresh")

	assert_str(panel._status_label.text).contains("NEUMÁTICOS")
	assert_str(panel._status_label.text).contains("1.8")


func test_panel_reports_the_fuel_phase_progress() -> void:
	var controller := _controller_with_selection()
	var panel := _bound_panel(controller)
	controller._begin_service()
	controller._advance_service(3.6)
	panel.call("_refresh")

	assert_str(panel._status_label.text).contains("RECARGA")


func test_panel_announces_the_completed_service() -> void:
	var controller := _controller_with_selection()
	var panel := _bound_panel(controller)
	controller._begin_service()
	controller._advance_service(3.1)
	controller._advance_service(6.2)
	panel.call("_refresh")

	assert_str(panel._status_label.text).contains("SERVICIO COMPLETO")


func _controller_with_selection() -> PitStopController:
	var vehicle := auto_free(PanelStubVehicle.new()) as PanelStubVehicle
	var controller := auto_free(PIT_STOP_SCRIPT.new()) as PitStopController
	var configured := controller.configure(
		vehicle,
		load(FUJI_TRACK_DEFINITION) as TrackDefinition,
		load(F1_2030_VEHICLE_DEFINITION) as VehicleDefinition,
		PIT_STOP_RULES_SCRIPT.load_from_json())
	assert_bool(configured).is_true()
	controller.in_pit_lane = true
	return controller


func _bound_panel(controller: PitStopController) -> PitStopPanel:
	var panel := auto_free(PIT_STOP_PANEL_SCRIPT.new()) as PitStopPanel
	panel.apply_settings(HudConfig.PitStopSettings.new())
	add_child(panel)
	panel.bind_pit_stop(controller)
	return panel
