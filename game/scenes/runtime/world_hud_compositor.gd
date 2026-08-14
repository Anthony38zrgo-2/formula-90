extends Control

const SESSION_SCENE := preload("res://scenes/runtime/race_session.tscn")

# Kept for bootstrap ABI compatibility; composition now uses definitions.
@export_file("*.tscn") var world_scene_path := ""
@export var session_config: RaceSessionConfig

@onready var world_viewport: SubViewport = $WorldViewport
@onready var world_presenter: TextureRect = $WorldPresenter
@onready var debug_hud: ArcadeRaceHud = $HudLayer/DebugHud


func _ready() -> void:
	world_presenter.texture = world_viewport.get_texture()
	var session := SESSION_SCENE.instantiate() as RaceSession
	session.name = "RaceSession"
	session.config = session_config
	session.composition_ready.connect(_on_composition_ready)
	world_viewport.add_child(session)

func _on_composition_ready(vehicle: Vehicle, _track: Node3D, aids: DrivingAidsController) -> void:
	debug_hud.bind_runtime(vehicle, aids)
	var minimap := debug_hud.get_node_or_null("Minimap") as TrackMinimapController
	if minimap != null:
		minimap.map_data = session_config.selected_track.map_data
		minimap.set_target(vehicle)
	var tuner := debug_hud.get_node_or_null("HandlingTuningPanel")
	if tuner != null and tuner.has_method("bind_vehicle"):
		tuner.call("bind_vehicle", vehicle, vehicle.get_parent())
