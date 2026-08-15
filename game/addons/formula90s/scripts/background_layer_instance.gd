class_name BackgroundLayerInstance
extends Sprite3D

## Instancia individual de renderizado para una capa de background multicapa (Formula-90)
## Aplica filtrado Nearest, configuracion unshaded, orden de profundidad y snapping a pixel.

var layer_config: BackgroundLayerConfig


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
	alpha_cut = Sprite3D.ALPHA_CUT_DISABLED # Preserva mezcla alfa
	
	scale = Vector3(config.scale.x, config.scale.y, 1.0)
	position = Vector3(config.offset.x, config.offset.y, config.distance_z)
	return true


func update_parallax(yaw_rad: float, pitch_rad: float) -> void:
	if layer_config == null:
		return
	
	var radius := absf(layer_config.distance_z)
	
	# Desplazamiento horizontal (yaw)
	var raw_x := (yaw_rad * layer_config.parallax_x * radius) + layer_config.offset.x
	
	# Desplazamiento vertical (pitch)
	var raw_y := (pitch_rad * layer_config.parallax_y * radius) + layer_config.offset.y
	
	# Pixel Snapping opcional para mantener consistencia de pixel art
	if layer_config.pixel_snap and layer_config.pixel_size > 0.0:
		var step_x := layer_config.pixel_size * layer_config.scale.x
		var step_y := layer_config.pixel_size * layer_config.scale.y
		if step_x > 0.0001:
			raw_x = roundf(raw_x / step_x) * step_x
		if step_y > 0.0001:
			raw_y = roundf(raw_y / step_y) * step_y
	
	position.x = raw_x
	position.y = raw_y
