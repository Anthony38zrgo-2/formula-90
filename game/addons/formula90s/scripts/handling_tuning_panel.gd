extends Control

## Development-only live tuner for the canonical Jordan handling scene.
## F10 toggles the panel. Values are runtime-only and never mutate the baseline scene.

@export var vehicle_path := NodePath("../Jordan191/VehicleRigidBody")
@export var controller_path := NodePath("../Jordan191")

const TOGGLE_KEY := KEY_F10
const CONTROL_DEFINITIONS := [
	{"id": "front_brake_bias", "label": "Reparto delantero", "min": 0.45, "max": 0.70, "step": 0.005},
	{"id": "max_steering_angle", "label": "Angulo maximo (grados)", "min": 8.0, "max": 40.0, "step": 0.5, "degrees": true},
	{"id": "max_torque", "label": "Torque maximo (Nm)", "min": 200.0, "max": 700.0, "step": 5.0},
	{"id": "motor_drag", "label": "Freno motor (drag/RPM)", "min": 0.0, "max": 0.03, "step": 0.0001},
	{"id": "coefficient_of_drag", "label": "Aero: coeficiente drag", "min": 0.20, "max": 1.50, "step": 0.01},
	{"id": "frontal_area", "label": "Aero: area frontal (m2)", "min": 0.50, "max": 2.50, "step": 0.05},
	{"id": "air_density", "label": "Aero: densidad aire", "min": 0.80, "max": 1.40, "step": 0.005},
]

var _vehicle: Vehicle
var _controller: Node
var _panel: PanelContainer
var _status_label: Label
var _controls: Dictionary = {}
var _baseline: Dictionary = {}
var _syncing := false
var _driving_suspended := false
var _controller_was_processing := true


func _ready() -> void:
	_build_ui()
	_panel.visible = false
	var exported_vehicle := get_node_or_null(vehicle_path) as Vehicle
	var exported_controller := get_node_or_null(controller_path)
	if exported_vehicle != null:
		bind_vehicle(exported_vehicle, exported_controller)
	else:
		var discovered_vehicle := _find_vehicle_in_scene()
		if discovered_vehicle != null:
			bind_vehicle(discovered_vehicle, discovered_vehicle.get_parent())
		else:
			_status_label.text = "Esperando vehiculo..."


func _exit_tree() -> void:
	_set_driving_enabled(true)


func _input(event: InputEvent) -> void:
	if (
		event is InputEventKey
		and event.pressed
		and not event.echo
		and (event.keycode == TOGGLE_KEY or event.physical_keycode == TOGGLE_KEY)
	):
		set_panel_visible(not _panel.visible)
		get_viewport().set_input_as_handled()


func set_panel_visible(show_panel: bool) -> void:
	if _panel == null:
		return
	_panel.visible = show_panel
	_set_driving_enabled(not show_panel)
	if show_panel and _vehicle != null:
		_sync_controls_from_vehicle()
		_status_label.text = "AJUSTE LIVE | conduccion pausada"
	elif show_panel:
		_status_label.text = "ERROR: vehiculo no enlazado"
	else:
		_status_label.text = "F10 abre el panel"


func bind_vehicle(vehicle: Vehicle, controller: Node = null) -> void:
	_vehicle = vehicle
	_controller = controller if controller != null else vehicle.get_parent()
	_capture_baseline()
	_sync_controls_from_vehicle()
	_status_label.text = "F10 abre el panel"


func get_tunable_ids() -> Array[String]:
	var ids: Array[String] = []
	for definition in CONTROL_DEFINITIONS:
		ids.append(definition.id)
	return ids


func _build_ui() -> void:
	_panel = PanelContainer.new()
	_panel.name = "LiveTuningPanel"
	_panel.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	_panel.position = Vector2(-430.0, 18.0)
	_panel.size = Vector2(410.0, 0.0)
	_panel.mouse_filter = Control.MOUSE_FILTER_STOP
	_panel.z_index = 100
	add_child(_panel)

	var margin := MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 16)
	margin.add_theme_constant_override("margin_top", 12)
	margin.add_theme_constant_override("margin_right", 16)
	margin.add_theme_constant_override("margin_bottom", 12)
	_panel.add_child(margin)

	var content := VBoxContainer.new()
	content.add_theme_constant_override("separation", 7)
	margin.add_child(content)

	var title := Label.new()
	title.text = "JORDAN 191 | HANDLING LIVE"
	title.add_theme_font_size_override("font_size", 20)
	content.add_child(title)

	var hint := Label.new()
	hint.text = "F10 cerrar | cambios temporales"
	hint.modulate = Color(0.75, 0.85, 1.0)
	content.add_child(hint)

	for definition in CONTROL_DEFINITIONS:
		var row := HBoxContainer.new()
		var label := Label.new()
		label.text = definition.label
		label.custom_minimum_size.x = 225.0
		row.add_child(label)

		var spin_box := SpinBox.new()
		spin_box.name = definition.id
		spin_box.custom_minimum_size.x = 145.0
		spin_box.min_value = definition.min
		spin_box.max_value = definition.max
		spin_box.step = definition.step
		spin_box.allow_greater = false
		spin_box.allow_lesser = false
		spin_box.value_changed.connect(_on_value_changed.bind(definition.id))
		row.add_child(spin_box)
		_controls[definition.id] = spin_box
		content.add_child(row)

	var aero_note := Label.new()
	aero_note.text = "Aero actual: solo drag. Downforce/balance aun no existen."
	aero_note.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	aero_note.modulate = Color(1.0, 0.78, 0.35)
	content.add_child(aero_note)

	var actions := HBoxContainer.new()
	var reset_button := Button.new()
	reset_button.text = "Restaurar valores iniciales"
	reset_button.pressed.connect(_restore_baseline)
	actions.add_child(reset_button)
	content.add_child(actions)

	_status_label = Label.new()
	_status_label.text = "F10 abre el panel"
	_status_label.modulate = Color(0.65, 1.0, 0.72)
	content.add_child(_status_label)


func _capture_baseline() -> void:
	_baseline.clear()
	for definition in CONTROL_DEFINITIONS:
		_baseline[definition.id] = _read_display_value(definition)


func _sync_controls_from_vehicle() -> void:
	if _vehicle == null:
		return
	_syncing = true
	for definition in CONTROL_DEFINITIONS:
		var spin_box: SpinBox = _controls[definition.id]
		spin_box.value = _read_display_value(definition)
	_syncing = false


func _read_display_value(definition: Dictionary) -> float:
	var value: float = _vehicle.get(definition.id)
	if definition.get("degrees", false):
		return rad_to_deg(value)
	return value


func _on_value_changed(value: float, property_name: String) -> void:
	if _syncing or _vehicle == null:
		return
	_apply_value(property_name, value)
	_status_label.text = "%s = %s" % [property_name, value]


func _apply_value(property_name: String, display_value: float) -> void:
	var value := display_value
	if property_name == "max_steering_angle":
		value = deg_to_rad(display_value)
	_vehicle.set(property_name, value)

	# GEVP caches these derived values during Vehicle._ready(), so keep the
	# runtime copies synchronized when their source property changes.
	if property_name == "front_brake_bias" and _vehicle.front_axle != null:
		_vehicle.front_axle.brake_bias = value
		_vehicle.rear_axle.brake_bias = 1.0 - value
	elif property_name == "max_torque":
		_vehicle.max_clutch_torque = value * _vehicle.max_clutch_torque_ratio


func _restore_baseline() -> void:
	_syncing = true
	for definition in CONTROL_DEFINITIONS:
		var value: float = _baseline[definition.id]
		var spin_box: SpinBox = _controls[definition.id]
		spin_box.value = value
		_apply_value(definition.id, value)
	_syncing = false
	_status_label.text = "Valores iniciales restaurados"


func _set_driving_enabled(enabled: bool) -> void:
	if not enabled and not _driving_suspended:
		if _controller != null:
			_controller_was_processing = _controller.is_physics_processing()
			_controller.set_physics_process(false)
		_driving_suspended = true
	elif enabled and _driving_suspended:
		if _controller != null:
			_controller.set_physics_process(_controller_was_processing)
		_driving_suspended = false
	if not enabled and _vehicle != null:
		_vehicle.throttle_input = 0.0
		_vehicle.brake_input = 0.0
		_vehicle.steering_input = 0.0
		_vehicle.handbrake_input = 0.0
		_vehicle.clutch_input = 0.0


func _find_vehicle_in_scene() -> Vehicle:
	var search_root: Node = self
	while search_root.get_parent() != null and search_root.get_parent() != get_tree().root:
		search_root = search_root.get_parent()
	return _find_vehicle_recursive(search_root)


func _find_vehicle_recursive(node: Node) -> Vehicle:
	if node is Vehicle:
		return node as Vehicle
	for child in node.get_children():
		var found := _find_vehicle_recursive(child)
		if found != null:
			return found
	return null
