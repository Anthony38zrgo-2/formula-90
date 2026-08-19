class_name BackgroundSkybox
extends Node3D

## Skybox desacoplado del sistema de capas parallax (Formula-90).
##
## Soporta dos modos:
## - "texture": textura PNG estática (sky.png legacy)
## - "gradient": degradado procedural según hora del día (shader spatial)
##
## En ambos modos, el sprite se adosa a la cámara: cada frame se coloca a
## [param distance] delante de la cámara y se alinea a su orientación.
## El pixel_size se calcula desde el FOV para llenar el frustum completo.
##
## pixel_size calculation mirrors `game/crates/skybox-engine/src/skybox.rs`.
## Same inputs must produce same outputs. Rust is the authority.

const COVERAGE_FACTOR := 1.05  # mirrors Rust skybox.rs COVERAGE_FACTOR
const PIXEL_SIZE_SNAP := 0.05  # mirrors Rust skybox.rs PIXEL_SIZE_SNAP
const SKY_GRADIENT_SHADER := "res://addons/formula90s/shaders/sky_gradient.gdshader"

var config: BackgroundSkyboxConfig
var _sprite: Sprite3D
var _camera: Camera3D
var _manual_pixel_size := false


func setup(cfg: BackgroundSkyboxConfig) -> bool:
	if cfg == null:
		push_error("BackgroundSkybox: config nula.")
		return false
	if cfg.mode == "gradient":
		return _setup_gradient(cfg)
	return _setup_texture(cfg)


func _setup_texture(cfg: BackgroundSkyboxConfig) -> bool:
	if cfg.texture_path.is_empty():
		push_error("BackgroundSkybox: mode=texture pero no se especifica 'texture'.")
		return false
	var tex := load(cfg.texture_path) as Texture2D
	if tex == null:
		push_error("BackgroundSkybox: no se pudo cargar '%s'." % cfg.texture_path)
		return false
	config = cfg
	_create_sprite(tex)
	return true


func _setup_gradient(cfg: BackgroundSkyboxConfig) -> bool:
	var shader := load(SKY_GRADIENT_SHADER) as Shader
	if shader == null:
		push_error("BackgroundSkybox: no se pudo cargar el shader '%s'." % SKY_GRADIENT_SHADER)
		return false
	config = cfg
	_create_sprite(_make_dummy_texture())
	var mat := ShaderMaterial.new()
	mat.shader = shader
	_apply_gradient_uniforms(mat, cfg.gradient)
	_sprite.material_override = mat
	return true


func _create_sprite(tex: Texture2D) -> void:
	_sprite = Sprite3D.new()
	_sprite.name = "SkySprite"
	_sprite.texture = tex
	_sprite.texture_filter = BaseMaterial3D.TEXTURE_FILTER_NEAREST
	_sprite.shaded = false
	_sprite.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	_sprite.gi_mode = GeometryInstance3D.GI_MODE_DISABLED
	_sprite.render_priority = -100
	_sprite.alpha_cut = Sprite3D.ALPHA_CUT_DISABLED
	# BG3-009 fix: skybox must ALWAYS render behind everything.
	# no_depth_test=true disables depth testing so the sprite renders regardless
	# of depth buffer state. Combined with render_priority=-100, it renders first.
	# The shader uses depth_draw_never to prevent writing to the depth buffer,
	# ensuring closer objects (mountain layers) can render on top.
	_sprite.no_depth_test = true
	if config.pixel_size > 0.0:
		_sprite.pixel_size = config.pixel_size
		_manual_pixel_size = true
	add_child(_sprite)


func set_camera_source(cam: Camera3D) -> void:
	_camera = cam


func _process(_delta: float) -> void:
	if not is_instance_valid(_camera) or not _camera.is_inside_tree() or _sprite == null:
		return
	if not _manual_pixel_size:
		_resolve_pixel_size()
	var cam_basis := _camera.global_transform.basis
	_sprite.global_position = _camera.global_position - cam_basis.z * config.distance
	_sprite.global_basis = cam_basis * Basis(Vector3(-1, 0, 0), Vector3(0, 1, 0), Vector3(0, 0, -1))


func _resolve_pixel_size() -> void:
	if _sprite == null or _sprite.texture == null or not is_instance_valid(_camera):
		return
	var viewport := get_viewport()
	if viewport == null:
		return
	var tex_size := _sprite.texture.get_size()
	_sprite.pixel_size = compute_pixel_size_from_camera(viewport, _camera, config.distance, tex_size)


static func compute_pixel_size_from_camera(
	viewport: Viewport,
	camera: Camera3D,
	distance: float,
	texture_size: Vector2i,
	coverage_factor: float = COVERAGE_FACTOR
) -> float:
	if viewport == null or camera == null or texture_size.y <= 0 or camera.fov <= 0.0:
		return 1.0
	var fov_rad := deg_to_rad(camera.fov)
	var viewport_size := viewport.get_visible_rect().size
	var aspect := viewport_size.x / maxf(viewport_size.y, 0.001)
	# camera.fov is the vertical FOV when keep_aspect == KEEP_HEIGHT (Godot default)
	# and horizontal FOV when KEEP_WIDTH.  Compute both frustum dimensions so a
	# square quad (e.g. 1×1 dummy texture) covers the entire viewport.
	var visible_fov := 2.0 * absf(distance) * tan(fov_rad * 0.5)
	var visible_h: float
	var visible_w: float
	if camera.keep_aspect == Camera3D.KEEP_HEIGHT:
		visible_h = visible_fov
		visible_w = visible_fov * aspect
	else:
		visible_w = visible_fov
		visible_h = visible_fov / maxf(aspect, 0.001)
	var largest := maxf(visible_h, visible_w)
	var needed_px := largest * coverage_factor / float(texture_size.y)
	return snappedf(maxf(needed_px, 0.1), PIXEL_SIZE_SNAP)


static func _apply_gradient_uniforms(mat: ShaderMaterial, gradient: Dictionary) -> void:
	if gradient.has("zenith_color"):
		var c = gradient["zenith_color"]
		if c is Array and c.size() >= 3:
			mat.set_shader_parameter("zenith_color", Vector3(float(c[0]), float(c[1]), float(c[2])))
	if gradient.has("horizon_color"):
		var c = gradient["horizon_color"]
		if c is Array and c.size() >= 3:
			mat.set_shader_parameter("horizon_color", Vector3(float(c[0]), float(c[1]), float(c[2])))
	if gradient.has("ground_color"):
		var c = gradient["ground_color"]
		if c is Array and c.size() >= 3:
			mat.set_shader_parameter("ground_color", Vector3(float(c[0]), float(c[1]), float(c[2])))
	if gradient.has("horizon_sharpness"):
		mat.set_shader_parameter("horizon_sharpness", float(gradient["horizon_sharpness"]))
	if gradient.has("ground_start"):
		mat.set_shader_parameter("ground_start", float(gradient["ground_start"]))


static func _make_dummy_texture() -> ImageTexture:
	var img := Image.create(1, 1, false, Image.FORMAT_RGBA8)
	img.set_pixel(0, 0, Color.WHITE)
	return ImageTexture.create_from_image(img)
