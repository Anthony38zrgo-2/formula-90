class_name EngineTemperaturePanel
extends PanelContainer

var _settings := HudConfig.EngineTemperaturesSettings.new()
var _vehicle: Node
var _lap_timing: LapTimingController
var _water_temperature_label: Label
var _oil_temperature_label: Label
var _fuel_label: Label
var _average_consumption_label: Label
var _laps_delta_label: Label

func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	_apply_layout()
	_build_ui()
	_show_missing_temperatures()


func bind_vehicle(vehicle: Node) -> void:
	_vehicle = vehicle
	_refresh_temperatures()


func bind_lap_timing(lap_timing: LapTimingController) -> void:
	_lap_timing = lap_timing
	_refresh_consumption()


func apply_settings(settings: HudConfig.EngineTemperaturesSettings) -> void:
	_settings = settings
	_apply_layout()
	if is_node_ready():
		_rebuild_ui()


func _apply_layout() -> void:
	custom_minimum_size = _settings.size
	size = _settings.size
	scale = Vector2(_settings.scale, _settings.scale)
	visible = _settings.visible


func _rebuild_ui() -> void:
	for child in get_children():
		child.free()
	_build_ui()
	_show_missing_temperatures()


func _process(_delta: float) -> void:
	_refresh_temperatures()
	_refresh_fuel()
	_refresh_consumption()


func _refresh_consumption() -> void:
	if _average_consumption_label == null or _laps_delta_label == null:
		return
	if _lap_timing == null or not _lap_timing.has_consumption_average:
		_average_consumption_label.text = "AVG  --"
		_average_consumption_label.add_theme_color_override("font_color", Color.WHITE)
	else:
		_average_consumption_label.text = "AVG  %.2f kg/l" % _lap_timing.average_consumption_kg_per_lap
		_average_consumption_label.add_theme_color_override("font_color", _settings.fuel_color)
	if _lap_timing == null or not _lap_timing.has_laps_delta:
		_laps_delta_label.text = "DELTA  --"
		_laps_delta_label.add_theme_color_override("font_color", Color.WHITE)
	else:
		_laps_delta_label.text = "DELTA  %+.2f laps" % _lap_timing.laps_delta
		_laps_delta_label.add_theme_color_override(
			"font_color",
			_delta_color(_lap_timing.laps_delta))


func _delta_color(laps_delta: float) -> Color:
	if laps_delta <= _settings.delta_critical_laps:
		return _settings.delta_critical_color
	if laps_delta <= _settings.delta_warning_laps:
		return _settings.delta_warning_color
	return _settings.delta_neutral_color


func _refresh_fuel() -> void:
	if _vehicle == null or not _vehicle.has_method(&"get_fuel_state_snapshot"):
		_show_missing_fuel()
		return
	var fuel_state_value: Variant = _vehicle.call(&"get_fuel_state_snapshot")
	if not (fuel_state_value is Dictionary):
		_show_missing_fuel()
		return
	var fuel_state: Dictionary = fuel_state_value
	var remaining_value: Variant = fuel_state.get("remaining_kg")
	var capacity_value: Variant = fuel_state.get("capacity_kg")
	if remaining_value == null or capacity_value == null:
		_show_missing_fuel()
		return
	var remaining_kg := float(remaining_value)
	var capacity_kg := float(capacity_value)
	if not is_finite(remaining_kg) or capacity_kg <= 0.0:
		_show_missing_fuel()
		return
	_fuel_label.text = "FUEL  %.1f kg" % remaining_kg
	_fuel_label.add_theme_color_override(
		"font_color",
		_fuel_color(remaining_kg / capacity_kg))


func _fuel_color(remaining_fraction: float) -> Color:
	if remaining_fraction <= _settings.fuel_critical_fraction:
		return _settings.fuel_critical_color
	if remaining_fraction <= _settings.fuel_low_fraction:
		return _settings.fuel_low_color
	return _settings.fuel_color


func _show_missing_fuel() -> void:
	if _fuel_label == null:
		return
	_fuel_label.text = "FUEL  ---"
	_fuel_label.add_theme_color_override("font_color", Color.WHITE)


func _refresh_temperatures() -> void:
	if _vehicle == null:
		_show_missing_temperatures()
		return

	var thermal_state: Dictionary = {}
	if _vehicle.has_method(&"get_engine_thermal_state_snapshot"):
		var native_state: Variant = _vehicle.call(&"get_engine_thermal_state_snapshot")
		if native_state is Dictionary:
			thermal_state = native_state
	if thermal_state.is_empty() and _vehicle.has_method(&"get_telemetry_snapshot"):
		var telemetry_value: Variant = _vehicle.call(&"get_telemetry_snapshot")
		if telemetry_value is Dictionary:
			var fallback_state: Variant = telemetry_value.get("engine_thermal", {})
			if fallback_state is Dictionary:
				thermal_state = fallback_state

	if thermal_state.is_empty():
		_show_missing_temperatures()
		return

	var water_state: Variant = thermal_state.get("water", {})
	var oil_state: Variant = thermal_state.get("oil", {})
	if water_state is Dictionary:
		_set_temperature_label(
			_water_temperature_label,
			"WATER",
			water_state.get("temperature_c"),
			float(water_state.get("optimal_min_c", 0.0)),
			float(water_state.get("optimal_max_c", 0.0)),
			float(water_state.get("derating_c", 0.0)),
			float(water_state.get("critical_c", 0.0)))
	else:
		_show_missing_temperature(_water_temperature_label, "WATER")
	if oil_state is Dictionary:
		_set_temperature_label(
			_oil_temperature_label,
			"OIL",
			oil_state.get("temperature_c"),
			float(oil_state.get("optimal_min_c", 0.0)),
			float(oil_state.get("optimal_max_c", 0.0)),
			float(oil_state.get("derating_c", 0.0)),
			float(oil_state.get("critical_c", 0.0)))
	else:
		_show_missing_temperature(_oil_temperature_label, "OIL")


func _set_temperature_label(
	label: Label,
	name: String,
	temperature_value: Variant,
	optimal_minimum_temperature: float,
	optimal_maximum_temperature: float,
	hot_derating_temperature: float,
	critical_temperature: float) -> void:
	if label == null or temperature_value == null:
		_show_missing_temperature(label, name)
		return
	var temperature := float(temperature_value)
	if not is_finite(temperature):
		_show_missing_temperature(label, name)
		return
	label.text = "%s  %.1f°" % [name, temperature]
	label.add_theme_color_override("font_color", _temperature_color(
		temperature,
		optimal_minimum_temperature,
		optimal_maximum_temperature,
		hot_derating_temperature,
		critical_temperature))


func _show_missing_temperatures() -> void:
	_show_missing_temperature(_water_temperature_label, "WATER")
	_show_missing_temperature(_oil_temperature_label, "OIL")
	_show_missing_fuel()


func _show_missing_temperature(label: Label, name: String) -> void:
	if label == null:
		return
	label.text = "%s  ---°" % name
	label.add_theme_color_override("font_color", Color.WHITE)


func _build_ui() -> void:
	var margin := MarginContainer.new()
	margin.add_theme_constant_override("margin_left", int(_settings.margin_left))
	margin.add_theme_constant_override("margin_right", int(_settings.margin_right))
	margin.add_theme_constant_override("margin_top", int(_settings.margin_top))
	margin.add_theme_constant_override("margin_bottom", int(_settings.margin_bottom))
	add_child(margin)

	var root_box := VBoxContainer.new()
	margin.add_child(root_box)

	var title_label := Label.new()
	title_label.text = _settings.title
	title_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	root_box.add_child(title_label)

	var temperature_columns := HBoxContainer.new()
	temperature_columns.add_theme_constant_override("separation", int(_settings.column_separation))
	root_box.add_child(temperature_columns)

	_water_temperature_label = Label.new()
	_water_temperature_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_water_temperature_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	temperature_columns.add_child(_water_temperature_label)

	_oil_temperature_label = Label.new()
	_oil_temperature_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_oil_temperature_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	temperature_columns.add_child(_oil_temperature_label)

	_fuel_label = Label.new()
	_fuel_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	root_box.add_child(_fuel_label)

	_average_consumption_label = Label.new()
	_average_consumption_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	root_box.add_child(_average_consumption_label)

	_laps_delta_label = Label.new()
	_laps_delta_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	root_box.add_child(_laps_delta_label)


func _temperature_color(
	temperature: float,
	optimal_minimum_temperature: float,
	optimal_maximum_temperature: float,
	hot_derating_temperature: float,
	critical_temperature: float) -> Color:
	if temperature < optimal_minimum_temperature:
		return _settings.cold_color
	if temperature <= optimal_maximum_temperature:
		return _settings.optimal_color
	if temperature < hot_derating_temperature:
		return _settings.warm_color
	if temperature < critical_temperature:
		return _settings.hot_color
	return _settings.critical_color
