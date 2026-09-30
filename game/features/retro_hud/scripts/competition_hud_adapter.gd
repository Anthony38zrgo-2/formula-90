class_name CompetitionHudAdapter
extends "res://scripts/hud/arcade_race_hud.gd"

const DATTO_REGULAR_FONT := preload("res://fonts/D-DIN.ttf")
const STANDARD_FONT_SIZE := 18

var _lap_timing_controller: LapTimingController


func bind_runtime(
		vehicle: Node,
		aids: Node,
		lap_timing_controller: LapTimingController = null,
		pit_stop_controller: PitStopController = null) -> void:
	super.bind_runtime(vehicle, aids, lap_timing_controller, pit_stop_controller)
	_lap_timing_controller = lap_timing_controller


func _ready() -> void:
	var display_theme := Theme.new()
	display_theme.default_font = DATTO_REGULAR_FONT
	display_theme.default_font_size = STANDARD_FONT_SIZE
	theme = display_theme
	super._ready()
	if engine_temperature_panel != null:
		engine_temperature_panel.visible = false
	_apply_competition_panel_backgrounds()
	_position_vehicle_status_panels()


func _apply_competition_panel_backgrounds() -> void:
	var display := retro_hud as RetroHudDisplay
	if display == null:
		return
	var background_style := StyleBoxFlat.new()
	background_style.bg_color = display.config.background_color
	var lap_and_map_background := get_node_or_null("LapAndMapBackground") as ColorRect
	if lap_and_map_background != null:
		lap_and_map_background.color = display.config.background_color
	var track_minimap := get_node_or_null("Minimap") as TrackMinimapController
	if track_minimap != null:
		track_minimap.background_color = Color.TRANSPARENT
		track_minimap.queue_redraw()
	if lap_timing_panel != null:
		lap_timing_panel.add_theme_stylebox_override("panel", StyleBoxEmpty.new())
	var handling_panel := get_node_or_null("HandlingTuningPanel/LiveTuningPanel") as PanelContainer
	for panel in [
		tire_status_panel,
		pit_stop_panel,
		engine_temperature_panel,
		handling_panel,
	]:
		if panel is PanelContainer:
			panel.add_theme_stylebox_override("panel", background_style)
	_position_lap_timing_panel()


func _position_lap_timing_panel() -> void:
	if lap_timing_panel == null:
		return
	var track_minimap := get_node_or_null("Minimap") as TrackMinimapController
	var lap_and_map_background := get_node_or_null("LapAndMapBackground") as ColorRect
	if track_minimap == null or lap_and_map_background == null:
		super._position_lap_timing_panel()
		return

	var timing_visual_size := _visual_panel_size(lap_timing_panel)
	var map_visual_size := track_minimap.size * track_minimap.scale
	var content_width := maxf(timing_visual_size.x, map_visual_size.x)
	var block_padding := _hud_config.lap_timing.block_padding
	var timing_height := timing_visual_size.y if lap_timing_panel.visible else 0.0
	var map_gap := _hud_config.lap_timing.map_gap if lap_timing_panel.visible else 0.0
	var block_position := Vector2(
		_hud_config.lap_timing.margin_left,
		_hud_config.lap_timing.margin_top)
	var content_left := block_position.x + block_padding
	var content_top := block_position.y + block_padding

	lap_timing_panel.global_position = Vector2(
		content_left + (content_width - timing_visual_size.x) * 0.5,
		content_top)
	track_minimap.global_position = Vector2(
		content_left + (content_width - map_visual_size.x) * 0.5,
		content_top + timing_height + map_gap)
	lap_and_map_background.global_position = block_position
	lap_and_map_background.size = Vector2(
		content_width + block_padding * 2.0,
		timing_height + map_gap + map_visual_size.y + block_padding * 2.0)
	lap_and_map_background.visible = lap_timing_panel.visible or track_minimap.visible


func _update_speed_gauge() -> void:
	super._update_speed_gauge()
	if retro_hud == null:
		return
	if _vehicle == null:
		retro_hud.call(
			"set_competition_readout",
			0.0,
			0.0,
			"N",
			-1.0,
			-1.0,
			-1.0,
			0.0,
			0.0,
			false,
			0.0,
			false)
		return

	var speed_value: Variant = _vehicle.get("speed")
	var speed_meters_per_second := float(speed_value) if speed_value != null else 0.0
	var gear_value: Variant = _vehicle.get("current_gear")
	var gear_number := int(gear_value) if gear_value != null else 0
	var gear_label := "R" if gear_number < 0 else ("N" if gear_number == 0 else str(gear_number))
	var revolutions_per_minute_value: Variant = _vehicle.get("motor_rpm")
	var engine_revolutions_per_minute := (
		float(revolutions_per_minute_value)
		if revolutions_per_minute_value != null
		else 0.0)
	var thermal_state := _engine_thermal_state()
	var oil_temperature_celsius := _temperature_from_state(thermal_state, "oil")
	var water_temperature_celsius := _temperature_from_state(thermal_state, "water")
	var oil_temperature_color := _engine_temperature_color(thermal_state, "oil", oil_temperature_celsius)
	var water_temperature_color := _engine_temperature_color(thermal_state, "water", water_temperature_celsius)
	var fuel_state := _fuel_state()
	var fuel_remaining_kg := _finite_value(fuel_state.get("remaining_kg"), -1.0)
	var fuel_capacity_kg := _finite_value(fuel_state.get("capacity_kg"), 0.0)
	var has_average_consumption := (
		_lap_timing_controller != null
		and _lap_timing_controller.has_consumption_average)
	var average_consumption_kg_per_lap := (
		_lap_timing_controller.average_consumption_kg_per_lap
		if has_average_consumption
		else 0.0)
	var has_fuel_delta := (
		_lap_timing_controller != null
		and _lap_timing_controller.has_laps_delta)
	var fuel_delta_laps := (
		_lap_timing_controller.laps_delta
		if has_fuel_delta
		else 0.0)

	retro_hud.call(
		"set_competition_readout",
		absf(speed_meters_per_second) * 3.6,
		engine_revolutions_per_minute,
		gear_label,
		oil_temperature_celsius,
		water_temperature_celsius,
		fuel_remaining_kg,
		fuel_capacity_kg,
		average_consumption_kg_per_lap,
		has_average_consumption,
		fuel_delta_laps,
		has_fuel_delta,
		oil_temperature_color,
		water_temperature_color)


func _engine_thermal_state() -> Dictionary:
	if _vehicle == null:
		return {}
	if _vehicle.has_method(&"get_engine_thermal_state_snapshot"):
		var native_state: Variant = _vehicle.call(&"get_engine_thermal_state_snapshot")
		if native_state is Dictionary and not native_state.is_empty():
			return native_state
	if _vehicle.has_method(&"get_telemetry_snapshot"):
		var telemetry_value: Variant = _vehicle.call(&"get_telemetry_snapshot")
		if telemetry_value is Dictionary:
			var thermal_state: Variant = telemetry_value.get("engine_thermal", {})
			if thermal_state is Dictionary:
				return thermal_state
	return {}


func _temperature_from_state(thermal_state: Dictionary, system_name: String) -> float:
	var system_state: Variant = thermal_state.get(system_name, {})
	if not (system_state is Dictionary):
		return -1.0
	return _finite_value(system_state.get("temperature_c"), -1.0)


func _engine_temperature_color(thermal_state: Dictionary, system_name: String, temperature_celsius: float) -> Color:
	var system_state: Variant = thermal_state.get(system_name, {})
	if temperature_celsius < 0.0 or not (system_state is Dictionary):
		return _hud_config.engine_temperatures.cold_color
	var temperature_settings := _hud_config.engine_temperatures
	if not system_state.has("optimal_min_c"):
		return temperature_settings.optimal_color
	if temperature_celsius < float(system_state.get("optimal_min_c")):
		return temperature_settings.cold_color
	if temperature_celsius <= float(system_state.get("optimal_max_c", INF)):
		return temperature_settings.optimal_color
	if temperature_celsius < float(system_state.get("derating_c", INF)):
		return temperature_settings.warm_color
	if temperature_celsius < float(system_state.get("critical_c", INF)):
		return temperature_settings.hot_color
	return temperature_settings.critical_color


func _fuel_state() -> Dictionary:
	if _vehicle == null or not _vehicle.has_method(&"get_fuel_state_snapshot"):
		return {}
	var fuel_state_value: Variant = _vehicle.call(&"get_fuel_state_snapshot")
	if fuel_state_value is Dictionary:
		return fuel_state_value
	return {}


func _finite_value(value: Variant, fallback_value: float) -> float:
	if value == null:
		return fallback_value
	var numeric_value := float(value)
	return numeric_value if is_finite(numeric_value) else fallback_value
