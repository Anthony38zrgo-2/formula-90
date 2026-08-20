class_name TireStatusPanel
extends PanelContainer

## Compact four-wheel pressure + thermal panel.
## Expected native/GDScript data contract:
## {
##   "FL": {"pressure_kpa":145.0,"tread_inner_c":92.0,"tread_center_c":96.0,
##          "tread_outer_c":88.0,"carcass_c":81.0,"gas_c":73.0},
##   ...
## }

const WHEELS := ["FL", "FR", "RL", "RR"]

var _vehicle: Node
var _cells: Dictionary = {}

func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	custom_minimum_size = Vector2(390.0, 170.0)
	_build_ui()

func bind_vehicle(vehicle: Node) -> void:
	_vehicle = vehicle

func _process(_delta: float) -> void:
	if _vehicle == null:
		return

	var data: Dictionary = {}
	if _vehicle.has_method(&"get_tire_state_snapshot"):
		var native_data: Variant = _vehicle.call(&"get_tire_state_snapshot")
		if native_data is Dictionary:
			data = native_data
	elif _vehicle.has_method(&"get_telemetry_snapshot"):
		var snapshot: Variant = _vehicle.call(&"get_telemetry_snapshot")
		if snapshot is Dictionary:
			var tire_data: Variant = snapshot.get("tires", {})
			if tire_data is Dictionary:
				data = tire_data

	if not data.is_empty():
		set_tire_data(data)

func set_tire_data(data: Dictionary) -> void:
	for wheel: String in WHEELS:
		if not _cells.has(wheel):
			continue
		var wheel_data: Dictionary = data.get(wheel, {})
		if wheel_data.is_empty():
			continue

		var pressure := float(wheel_data.get("pressure_kpa", 0.0))
		var inner := float(wheel_data.get("tread_inner_c", 0.0))
		var center := float(wheel_data.get("tread_center_c", 0.0))
		var outer := float(wheel_data.get("tread_outer_c", 0.0))
		var carcass := float(wheel_data.get("carcass_c", 0.0))
		var gas := float(wheel_data.get("gas_c", 0.0))

		var cell: Dictionary = _cells[wheel]
		var pressure_label: Label = cell["pressure"]
		var zones_label: Label = cell["zones"]
		var carcass_label: Label = cell["carcass"]

		pressure_label.text = "P %.0f kPa" % pressure
		zones_label.text = "I %.0f°  C %.0f°  O %.0f°" % [inner, center, outer]
		carcass_label.text = "CAR %.0f°  GAS %.0f°" % [carcass, gas]

		# Overall zone colour is based on the hottest tread reading so overheating
		# remains visible without making the compact panel unreadable.
		var hottest := maxf(inner, maxf(center, outer))
		zones_label.add_theme_color_override("font_color", _temperature_color(hottest))
		carcass_label.add_theme_color_override("font_color", _temperature_color(carcass))

func _build_ui() -> void:
	var margin := MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 8)
	margin.add_theme_constant_override("margin_right", 8)
	margin.add_theme_constant_override("margin_top", 6)
	margin.add_theme_constant_override("margin_bottom", 6)
	add_child(margin)

	var root_box := VBoxContainer.new()
	margin.add_child(root_box)

	var title := Label.new()
	title.text = "TYRES"
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	root_box.add_child(title)

	var grid := GridContainer.new()
	grid.columns = 2
	grid.add_theme_constant_override("h_separation", 10)
	grid.add_theme_constant_override("v_separation", 6)
	root_box.add_child(grid)

	for wheel: String in WHEELS:
		var box := VBoxContainer.new()
		box.custom_minimum_size = Vector2(180.0, 60.0)

		var header := Label.new()
		header.text = wheel
		header.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		box.add_child(header)

		var pressure_label := Label.new()
		pressure_label.text = "P --- kPa"
		pressure_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		box.add_child(pressure_label)

		var zones_label := Label.new()
		zones_label.text = "I --°  C --°  O --°"
		zones_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		box.add_child(zones_label)

		var carcass_label := Label.new()
		carcass_label.text = "CAR --°  GAS --°"
		carcass_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		box.add_child(carcass_label)

		grid.add_child(box)
		_cells[wheel] = {
			"pressure": pressure_label,
			"zones": zones_label,
			"carcass": carcass_label,
		}

func _temperature_color(temp_c: float) -> Color:
	if temp_c < 65.0:
		return Color(0.45, 0.72, 1.0)
	if temp_c <= 107.0:
		return Color(0.65, 1.0, 0.62)
	if temp_c <= 120.0:
		return Color(1.0, 0.82, 0.42)
	return Color(1.0, 0.42, 0.38)
