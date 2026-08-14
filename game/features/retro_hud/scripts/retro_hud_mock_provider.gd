class_name RetroHudMockProvider
extends Node

const STATE_SCRIPT := preload("res://features/retro_hud/scripts/retro_hud_state.gd")

@export var display_path: NodePath
@export var cycle_seconds := 8.0

var state = STATE_SCRIPT.new()
var _elapsed := 0.0


func _ready() -> void:
	var display := get_node_or_null(display_path)
	if display != null:
		display.set_state(state)


func _process(delta: float) -> void:
	_elapsed = fmod(_elapsed + delta, maxf(cycle_seconds, 0.1))
	var phase := _elapsed / maxf(cycle_seconds, 0.1)
	var rpm := lerpf(1800.0, 14500.0, 0.5 + 0.5 * sin(phase * TAU))
	var speed := lerpf(0.0, 318.0, clampf((rpm - 1500.0) / 13000.0, 0.0, 1.0))
	var gear := clampi(int(rpm / 2400.0), 1, 6)
	state.set_readout(speed, rpm, str(gear))
