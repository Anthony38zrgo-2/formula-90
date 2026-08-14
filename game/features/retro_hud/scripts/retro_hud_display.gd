class_name RetroHudDisplay
extends Control

const STATE_SCRIPT := preload("res://features/retro_hud/scripts/retro_hud_state.gd")
const CONFIG_SCRIPT := preload("res://features/retro_hud/scripts/retro_hud_config.gd")

@export_file("*.json") var config_path := "res://features/retro_hud/config/retro_hud.json"
@export var apply_layout_from_config := false

@onready var base: TextureRect = $Base
@onready var tachometer: Control = $Tachometer
@onready var rpm_caption: Control = $RpmCaption
@onready var rpm_scale: Control = $RpmScale
@onready var speed_value: Control = $SpeedValue
@onready var speed_unit: Control = $SpeedUnit
@onready var gear_caption: Control = $GearCaption
@onready var gear_value: Control = $GearValue

var state = STATE_SCRIPT.new()
var config = CONFIG_SCRIPT.new()


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	state.changed.connect(_refresh)
	reload_config()
	_refresh()


func set_readout(speed_kph: float, rpm: float, gear_label: String) -> void:
	state.set_readout(speed_kph, rpm, gear_label)


func set_state(next_state: RefCounted) -> void:
	if next_state == null or state == next_state:
		return
	if state.changed.is_connected(_refresh):
		state.changed.disconnect(_refresh)
	state = next_state
	state.changed.connect(_refresh)
	_refresh()


func reload_config() -> void:
	config = CONFIG_SCRIPT.load_from_json(config_path)
	base.texture = load(config.base_texture_path) as Texture2D
	base.modulate.a = config.background_opacity
	if apply_layout_from_config:
		position = config.display_position
		scale = Vector2.ONE * config.display_scale
	tachometer.configure(config)
	for glyph in [rpm_caption, rpm_scale, speed_value, speed_unit, gear_caption]:
		glyph.glyph_color = config.glyph_color
		glyph.queue_redraw()
	gear_value.glyph_color = config.gear_color
	gear_value.queue_redraw()
	_refresh()


func _refresh() -> void:
	if not is_node_ready():
		return
	tachometer.set_rpm(state.rpm)
	rpm_caption.set_text("RPM")
	rpm_scale.set_text("X1000")
	speed_value.set_text(str(int(roundi(state.speed_kph))))
	speed_unit.set_text(config.speed_unit)
	gear_caption.set_text("GEAR")
	gear_value.set_text(state.gear_label)
