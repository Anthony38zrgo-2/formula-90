class_name BackgroundMountains3D
extends Node3D

## Fondo 3D procedural: anillos de montanas + sky dome + cascadas.
## Generado offline por generate_mountains_3d.py, importado como GLB.
## Reemplaza al legacy SourceSkyboxRig + BackgroundSkybox + BackgroundController.

const WATERFALL_SHEET := "res://assets/skybox/la_chutana/la_chutana_waterfall_sheet.png"
const WATERFALL_GROUP := &"mountains_3d_waterfalls"

var _camera: Camera3D
var _sky_dome: MeshInstance3D
var _far_ring: MeshInstance3D
var _near_ring: MeshInstance3D
var _waterfalls: Array[Sprite3D] = []

var _waterfall_elapsed := 0.0
var _waterfall_frame := -1
var _waterfall_fps := 8.0
var _manifest: Dictionary = {}
var _is_active := false


func setup(manifest_path: String) -> bool:
	var file := FileAccess.open(manifest_path, FileAccess.READ)
	if file == null:
		push_error("BackgroundMountains3D: No se pudo abrir manifest '%s'." % manifest_path)
		return false

	var json_text := file.get_as_text()
	var json := JSON.new()
	var err := json.parse(json_text)
	if err != OK:
		push_error("BackgroundMountains3D: Error parseando manifest: %s" % json.get_error_message())
		return false

	_manifest = json.data
	var base_dir: String = manifest_path.get_base_dir()

	# Load sky dome (background sky only)
	var sky_path: String = base_dir + "/" + String(_manifest.geometry_assets.sky_dome)
	_sky_dome = _load_mesh(sky_path, "SkyDome", true)
	if _sky_dome == null:
		return false

	# Load far mountains ring (real 3D geometry at R=1600m)
	var far_path: String = base_dir + "/" + String(_manifest.geometry_assets.far_mountains)
	_far_ring = _load_mesh(far_path, "FarMountains", false)
	if _far_ring == null:
		return false

	# Load near mountains ring (real 3D geometry at R=1150m)
	var near_path: String = base_dir + "/" + String(_manifest.geometry_assets.near_mountains)
	_near_ring = _load_mesh(near_path, "NearMountains", false)
	if _near_ring == null:
		return false

	# Create waterfalls
	_create_waterfalls()

	_is_active = true
	return true


func _load_mesh(glb_path: String, node_name: String, is_sky: bool = false) -> MeshInstance3D:
	var loaded = load(glb_path)
	if loaded == null:
		push_error("BackgroundMountains3D: No se pudo cargar '%s'." % glb_path)
		return null

	var mi := MeshInstance3D.new()
	mi.name = node_name

	# GLB files import as PackedScene; extract mesh from first MeshInstance3D child
	if loaded is PackedScene:
		var scene := loaded as PackedScene
		var instance := scene.instantiate()
		var source_mi := _find_first_mesh_instance(instance)
		if source_mi != null:
			mi.mesh = source_mi.mesh
		else:
			push_error("BackgroundMountains3D: No MeshInstance3D found in '%s'." % glb_path)
			instance.queue_free()
			return null
		instance.queue_free()
	elif loaded is Mesh:
		mi.mesh = loaded

	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.vertex_color_use_as_albedo = true
	mat.texture_filter = BaseMaterial3D.TEXTURE_FILTER_NEAREST
	mat.cull_mode = BaseMaterial3D.CULL_DISABLED

	if is_sky:
		# Sky dome is background only: does not write depth and draws first
		mat.depth_draw_mode = BaseMaterial3D.DEPTH_DRAW_DISABLED
		mat.render_priority = -10
	else:
		# Mountains are real 3D geometry at distance (1150m / 1600m):
		# must write depth so foreground track, trees, fences and vehicles
		# at Z < 600m correctly occlude the background.
		mat.depth_draw_mode = BaseMaterial3D.DEPTH_DRAW_OPAQUE_ONLY
		mat.render_priority = 0

	mi.material_override = mat

	add_child(mi)
	return mi


func _find_first_mesh_instance(node: Node) -> MeshInstance3D:
	if node is MeshInstance3D:
		return node as MeshInstance3D
	for child in node.get_children():
		var result := _find_first_mesh_instance(child)
		if result != null:
			return result
	return null


func _create_waterfalls() -> void:
	if not _manifest.has("waterfalls"):
		return

	var wf_sheet := load(WATERFALL_SHEET) as Texture2D
	if wf_sheet == null:
		push_warning("BackgroundMountains3D: No se pudo cargar waterfall sheet.")
		return

	var wf_data: Array = _manifest.waterfalls
	for i in range(wf_data.size()):
		var wf: Dictionary = wf_data[i]
		var sprite := Sprite3D.new()
		sprite.name = "Waterfall_%d" % i
		sprite.texture = wf_sheet
		sprite.texture_filter = BaseMaterial3D.TEXTURE_FILTER_NEAREST
		sprite.shaded = false
		sprite.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		sprite.gi_mode = GeometryInstance3D.GI_MODE_DISABLED
		sprite.hframes = 4
		sprite.vframes = 1
		sprite.pixel_size = 0.45
		sprite.render_priority = 2
		sprite.alpha_cut = Sprite3D.ALPHA_CUT_DISCARD

		# Position from manifest
		var pos_arr: Array = wf.position
		sprite.position = Vector3(pos_arr[0], pos_arr[1], pos_arr[2])

		# Scale Y from manifest (taller or shorter cascade)
		var sy: float = wf.get("scale_y", 1.15)
		sprite.scale = Vector3(1.0, sy, 1.0)

		# Face outward from ring center (perpendicular to the radial direction).
		# angle = atan2(z, x); the sprite's front must point radially outward,
		# so rotate +90deg from the tangent direction.
		var angle: float = wf.get("angle_rad", 0.0)
		sprite.rotation.y = -(angle + PI / 2.0)

		# Slightly in front of mountain surface to avoid z-fighting
		sprite.position += Vector3(cos(angle), 0, sin(angle)) * 0.5

		add_child(sprite)
		_waterfalls.append(sprite)


func _process(delta: float) -> void:
	if not _is_active:
		return

	_follow_camera()
	_animate_waterfalls(delta)


func _follow_camera() -> void:
	if not is_instance_valid(_camera) or not is_inside_tree() or not _camera.is_inside_tree():
		return
	# SkyDome follows camera XZ; rings stay in world-space for real parallax.
	if is_instance_valid(_sky_dome) and _sky_dome.is_inside_tree():
		_sky_dome.global_position.x = _camera.global_position.x
		_sky_dome.global_position.z = _camera.global_position.z


func _animate_waterfalls(delta: float) -> void:
	_waterfall_elapsed += delta
	var next_frame := int(floor(_waterfall_elapsed * _waterfall_fps)) % 4
	if next_frame == _waterfall_frame:
		return
	_waterfall_frame = next_frame

	for i in range(_waterfalls.size()):
		var wf := _waterfalls[i]
		if is_instance_valid(wf):
			wf.frame = (next_frame + i) % maxi(1, wf.hframes)


func set_camera_source(cam: Camera3D) -> void:
	_camera = cam
	_follow_camera()


func is_active() -> bool:
	return _is_active


func get_debug_info() -> Dictionary:
	return {
		"is_active": _is_active,
		"camera_valid": is_instance_valid(_camera),
		"sky_dome": _sky_dome != null,
		"far_ring": _far_ring != null,
		"near_ring": _near_ring != null,
		"waterfalls": _waterfalls.size(),
		"manifest": _manifest.get("asset", "none"),
	}
