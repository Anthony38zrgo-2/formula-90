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
		# The world viewport is the final presentation surface. Linear filtering
		# blends its texels again when the window is scaled, making every texture
		# look soft. Keep the pixel-authentic nearest filter used by the project.
		world_presenter.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
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
	_apply_high_quality_texture_filter(vehicle)
	_apply_high_quality_texture_filter(_track)
	_apply_vehicle_shadow_caster(vehicle)
	_apply_non_vehicle_full_bright(_track)
	# Background/controllers live beside the vehicle and track in RaceSession.
	# Apply the same no-shadow/full-bright policy to those siblings while keeping
	# the vehicle subtree untouched so its lighting and shadow remain available.
	var session_root := _track.get_parent().get_parent() if _track != null and _track.get_parent() != null and _track.get_parent().get_parent() != null else null
	if session_root != null:
		var vehicle_root := vehicle
		while vehicle_root != null and vehicle_root.get_parent() != session_root:
			vehicle_root = vehicle_root.get_parent()
		for child in session_root.get_children():
			if child != _track.get_parent() and child != vehicle_root:
				_apply_non_vehicle_full_bright(child)
	debug_hud.bind_runtime(vehicle, aids)
	var minimap := debug_hud.get_node_or_null("Minimap") as TrackMinimapController
	if minimap != null:
		minimap.map_data = session_config.selected_track.map_data
		minimap.set_target(vehicle)
	var tuner := debug_hud.get_node_or_null("HandlingTuningPanel")
	if tuner != null and tuner.has_method("bind_vehicle"):
		tuner.call("bind_vehicle", vehicle, vehicle.get_parent())


func _apply_high_quality_texture_filter(root: Node) -> void:
	# Normalize imported vehicle/track materials at runtime. Keep nearest only
	# for the final 2D presenter; 3D materials need mipmaps and anisotropy to
	# avoid shimmer/stipple on distant or oblique surfaces. ShaderMaterial is
	# intentionally excluded because its sampler hints are part of the shader.
	if root == null:
		return
	if root is Sprite3D:
		(root as Sprite3D).texture_filter = BaseMaterial3D.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS_ANISOTROPIC
	elif root is MeshInstance3D and (root as MeshInstance3D).mesh != null:
		var mesh_instance := root as MeshInstance3D
		for surface in range(mesh_instance.mesh.get_surface_count()):
			var source := mesh_instance.get_active_material(surface)
			if source is StandardMaterial3D:
				(source as StandardMaterial3D).texture_filter = BaseMaterial3D.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS_ANISOTROPIC
	for child in root.get_children():
		_apply_high_quality_texture_filter(child)


func _apply_vehicle_shadow_caster(root: Node) -> void:
	if root == null:
		return
	if root is CPUParticles3D or root is GPUParticles3D:
		# World-space tire smoke is an effect, not solid vehicle geometry.
		(root as GeometryInstance3D).cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	elif root is GeometryInstance3D:
		(root as GeometryInstance3D).cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	for child in root.get_children():
		_apply_vehicle_shadow_caster(child)


func _apply_non_vehicle_full_bright(root: Node) -> void:
	if root == null:
		return
	if root is GeometryInstance3D:
		var geometry := root as GeometryInstance3D
		geometry.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		geometry.gi_mode = GeometryInstance3D.GI_MODE_DISABLED
	if root is SpriteBase3D:
		(root as SpriteBase3D).shaded = false
	if root is MeshInstance3D and (root as MeshInstance3D).mesh != null:
		var mesh_instance := root as MeshInstance3D
		var receives_vehicle_shadow := _is_shadow_receiver(mesh_instance)
		for surface in range(mesh_instance.mesh.get_surface_count()):
			var material := mesh_instance.get_active_material(surface)
			if material is BaseMaterial3D:
				# Unshaded materials cannot receive dynamic shadows. Keep the Fuji
				# road meshes lit so the vehicle shadow is visible on the asphalt;
				# every other environment material remains max full bright.
				(material as BaseMaterial3D).shading_mode = BaseMaterial3D.SHADING_MODE_PER_PIXEL if receives_vehicle_shadow else BaseMaterial3D.SHADING_MODE_UNSHADED
	for child in root.get_children():
		_apply_non_vehicle_full_bright(child)


func _is_shadow_receiver(node: Node) -> bool:
	var lower := node.name.to_lower()
	return lower.begins_with("trak") or "road" in lower or "asphalt" in lower or "ground" in lower
