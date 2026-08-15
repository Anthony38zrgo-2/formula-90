class_name BackgroundSkybox
extends Node3D

## Skybox desacoplado del sistema de capas parallax (Formula-90).
##
## Un Sprite3D adosado a la camara: cada frame se coloca a [param distance]
## delante de la camara y se alinea a su orientacion. El pixel_size se calcula
## desde el FOV y el tamano del viewport para llenar el frustum completo, por
## lo que el cielo siempre cubre la pantalla sin importar la rotacion de la camara.

const COVERAGE_FACTOR := 1.05
const PIXEL_SIZE_SNAP := 0.05

var config: BackgroundSkyboxConfig
var _sprite: Sprite3D
var _camera: Camera3D
var _manual_pixel_size := false


func setup(cfg: BackgroundSkyboxConfig) -> bool:
	if cfg == null or cfg.texture_path.is_empty():
		push_error("BackgroundSkybox: config invalida o sin textura.")
		return false
	
	var tex := load(cfg.texture_path) as Texture2D
	if tex == null:
		push_error("BackgroundSkybox: no se pudo cargar la textura '%s'." % cfg.texture_path)
		return false
	
	config = cfg
	_sprite = Sprite3D.new()
	_sprite.name = "SkySprite"
	_sprite.texture = tex
	_sprite.texture_filter = BaseMaterial3D.TEXTURE_FILTER_NEAREST
	_sprite.shaded = false
	_sprite.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	_sprite.gi_mode = GeometryInstance3D.GI_MODE_DISABLED
	_sprite.render_priority = -100
	_sprite.alpha_cut = Sprite3D.ALPHA_CUT_DISABLED
	if config.pixel_size > 0.0:
		_sprite.pixel_size = config.pixel_size
		_manual_pixel_size = true
	add_child(_sprite)
	return true


func set_camera_source(cam: Camera3D) -> void:
	_camera = cam


func _process(_delta: float) -> void:
	if not is_instance_valid(_camera) or not _camera.is_inside_tree() or _sprite == null:
		return
	
	# Re-calcular el pixel_size desde el FOV actual de la camara: el FOV puede
	# cambiar en runtime (p.ej. chase camera), y el calculo es barato.
	if not _manual_pixel_size:
		_resolve_pixel_size()
	
	# Posicionar el sprite delante de la camara, alineado a su orientacion.
	# La cara frontal del sprite (+Z) debe apuntar hacia la camara para evitar
	# el espejado horizontal de la textura.
	var cam_basis := _camera.global_transform.basis
	_sprite.global_position = _camera.global_position - cam_basis.z * config.distance
	_sprite.global_basis = cam_basis * Basis(Vector3(-1, 0, 0), Vector3(0, 1, 0), Vector3(0, 0, -1))


## Calcula el pixel_size minimo para que el sprite llene el frustum de la camara:
## sprite_world_h = texture_h * pixel_size debe ser >= visible_h * COVERAGE_FACTOR.
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
	var fov_h_rad := deg_to_rad(camera.fov)
	var viewport_size := viewport.get_visible_rect().size
	var aspect := viewport_size.x / maxf(viewport_size.y, 0.001)
	var fov_v_rad := 2.0 * atan(tan(fov_h_rad * 0.5) / aspect)
	var visible_h := 2.0 * absf(distance) * tan(fov_v_rad * 0.5)
	var needed_px := visible_h * coverage_factor / float(texture_size.y)
	return snappedf(maxf(needed_px, 0.1), PIXEL_SIZE_SNAP)
