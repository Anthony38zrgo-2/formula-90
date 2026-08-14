class_name ArcadeRaceHud
extends Control

const AID_NOTIFICATION_SECONDS := 2.4
const AID_FADE_SECONDS := 0.35

@export var vehicle_path: NodePath
@export var aids_path: NodePath

@onready var speed_gauge: ArcadeSpeedGauge = $SpeedGauge
@onready var retro_hud: Node = $RetroHud
@onready var aid_message: Label = $AidMessage

var _vehicle: Node
var _aids: Node
var _notification_remaining := 0.0

func bind_runtime(vehicle: Node, aids: Node) -> void:
	_vehicle = vehicle
	_aids = aids
	_connect_aid_notifications()


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	aid_message.visible = false
	_resolve_runtime_nodes()


func _process(delta: float) -> void:
	_resolve_runtime_nodes()
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


func _update_speed_gauge() -> void:
	if _vehicle == null or speed_gauge == null:
		return

	var speed_mps := float(_vehicle.get("speed"))
	var gear := int(_vehicle.get("current_gear"))
	var motor_rpm_value: Variant = _vehicle.get("motor_rpm")
	var motor_rpm := float(motor_rpm_value) if motor_rpm_value != null else 0.0
	var displayed_gear := "R" if gear < 0 else ("N" if gear == 0 else str(gear))
	speed_gauge.set_readout(absf(speed_mps) * 3.6, displayed_gear)
	if retro_hud != null and retro_hud.has_method("set_readout"):
		retro_hud.call("set_readout", absf(speed_mps) * 3.6, motor_rpm, displayed_gear)


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


# FUTURE_UI-002: Put LAP, POS, countdown, and rival markers here only after a
# race-session authority exposes verified values. Do not render placeholder data.
