extends Control

const SESSION_SCENE := preload("res://scenes/runtime/race_session.tscn")
const UPSCALE_MODE_NATIVE_HIRES_PSX := 3

## Temporary visual toggle. Keep the PSX controller and preset path intact so
## the presentation can be re-enabled without changing the runtime topology.
@export var psx_enabled := false

# Kept for bootstrap ABI compatibility; composition now uses definitions.
@export_file("*.tscn") var world_scene_path := ""
@export var session_config: RaceSessionConfig

@onready var world_viewport: SubViewport = $WorldViewport
@onready var display_stage: Control = $DisplayAspect/DisplayStage
@onready var world_presenter: TextureRect = $DisplayAspect/DisplayStage/WorldPresenter
@onready var hud_layer: Control = $DisplayAspect/DisplayStage/HudLayer
@onready var debug_hud: ArcadeRaceHud = $DisplayAspect/DisplayStage/HudLayer/DebugHud
@onready var psx_art: PsxArtController = get_node_or_null("PsxArtController") as PsxArtController


func _ready() -> void:
	display_stage.resized.connect(_sync_native_viewport_size)
	if psx_art != null:
		psx_art.preset_applied.connect(_on_visual_preset_applied)
		if psx_enabled and not psx_art.is_preset_loaded():
			psx_art.load_preset(psx_art.get_preset_path())
	if not psx_enabled:
		world_presenter.material = null
		world_presenter.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
	world_presenter.texture = world_viewport.get_texture()
	_sync_native_viewport_size()
	call_deferred("_sync_native_viewport_size")
	var session := SESSION_SCENE.instantiate() as RaceSession
	session.name = "RaceSession"
	session.config = session_config
	session.composition_ready.connect(_on_composition_ready)
	world_viewport.add_child(session)

func set_visual_preset(preset_path: String) -> bool:
	if psx_art != null:
		return psx_art.load_preset(preset_path)
	return false


func _on_visual_preset_applied(_profile_name: String) -> void:
	_sync_native_viewport_size()


func _sync_native_viewport_size() -> void:
	if display_stage == null or world_viewport == null or hud_layer == null or debug_hud == null:
		return
	var display_size := display_stage.size
	if display_size.x <= 0.0 or display_size.y <= 0.0:
		return
	hud_layer.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	debug_hud.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	if not psx_enabled or psx_art == null or not psx_art.is_preset_loaded():
		world_viewport.size = Vector2i(maxi(1, roundi(display_size.x)), maxi(1, roundi(display_size.y)))
	elif psx_art.get_upscale_mode() == UPSCALE_MODE_NATIVE_HIRES_PSX:
		world_viewport.size = Vector2i(maxi(1, roundi(display_size.x)), maxi(1, roundi(display_size.y)))

func _on_composition_ready(vehicle: Node, _track: Node3D, aids: DrivingAidsController) -> void:
	debug_hud.bind_runtime(vehicle, aids)
	var minimap := debug_hud.get_node_or_null("Minimap") as TrackMinimapController
	if minimap != null:
		minimap.map_data = session_config.selected_track.map_data
		minimap.set_target(vehicle)
	var tuner := debug_hud.get_node_or_null("HandlingTuningPanel")
	if tuner != null and tuner.has_method("bind_vehicle"):
		tuner.call("bind_vehicle", vehicle, vehicle.get_parent())
