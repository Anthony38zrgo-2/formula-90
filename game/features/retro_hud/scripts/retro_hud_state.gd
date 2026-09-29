class_name RetroHudState
extends RefCounted

signal changed

var speed_kph := 0.0
var rpm := 0.0
var gear_label := "N"
var throttle := 0.0
var brake := 0.0
var speed_kilometers_per_hour := 0.0
var engine_revolutions_per_minute := 0.0
var oil_temperature_celsius := -1.0
var water_temperature_celsius := -1.0
var oil_temperature_color := Color.WHITE
var water_temperature_color := Color.WHITE
var fuel_remaining_kg := -1.0
var fuel_capacity_kg := 0.0
var average_consumption_kg_per_lap := 0.0
var has_average_consumption := false
var fuel_delta_laps := 0.0
var has_fuel_delta := false


func set_readout(
		next_speed_kph: float,
		next_rpm: float,
		next_gear_label: String,
		next_throttle: float = 0.0,
		next_brake: float = 0.0) -> void:
	var normalized_speed := maxf(next_speed_kph, 0.0)
	var normalized_rpm := maxf(next_rpm, 0.0)
	var normalized_gear := next_gear_label if not next_gear_label.is_empty() else "N"
	var normalized_throttle := clampf(next_throttle, 0.0, 1.0)
	var normalized_brake := clampf(next_brake, 0.0, 1.0)
	if (
			is_equal_approx(speed_kph, normalized_speed)
			and is_equal_approx(rpm, normalized_rpm)
			and gear_label == normalized_gear
			and is_equal_approx(throttle, normalized_throttle)
			and is_equal_approx(brake, normalized_brake)):
		return
	speed_kph = normalized_speed
	rpm = normalized_rpm
	gear_label = normalized_gear
	throttle = normalized_throttle
	brake = normalized_brake
	speed_kilometers_per_hour = normalized_speed
	engine_revolutions_per_minute = normalized_rpm
	changed.emit()


func set_competition_readout(
		next_speed_kilometers_per_hour: float,
		next_engine_revolutions_per_minute: float,
		next_gear_label: String,
		next_oil_temperature_celsius: float,
		next_water_temperature_celsius: float,
		next_fuel_remaining_kg: float,
		next_fuel_capacity_kg: float,
		next_average_consumption_kg_per_lap: float,
		next_has_average_consumption: bool,
		next_fuel_delta_laps: float,
		next_has_fuel_delta: bool,
		next_oil_temperature_color: Color = Color.WHITE,
		next_water_temperature_color: Color = Color.WHITE) -> void:
	var normalized_speed := maxf(next_speed_kilometers_per_hour, 0.0)
	var normalized_engine_revolutions_per_minute := maxf(next_engine_revolutions_per_minute, 0.0)
	var normalized_gear_label := next_gear_label if not next_gear_label.is_empty() else "N"
	var normalized_oil_temperature := next_oil_temperature_celsius if is_finite(next_oil_temperature_celsius) else -1.0
	var normalized_water_temperature := next_water_temperature_celsius if is_finite(next_water_temperature_celsius) else -1.0
	var normalized_fuel_remaining := next_fuel_remaining_kg if is_finite(next_fuel_remaining_kg) and next_fuel_remaining_kg >= 0.0 else -1.0
	var normalized_fuel_capacity := next_fuel_capacity_kg if is_finite(next_fuel_capacity_kg) and next_fuel_capacity_kg > 0.0 else 0.0
	var normalized_average_consumption := (
		next_average_consumption_kg_per_lap
		if is_finite(next_average_consumption_kg_per_lap) and next_average_consumption_kg_per_lap >= 0.0
		else 0.0)
	var normalized_has_average_consumption := (
		next_has_average_consumption
		and is_finite(next_average_consumption_kg_per_lap)
		and next_average_consumption_kg_per_lap >= 0.0)
	var normalized_fuel_delta := next_fuel_delta_laps if is_finite(next_fuel_delta_laps) else 0.0
	var normalized_has_fuel_delta := next_has_fuel_delta and is_finite(next_fuel_delta_laps)
	if (
			is_equal_approx(speed_kilometers_per_hour, normalized_speed)
			and is_equal_approx(engine_revolutions_per_minute, normalized_engine_revolutions_per_minute)
			and gear_label == normalized_gear_label
			and is_equal_approx(oil_temperature_celsius, normalized_oil_temperature)
			and is_equal_approx(water_temperature_celsius, normalized_water_temperature)
			and oil_temperature_color == next_oil_temperature_color
			and water_temperature_color == next_water_temperature_color
			and is_equal_approx(fuel_remaining_kg, normalized_fuel_remaining)
			and is_equal_approx(fuel_capacity_kg, normalized_fuel_capacity)
			and is_equal_approx(average_consumption_kg_per_lap, normalized_average_consumption)
			and has_average_consumption == normalized_has_average_consumption
			and is_equal_approx(fuel_delta_laps, normalized_fuel_delta)
			and has_fuel_delta == normalized_has_fuel_delta):
		return
	speed_kph = normalized_speed
	rpm = normalized_engine_revolutions_per_minute
	gear_label = normalized_gear_label
	speed_kilometers_per_hour = normalized_speed
	engine_revolutions_per_minute = normalized_engine_revolutions_per_minute
	oil_temperature_celsius = normalized_oil_temperature
	water_temperature_celsius = normalized_water_temperature
	oil_temperature_color = next_oil_temperature_color
	water_temperature_color = next_water_temperature_color
	fuel_remaining_kg = normalized_fuel_remaining
	fuel_capacity_kg = normalized_fuel_capacity
	average_consumption_kg_per_lap = normalized_average_consumption
	has_average_consumption = normalized_has_average_consumption
	fuel_delta_laps = normalized_fuel_delta
	has_fuel_delta = normalized_has_fuel_delta
	changed.emit()
