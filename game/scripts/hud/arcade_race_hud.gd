class_name ArcadeRaceHud
extends Control

const AID_NOTIFICATION_SECONDS := 2.4
const AID_FADE_SECONDS := 0.35
const TireStatusPanelScript := preload("res://scripts/hud/tire_status_panel.gd")
const EngineTemperaturePanelScript := preload("res://scripts/hud/engine_temperature_panel.gd")
const LapTimingPanelScript := preload("res://scripts/hud/lap_timing_panel.gd")
const DEFAULT_GAP := 10.0

@export var vehicle_path: NodePath
@export var aids_path: NodePath
@export_file("*.json") var hud_config_path := HudConfig.DEFAULT_PATH

@onready var speed_gauge: ArcadeSpeedGauge = $SpeedGauge
@onready var retro_hud: Node = $RetroHud
@onready var aid_message: Label = $AidMessage

var _vehicle: Node
var _aids: Node
var _notification_remaining := 0.0
var _hud_config: HudConfig = HudConfig.load_from_json(hud_config_path)
var tire_status_panel: TireStatusPanel
var engine_temperature_panel: EngineTemperaturePanel
var lap_timing_panel: LapTimingPanel

func bind_runtime(vehicle: Node, aids: Node, lap_timing: LapTimingController = null) -> void:
	_vehicle = vehicle
	_aids = aids
	if tire_status_panel != null:
		tire_status_panel.bind_vehicle(_vehicle)
	if engine_temperature_panel != null:
		engine_temperature_panel.bind_vehicle(_vehicle)
		engine_temperature_panel.bind_lap_timing(lap_timing)
	if lap_timing_panel != null:
		lap_timing_panel.bind_lap_timing(lap_timing)
	_connect_aid_notifications()


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	aid_message.visible = false
	_resolve_runtime_nodes()
	_ensure_tire_status_panel()
	_ensure_engine_temperature_panel()
	_ensure_lap_timing_panel()
	_apply_hud_layout()


func _process(delta: float) -> void:
	_resolve_runtime_nodes()
	_position_vehicle_status_panels()
	_position_lap_timing_panel()
	_update_speed_gauge()
	_update_aid_notification(delta)


func _resolve_runtime_nodes() -> void:
	if _vehicle == null and not vehicle_path.is_empty():
		_vehicle = get_node_or_null(vehicle_path)

	if _aids == null and not aids_path.is_empty():
		_aids = get_node_or_null(aids_path)
		_connect_aid_notifications()


func _connect_aid_notifications() -> void:
	if _aids == null or not _aids.has_signal(&"aid_toggled"):
		return

	var callback := Callable(self, "_on_aid_toggled")
	if not _aids.is_connected(&"aid_toggled", callback):
		_aids.connect(&"aid_toggled", callback)


func _ensure_tire_status_panel() -> void:
	if tire_status_panel != null:
		return
	tire_status_panel = TireStatusPanelScript.new()
	tire_status_panel.name = "TireStatusPanel"
	tire_status_panel.apply_settings(_hud_config.tires)
	add_child(tire_status_panel)
	tire_status_panel.bind_vehicle(_vehicle)
	_position_vehicle_status_panels()


func _ensure_engine_temperature_panel() -> void:
	if engine_temperature_panel != null:
		return
	engine_temperature_panel = EngineTemperaturePanelScript.new()
	engine_temperature_panel.name = "EngineTemperaturePanel"
	engine_temperature_panel.apply_settings(_hud_config.engine_temperatures)
	add_child(engine_temperature_panel)
	engine_temperature_panel.bind_vehicle(_vehicle)
	_position_vehicle_status_panels()

func _ensure_lap_timing_panel() -> void:
	if lap_timing_panel != null:
		return
	lap_timing_panel = LapTimingPanelScript.new()
	lap_timing_panel.name = "LapTimingPanel"
	lap_timing_panel.apply_settings(_hud_config.lap_timing)
	add_child(lap_timing_panel)
	_position_lap_timing_panel()


func _apply_hud_layout() -> void:
	# RetroHud theme lives in retro_hud.json (referenced from hud_config.json); the
	# unified config only drives the JSON-safe scale/visibility. Position stays
	# anchored in the scene so the RetroHud layout is preserved.
	if retro_hud != null and retro_hud.has_method("apply_hud_layout"):
		retro_hud.call("apply_hud_layout", _hud_config.retro_hud.display_scale, _hud_config.retro_hud.visible)


func _position_vehicle_status_panels() -> void:
	if tire_status_panel == null or engine_temperature_panel == null:
		return

	# RetroHud is the primary reference because it owns the tachometer/readout region.
	var reference: Control = retro_hud as Control
	if reference == null:
		reference = speed_gauge as Control
	if reference == null:
		return

	var ref_rect := reference.get_global_rect()
	var tire_visual_size := _visual_panel_size(tire_status_panel)
	var engine_visual_size := _visual_panel_size(engine_temperature_panel)
	var panel_group_width := maxf(tire_visual_size.x, engine_visual_size.x)
	var group_center_horizontal_position := ref_rect.position.x + ref_rect.size.x * 0.5
	var viewport_size := get_viewport_rect().size
	group_center_horizontal_position = clampf(group_center_horizontal_position, panel_group_width * 0.5 + 6.0, viewport_size.x - panel_group_width * 0.5 - 6.0)
	var tire_vertical_position := ref_rect.position.y - tire_visual_size.y - _hud_config.tires.gap
	var engine_vertical_position := tire_vertical_position - engine_visual_size.y - _hud_config.engine_temperatures.gap
	var full_group_height := ref_rect.position.y + ref_rect.size.y - engine_vertical_position
	var group_top := clampf(engine_vertical_position, 6.0, maxf(6.0, viewport_size.y - full_group_height - 6.0))
	var vertical_adjustment := group_top - engine_vertical_position
	tire_vertical_position += vertical_adjustment
	engine_vertical_position += vertical_adjustment
	tire_status_panel.global_position = Vector2(
		group_center_horizontal_position - tire_visual_size.x * 0.5,
		tire_vertical_position)
	engine_temperature_panel.global_position = Vector2(
		group_center_horizontal_position - engine_visual_size.x * 0.5,
		engine_vertical_position)


func _position_lap_timing_panel() -> void:
	if lap_timing_panel == null:
		return
	lap_timing_panel.global_position = Vector2(
		_hud_config.lap_timing.margin_left,
		_hud_config.lap_timing.margin_top)


func _visual_panel_size(panel: Control) -> Vector2:
	var panel_size := panel.size
	if panel_size.x <= 0.0 or panel_size.y <= 0.0:
		panel_size = panel.custom_minimum_size
	return panel_size * panel.scale


func _update_speed_gauge() -> void:
	if _vehicle == null or speed_gauge == null:
		return

	var speed_mps := float(_vehicle.get("speed"))
	var gear := int(_vehicle.get("current_gear"))
	var motor_rpm_value: Variant = _vehicle.get("motor_rpm")
	var motor_rpm := float(motor_rpm_value) if motor_rpm_value != null else 0.0
	var displayed_gear := "R" if gear < 0 else ("N" if gear == 0 else str(gear))
	var throttle_value: Variant = _vehicle.get("throttle_input")
	var brake_value: Variant = _vehicle.get("brake_input")
	var throttle := clampf(float(throttle_value) if throttle_value != null else 0.0, 0.0, 1.0)
	var brake := clampf(float(brake_value) if brake_value != null else 0.0, 0.0, 1.0)
	speed_gauge.set_readout(absf(speed_mps) * 3.6, displayed_gear)
	if retro_hud != null and retro_hud.has_method("set_readout"):
		retro_hud.call("set_readout", absf(speed_mps) * 3.6, motor_rpm, displayed_gear, throttle, brake)


func _on_aid_toggled(aid_label: String, enabled: bool) -> void:
	var state := "ACTIVADA" if enabled else "DESACTIVADA"
	aid_message.text = "AYUDA %s %s" % [aid_label, state]
	aid_message.modulate = Color.WHITE
	aid_message.visible = true
	_notification_remaining = AID_NOTIFICATION_SECONDS


func _update_aid_notification(delta: float) -> void:
	if _notification_remaining <= 0.0:
		return

	_notification_remaining = maxf(_notification_remaining - delta, 0.0)
	if _notification_remaining <= 0.0:
		aid_message.visible = false
		return

	if _notification_remaining < AID_FADE_SECONDS:
		aid_message.modulate.a = _notification_remaining / AID_FADE_SECONDS


# FUTURE_UI-002: LAP is delivered by LapTimingPanel from the RaceSession lap
# timing authority. POS, countdown, and rival markers stay absent until a
# race-session authority exposes verified values. Do not render placeholder data.
