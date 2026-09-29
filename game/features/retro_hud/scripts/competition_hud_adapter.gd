class_name CompetitionHudAdapter
extends "res://scripts/hud/arcade_race_hud.gd"

var _lap_timing_controller: LapTimingController


func bind_runtime(
		vehicle: Node,
		aids: Node,
		lap_timing_controller: LapTimingController = null,
		pit_stop_controller: PitStopController = null) -> void:
	super.bind_runtime(vehicle, aids, lap_timing_controller, pit_stop_controller)
	_lap_timing_controller = lap_timing_controller


func _ready() -> void:
	super._ready()
	if engine_temperature_panel != null:
		engine_temperature_panel.visible = false


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
		has_fuel_delta)


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
