extends CanvasLayer

const PREFERENCES_PATH := "user://cockpit_camera_preferences.json"

var configuration: CockpitCameraConfiguration
var camera_rig: Node3D
var load_saved_preferences := true
var default_preferences: Dictionary
var sliders: Dictionary = {}
var status_label: Label
var panel: PanelContainer

func _ready() -> void:
	visible = false
	layer = 20
	default_preferences = configuration.get_motion_preferences()
	if load_saved_preferences and FileAccess.file_exists(PREFERENCES_PATH):
		var preferences_file := FileAccess.open(PREFERENCES_PATH, FileAccess.READ)
		if preferences_file != null:
			var saved_preferences: Variant = JSON.parse_string(preferences_file.get_as_text())
			if saved_preferences is Dictionary:
				configuration.apply_motion_preferences(saved_preferences)
	panel = PanelContainer.new()
	var panel_theme := Theme.new()
	panel_theme.default_font_size = 20
	panel.theme = panel_theme
	var panel_background := StyleBoxFlat.new()
	panel_background.bg_color = Color(0.06, 0.08, 0.12, 0.96)
	panel_background.border_color = Color(0.25, 0.38, 0.55)
	panel_background.set_border_width_all(1)
	panel_background.set_corner_radius_all(6)
	panel.add_theme_stylebox_override("panel", panel_background)
	add_child(panel)
	var margins := MarginContainer.new()
	for margin_name in ["margin_left", "margin_top", "margin_right", "margin_bottom"]:
		margins.add_theme_constant_override(margin_name, 16)
	panel.add_child(margins)
	var contents := VBoxContainer.new()
	contents.add_theme_constant_override("separation", 12)
	margins.add_child(contents)
	var title := Label.new()
	title.text = "Cámara de cockpit · F9 para cerrar"
	contents.add_child(title)
	var definitions := [
		["horizon_stabilization_strength", "Estabilidad de la mirada", 1.0],
		["longitudinal_force_response_strength", "Aceleración y frenada", 2.0],
		["vertical_bump_response_strength", "Feedback de baches verticales", 2.0],
		["lateral_bump_response_strength", "Balanceo por baches", 2.0],
		["positional_stabilization_strength", "Absorción del movimiento", 1.0],
		["velocity_alignment_strength", "Mirar hacia la trayectoria", 1.0]
	]
	for definition in definitions:
		var row := HBoxContainer.new()
		contents.add_child(row)
		var label := Label.new()
		label.text = definition[1]
		label.custom_minimum_size.x = 320.0
		row.add_child(label)
		var slider := HSlider.new()
		slider.custom_minimum_size.x = 180.0
		slider.min_value = 0.0
		slider.max_value = definition[2]
		slider.step = 0.01
		slider.value = configuration.get(definition[0])
		slider.value_changed.connect(update_preference.bind(definition[0]))
		row.add_child(slider)
		sliders[definition[0]] = slider
	var actions := HBoxContainer.new()
	contents.add_child(actions)
	var save_button := Button.new()
	save_button.text = "Guardar preferencias"
	save_button.pressed.connect(save_preferences)
	actions.add_child(save_button)
	var reset_button := Button.new()
	reset_button.text = "Restablecer"
	reset_button.pressed.connect(reset_preferences)
	actions.add_child(reset_button)
	status_label = Label.new()
	status_label.text = "Los ajustes se aplican mientras conduces."
	contents.add_child(status_label)
	panel.reset_size()
	get_viewport().size_changed.connect(position_panel)
	position_panel()

func position_panel() -> void:
	panel.position = (get_viewport().get_visible_rect().size - panel.size) * 0.5

func _unhandled_input(input_event: InputEvent) -> void:
	if not is_instance_valid(camera_rig):
		return
	var was_visible := visible
	camera_rig.call("_unhandled_input", input_event)
	if was_visible != visible:
		get_viewport().set_input_as_handled()

func update_preference(value: float, property_name: String) -> void:
	configuration.apply_motion_preferences({property_name: value})
	status_label.text = "Ajuste aplicado. Guarda para conservarlo."

func save_preferences() -> void:
	var preferences_file := FileAccess.open(PREFERENCES_PATH, FileAccess.WRITE)
	if preferences_file == null:
		status_label.text = "No se pudieron guardar las preferencias."
		return
	preferences_file.store_string(JSON.stringify(configuration.get_motion_preferences(), "  "))
	status_label.text = "Preferencias guardadas."

func reset_preferences() -> void:
	configuration.apply_motion_preferences(default_preferences)
	for property_name in sliders:
		(sliders[property_name] as HSlider).set_value_no_signal(configuration.get(property_name))
	status_label.text = "Valores iniciales restaurados. Guarda para conservarlos."
