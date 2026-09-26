class_name LapTimingPanel
extends PanelContainer

var _settings := HudConfig.LapTimingSettings.new()
var _controller: LapTimingController
var _lap_value_label: Label
var _last_lap_value_label: Label
var _best_lap_value_label: Label

func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	_apply_layout()
	_build_ui()
	_refresh()

func bind_lap_timing(controller: LapTimingController) -> void:
	_controller = controller
	_refresh()

func apply_settings(settings: HudConfig.LapTimingSettings) -> void:
	_settings = settings
	_apply_layout()
	if is_node_ready():
		_rebuild_ui()

func _process(_delta: float) -> void:
	_refresh()

func _apply_layout() -> void:
	custom_minimum_size = _settings.size
	size = _settings.size
	scale = Vector2(_settings.scale, _settings.scale)
	visible = _settings.visible

func _rebuild_ui() -> void:
	for child in get_children():
		child.free()
	_build_ui()
	_refresh()

func _build_ui() -> void:
	var margin := MarginContainer.new()
	margin.add_theme_constant_override("margin_left", int(_settings.margin_left))
	margin.add_theme_constant_override("margin_right", int(_settings.margin_right))
	margin.add_theme_constant_override("margin_top", int(_settings.margin_top))
	margin.add_theme_constant_override("margin_bottom", int(_settings.margin_bottom))
	add_child(margin)

	var root_box := VBoxContainer.new()
	root_box.add_theme_constant_override("separation", int(_settings.row_separation))
	margin.add_child(root_box)

	var title_label := Label.new()
	title_label.text = _settings.title
	title_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	root_box.add_child(title_label)

	_lap_value_label = _add_row(root_box, "LAP")
	_last_lap_value_label = _add_row(root_box, "LAST")
	_best_lap_value_label = _add_row(root_box, "BEST")

func _add_row(root_box: VBoxContainer, caption: String) -> Label:
	var row := HBoxContainer.new()
	root_box.add_child(row)
	var caption_label := Label.new()
	caption_label.text = caption
	caption_label.custom_minimum_size.x = _settings.caption_width
	row.add_child(caption_label)
	var value_label := Label.new()
	value_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	value_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(value_label)
	return value_label

func _refresh() -> void:
	if _lap_value_label == null:
		return
	if _controller == null or not _controller.is_configured:
		_lap_value_label.text = "--"
		_last_lap_value_label.text = "--:--.---"
		_best_lap_value_label.text = "--:--.---"
		_apply_value_colors(false, false)
		return
	_lap_value_label.text = str(_controller.current_lap_number)
	_last_lap_value_label.text = _controller.format_lap_time(_controller.last_lap_time_seconds)
	_best_lap_value_label.text = _controller.format_lap_time(_controller.best_lap_time_seconds)
	_apply_value_colors(_controller.last_lap_time_seconds >= 0.0, _controller.best_lap_time_seconds >= 0.0)

func _apply_value_colors(has_last_lap: bool, has_best_lap: bool) -> void:
	_lap_value_label.add_theme_color_override(
		"font_color",
		_settings.best_lap_color if has_best_lap else _settings.pending_color)
	_last_lap_value_label.add_theme_color_override(
		"font_color",
		_settings.last_lap_color if has_last_lap else _settings.pending_color)
	_best_lap_value_label.add_theme_color_override(
		"font_color",
		_settings.best_lap_color if has_best_lap else _settings.pending_color)
