class_name TireStatusPanel
extends PanelContainer

## Compact four-wheel pressure + thermal panel.
## Expected native/GDScript data contract (post-BRAKE-1000, snapshot schema 1):
## {
##   "FL": {"pressure_kpa":145.0,"tread_inner_c":92.0,"tread_center_c":96.0,
##          "tread_outer_c":88.0,"carcass_c":81.0,"gas_c":73.0},
##   ...
##   "schema_version": 1
## }
## Brake wheels (two-node compact contract, snapshot schema 1) keyed FL/FR/RL/RR:
## {"disc_c":..,"rim_c":..,"efficiency":..,"natural_cooling_w_k":..,
##  "speed_cooling_w_k":..,"duct_mass_flow_kg_s":..,"duct_drag_n":..,
##  "optimal_min_c":..,"optimal_max_c":..,"fade_start_c":..,"critical_c":..}
## Removed states (caliper_c, hub_c) are never read; the panel branches on
## field availability so transitional snapshots never render stale zeros.

const WHEELS := ["FL", "FR", "RL", "RR"]

## Visual scale of the whole panel (1.0 = full size). Reduced to 50% so the
## compact tyre readout does not crowd the tachometer/HUD.
@export var display_scale: float = 0.5

var _vehicle: Node
var _cells: Dictionary = {}

func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	custom_minimum_size = Vector2(390.0, 170.0)
	scale = Vector2(display_scale, display_scale)
	_build_ui()

func bind_vehicle(vehicle: Node) -> void:
	_vehicle = vehicle

func _process(_delta: float) -> void:
	if _vehicle == null:
		return

	var data: Dictionary = {}
	var brake_data: Dictionary = {}
	var snapshot: Dictionary = {}
	if _vehicle.has_method(&"get_tire_state_snapshot"):
		var native_data: Variant = _vehicle.call(&"get_tire_state_snapshot")
		if native_data is Dictionary:
			data = native_data
	elif _vehicle.has_method(&"get_telemetry_snapshot"):
		var snapshot_value: Variant = _vehicle.call(&"get_telemetry_snapshot")
		if snapshot_value is Dictionary:
			snapshot = snapshot_value
			var tire_data: Variant = snapshot.get("tires", {})
			if tire_data is Dictionary:
				data = tire_data

	if _vehicle.has_method(&"get_brake_state_snapshot"):
		var native_brake_data: Variant = _vehicle.call(&"get_brake_state_snapshot")
		if native_brake_data is Dictionary:
			brake_data = native_brake_data
	elif not snapshot.is_empty():
		var snapshot_brake_data: Variant = snapshot.get("brakes", {})
		if snapshot_brake_data is Dictionary:
			brake_data = snapshot_brake_data

	if not data.is_empty() or not brake_data.is_empty():
		set_tire_data(data, brake_data)

func set_tire_data(data: Dictionary, brake_data: Dictionary = {}) -> void:
	for wheel: String in WHEELS:
		if not _cells.has(wheel) or not data.has(wheel):
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
		var brake_label: Label = cell["brake"]

		pressure_label.text = "P %.0f kPa" % pressure
		zones_label.text = "I %.0f°  C %.0f°  O %.0f°" % [inner, center, outer]
		carcass_label.text = "CAR %.0f°  GAS %.0f°" % [carcass, gas]

		var brake_wheel: Dictionary = brake_data.get(wheel, {})
		if not brake_wheel.is_empty():
			var disc_text := "D---"
			var disc_value: Variant = brake_wheel.get("disc_c")
			if disc_value != null:
				disc_text = "D%.0f°" % float(disc_value)
			var rim_text := "RIM---"
			var rim_value: Variant = brake_wheel.get("rim_c")
			if rim_value != null:
				rim_text = "RIM%.0f°" % float(rim_value)
			brake_label.text = "BRK %s %s" % [disc_text, rim_text]
			brake_label.add_theme_color_override("font_color", _brake_temperature_color(
				float(disc_value if disc_value != null else 0.0),
				float(brake_wheel.get("optimal_min_c", 400.0)),
				float(brake_wheel.get("optimal_max_c", 800.0)),
				float(brake_wheel.get("fade_start_c", 900.0)),
				float(brake_wheel.get("critical_c", 1100.0))))

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

		var brake_label := Label.new()
		brake_label.text = "BRK D---° RIM---°"
		brake_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		box.add_child(brake_label)

		grid.add_child(box)
		_cells[wheel] = {
			"pressure": pressure_label,
			"zones": zones_label,
			"carcass": carcass_label,
			"brake": brake_label,
		}

func _temperature_color(temp_c: float) -> Color:
	if temp_c < 65.0:
		return Color(0.45, 0.72, 1.0)
	if temp_c <= 107.0:
		return Color(0.65, 1.0, 0.62)
	if temp_c <= 120.0:
		return Color(1.0, 0.82, 0.42)
	return Color(1.0, 0.42, 0.38)

func _brake_temperature_color(temp_c: float, optimal_min_c: float, optimal_max_c: float, fade_start_c: float, critical_c: float) -> Color:
	if temp_c < optimal_min_c:
		return Color(0.45, 0.72, 1.0)
	if temp_c <= optimal_max_c:
		return Color(0.65, 1.0, 0.62)
	if temp_c < fade_start_c:
		return Color(1.0, 0.82, 0.42)
	if temp_c < critical_c:
		return Color(1.0, 0.42, 0.38)
	return Color(0.85, 0.12, 0.12)
