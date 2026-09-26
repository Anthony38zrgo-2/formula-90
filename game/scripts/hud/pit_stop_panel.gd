class_name PitStopPanel
extends PanelContainer

const COMPLETE_NOTICE_SECONDS := 2.5

var _controller: PitStopController
var _settings := HudConfig.PitStopSettings.new()
var _title_label: Label
var _hint_label: Label
var _compound_label: Label
var _fuel_label: Label
var _status_label: Label
var _complete_notice_remaining := 0.0


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	_apply_layout()
	_build_ui()
	_refresh()


func apply_settings(settings: HudConfig.PitStopSettings) -> void:
	_settings = settings
	_apply_layout()
	if is_node_ready():
		_rebuild_ui()
		_refresh()


func bind_pit_stop(controller: PitStopController) -> void:
	if _controller != null:
		_controller.pit_lane_entered.disconnect(_on_pit_lane_state_changed)
		_controller.pit_lane_exited.disconnect(_on_pit_lane_state_changed)
		_controller.selection_changed.disconnect(_on_selection_changed)
		_controller.service_started.disconnect(_on_service_started)
		_controller.service_progress.disconnect(_on_service_progress)
		_controller.service_completed.disconnect(_on_service_completed)
	_controller = controller
	if _controller != null:
		_controller.pit_lane_entered.connect(_on_pit_lane_state_changed)
		_controller.pit_lane_exited.connect(_on_pit_lane_state_changed)
		_controller.selection_changed.connect(_on_selection_changed)
		_controller.service_started.connect(_on_service_started)
		_controller.service_progress.connect(_on_service_progress)
		_controller.service_completed.connect(_on_service_completed)
	_refresh()


func _process(delta: float) -> void:
	if _complete_notice_remaining > 0.0:
		_complete_notice_remaining = maxf(_complete_notice_remaining - delta, 0.0)
	_refresh()


func _on_pit_lane_state_changed() -> void:
	_refresh()


func _on_selection_changed(_selection: Dictionary) -> void:
	_refresh()


func _on_service_started(_plan: Dictionary) -> void:
	_complete_notice_remaining = 0.0
	_refresh()


func _on_service_progress(_status: Dictionary) -> void:
	_refresh()


func _on_service_completed() -> void:
	_complete_notice_remaining = COMPLETE_NOTICE_SECONDS
	_refresh()


func _apply_layout() -> void:
	custom_minimum_size = _settings.size
	size = _settings.size
	scale = Vector2(_settings.scale, _settings.scale)


func _rebuild_ui() -> void:
	for child in get_children():
		child.free()
	_build_ui()


func _build_ui() -> void:
	var margin := MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 10)
	margin.add_theme_constant_override("margin_right", 10)
	margin.add_theme_constant_override("margin_top", 6)
	margin.add_theme_constant_override("margin_bottom", 6)
	add_child(margin)

	var root_box := VBoxContainer.new()
	root_box.add_theme_constant_override("separation", 2)
	margin.add_child(root_box)

	_title_label = Label.new()
	_title_label.text = _settings.title
	_title_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	root_box.add_child(_title_label)

	_hint_label = Label.new()
	_hint_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	root_box.add_child(_hint_label)

	_compound_label = Label.new()
	_compound_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	root_box.add_child(_compound_label)

	_fuel_label = Label.new()
	_fuel_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	root_box.add_child(_fuel_label)

	_status_label = Label.new()
	_status_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	root_box.add_child(_status_label)


func _refresh() -> void:
	if _title_label == null:
		return
	var is_active := _controller != null and _controller.is_selection_active()
	visible = _settings.visible and is_active
	if not is_active:
		return
	var selection := _controller.get_selection()
	var fuel_laps := int(selection.get("fuel_laps", 0))
	var fuel_target_kg := float(selection.get("fuel_target_kg", 0.0))
	var compound_label := str(selection.get("compound_label", ""))
	var active_field := int(selection.get("field", PitStopController.SELECTION_FIELD_FUEL))
	_hint_label.text = "BOX %d  —  DETENERSE EN EL RECUADRO" % (_controller.get_assigned_box_index() + 1)
	_hint_label.add_theme_color_override("font_color", _settings.hint_color)
	_compound_label.text = "%s NEUMÁTICO  %s" % [
		">" if active_field == PitStopController.SELECTION_FIELD_COMPOUND else " ",
		compound_label,
	]
	_compound_label.add_theme_color_override(
		"font_color",
		_settings.active_field_color if active_field == PitStopController.SELECTION_FIELD_COMPOUND else _settings.field_color)
	_fuel_label.text = "%s GASOLINA  %d VUELTAS (%.1f kg)" % [
		">" if active_field == PitStopController.SELECTION_FIELD_FUEL else " ",
		fuel_laps,
		fuel_target_kg,
	]
	_fuel_label.add_theme_color_override(
		"font_color",
		_settings.active_field_color if active_field == PitStopController.SELECTION_FIELD_FUEL else _settings.fuel_value_color)
	_refresh_status(selection)


func _refresh_status(selection: Dictionary) -> void:
	if _complete_notice_remaining > 0.0:
		_status_label.text = "SERVICIO COMPLETO — LIBERADO EN NEUTRO"
		_status_label.add_theme_color_override("font_color", _settings.complete_color)
		return
	if _controller.is_servicing():
		var status := _controller.get_service_status()
		var phase := int(status.get("phase", PitStopController.SERVICE_PHASE_NONE))
		if phase == PitStopController.SERVICE_PHASE_TIRES:
			_status_label.text = "CAMBIO DE NEUMÁTICOS  %.1f s" % float(status.get("tire_seconds_remaining", 0.0))
		else:
			_status_label.text = "RECARGA  %.1f s  (%.1f kg)" % [
				float(status.get("fuel_seconds_remaining", 0.0)),
				float(status.get("fill_kg", 0.0)),
			]
		_status_label.add_theme_color_override("font_color", _settings.progress_color)
		return
	if bool(selection.get("confirmed", false)):
		_status_label.text = "SELECCIÓN CONFIRMADA"
		_status_label.add_theme_color_override("font_color", _settings.complete_color)
		return
	_status_label.text = "W/S CAMPO   Q/E AJUSTA   ENTER CONFIRMA"
	_status_label.add_theme_color_override("font_color", _settings.hint_color)
