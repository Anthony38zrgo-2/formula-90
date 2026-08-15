class_name BackgroundLayerInstance
extends Sprite3D

## Instancia individual de renderizado para una capa de background multicapa (Formula-90)
## Aplica filtrado Nearest, configuracion unshaded, orden de profundidad y snapping a pixel.

const SKY_SHADER_PATH := "res://addons/formula90s/shaders/background_sky_follow.gdshader"

var layer_config: BackgroundLayerConfig
var _sky_material: ShaderMaterial


func setup_layer(config: BackgroundLayerConfig) -> bool:
	if config == null:
		push_error("BackgroundLayerInstance: No se puede inicializar con BackgroundLayerConfig nulo.")
		return false
	
	layer_config = config
	name = String(config.id)
	
	var tex = load(config.texture_path) as Texture2D
	if tex == null:
		push_error("BackgroundLayerInstance: Fallo al cargar la textura en '%s'." % config.texture_path)
		return false
	
	texture = tex
	texture_filter = BaseMaterial3D.TEXTURE_FILTER_NEAREST
	shaded = false
	cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	gi_mode = GeometryInstance3D.GI_MODE_DISABLED
	render_priority = config.depth
	pixel_size = config.pixel_size
	alpha_cut = Sprite3D.ALPHA_CUT_DISABLED
	
	# Modo FOLLOW: el cielo siempre llena la pantalla sin importar la rotacion.
	# Usa un shader de proyeccion equirectangular en vez de un Sprite3D plano.
	if config.parallax_mode == BackgroundLayerConfig.ParallaxMode.FOLLOW:
		_setup_sky_shader(tex)
	
	scale = Vector3(config.scale.x, config.scale.y, 1.0)
	var snapped := _apply_pixel_snap(config.offset.x, config.offset.y)
	position = Vector3(snapped.x, snapped.y, config.distance_z)
	return true


func _setup_sky_shader(tex: Texture2D) -> void:
	var shader := load(SKY_SHADER_PATH) as Shader
	if shader == null:
		push_error("BackgroundLayerInstance: No se pudo cargar el sky shader en '%s'." % SKY_SHADER_PATH)
		return
	_sky_material = ShaderMaterial.new()
	_sky_material.shader = shader
	_sky_material.set_shader_parameter("sky_texture", tex)
	_sky_material.set_shader_parameter("camera_euler", Vector2.ZERO)
	material_override = _sky_material
	# Mantener la textura para que el Sprite3D tenga aabb valida y no sea
	# frustum-culled. El shader sobreescribe la visual con SCREEN_UV.
	# Aumentar pixel_size para que el sprite sea siempre visible en el frustum.
	pixel_size = 100.0


## Calcula la posicion de la capa segun su modo de parallax:
## - FOLLOW: siempre centrado en la camara (para cielo/skybox). Actualiza
##   el shader con los angulos de la camara para que la proyeccion sea correcta.
## - PARALLAX: desplazamiento angular desde la camara (para montanas/capas depth).
func update_parallax(yaw_rad: float, pitch_rad: float) -> void:
	if layer_config == null:
		return
	
	if layer_config.parallax_mode == BackgroundLayerConfig.ParallaxMode.FOLLOW:
		# Actualizar el shader con la direccion de vista de la camara.
		if _sky_material != null:
			_sky_material.set_shader_parameter("camera_euler", Vector2(pitch_rad, yaw_rad))
		# Posicion siempre en el centro del controller (que sigue la camara).
		var snapped := _apply_pixel_snap(layer_config.offset.x, layer_config.offset.y)
		position.x = snapped.x
		position.y = snapped.y
		return
	
	# Modo PARALLAX: offset angular proporcional a yaw/pitch y distancia.
	# Usar sin() para que el offset sea correcto a cualquier angulo de giro.
	var radius := absf(layer_config.distance_z)
	var raw_x := (sin(yaw_rad) * layer_config.parallax_x * radius) + layer_config.offset.x
	var raw_y := (sin(pitch_rad) * layer_config.parallax_y * radius) + layer_config.offset.y
	
	var snapped := _apply_pixel_snap(raw_x, raw_y)
	position.x = snapped.x
	position.y = snapped.y


## Aplica pixel snapping consistente en setup y parallax para evitar
## desalineacion de un frame entre posicion inicial y posicion calculada.
func _apply_pixel_snap(value_x: float, value_y: float) -> Vector2:
	if layer_config != null and layer_config.pixel_snap and layer_config.pixel_size > 0.0:
		var step_x := layer_config.pixel_size * layer_config.scale.x
		var step_y := layer_config.pixel_size * layer_config.scale.y
		if step_x > 0.0001:
			value_x = roundf(value_x / step_x) * step_x
		if step_y > 0.0001:
			value_y = roundf(value_y / step_y) * step_y
	return Vector2(value_x, value_y)
