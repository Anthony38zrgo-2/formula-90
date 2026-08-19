class_name BackgroundLayerInstance
extends Sprite3D

## Instancia individual de renderizado para una capa parallax (Formula-90).
## Soporta capas con textura (PNG) y capas procedurales (shader FBM).
## Aplica filtrado Nearest, configuracion unshaded, orden de profundidad y snapping a pixel.
## Cuando repeat_x es true, crea copias de tile a los lados para cobertura continua.
##
## Parallax math mirrors `game/crates/skybox-engine/src/parallax.rs`.
## Same inputs must produce same outputs. Rust is the authority.

var layer_config: BackgroundLayerConfig
var _tile_copies: Array[Sprite3D] = []
var _sprite_width: float = 0.0


func setup_layer(config: BackgroundLayerConfig) -> bool:
	if config == null:
		push_error("BackgroundLayerInstance: No se puede inicializar con BackgroundLayerConfig nulo.")
		return false
	
	layer_config = config
	name = String(config.id)
	
	texture_filter = BaseMaterial3D.TEXTURE_FILTER_NEAREST
	shaded = false
	cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	gi_mode = GeometryInstance3D.GI_MODE_DISABLED
	render_priority = config.depth
	alpha_cut = Sprite3D.ALPHA_CUT_DISABLED
	
	if config.procedural and not config.shader_path.is_empty():
		_setup_procedural(config)
	else:
		_setup_textured(config)
	
	scale = Vector3(config.scale.x, config.scale.y, 1.0)
	var snapped := _apply_pixel_snap(config.offset.x, config.offset.y)
	position = Vector3(snapped.x, snapped.y, config.distance_z)

	# Calculate sprite width for tiling (in local space before parent scale)
	if texture != null and pixel_size > 0.0:
		_sprite_width = texture.get_width() * pixel_size

	# Create horizontal tile copies so mountains wrap continuously
	if config.repeat_x and _sprite_width > 0.0:
		_create_tile_copies()

	return true


func _setup_textured(config: BackgroundLayerConfig) -> void:
	var tex = load(config.texture_path) as Texture2D
	if tex == null:
		push_error("BackgroundLayerInstance: Fallo al cargar la textura en '%s'." % config.texture_path)
		return
	texture = tex
	pixel_size = config.pixel_size


func _setup_procedural(config: BackgroundLayerConfig) -> void:
	var shader := load(config.shader_path) as Shader
	if shader == null:
		push_error("BackgroundLayerInstance: Fallo al cargar el shader en '%s'." % config.shader_path)
		return
	var mat := ShaderMaterial.new()
	mat.shader = shader
	_apply_uniforms(mat, config.uniforms)
	# Textura dummy 1x1 para que el Sprite3D tenga aabb válido (no frustum-culled)
	var img := Image.create(1, 1, false, Image.FORMAT_RGBA8)
	img.set_pixel(0, 0, Color.WHITE)
	texture = ImageTexture.create_from_image(img)
	# pixel_size alto para que el sprite sea siempre visible en el frustum
	pixel_size = 100.0
	mat.set_shader_parameter("quad_size", pixel_size)
	material_override = mat


## Crea dos copias del sprite a izquierda y derecha para tiling horizontal continuo.
func _create_tile_copies() -> void:
	for side in [-1, 1]:
		var copy := Sprite3D.new()
		copy.texture = texture
		copy.texture_filter = texture_filter
		copy.shaded = shaded
		copy.cast_shadow = cast_shadow
		copy.gi_mode = gi_mode
		copy.render_priority = render_priority
		copy.alpha_cut = alpha_cut
		copy.pixel_size = pixel_size
		if material_override != null:
			copy.material_override = material_override
		copy.position = Vector3(float(side) * _sprite_width, 0.0, 0.0)
		copy.name = "TileCopy_%s" % ("L" if side == -1 else "R")
		add_child(copy)
		_tile_copies.append(copy)


func _apply_uniforms(mat: ShaderMaterial, uniforms: Dictionary) -> void:
	for key in uniforms:
		var value = uniforms[key]
		if value is Array and value.size() == 3:
			mat.set_shader_parameter(key, Vector3(float(value[0]), float(value[1]), float(value[2])))
		elif value is Array and value.size() == 4:
			mat.set_shader_parameter(key, Vector4(float(value[0]), float(value[1]), float(value[2]), float(value[3])))
		elif value is float or value is int:
			mat.set_shader_parameter(key, float(value))
		elif value is bool:
			mat.set_shader_parameter(key, value)


## Desplazamiento parallax proporcional a yaw/pitch.
## El controller rota con el yaw de la cámara, así que el offset en X local
## crea la ilusión de profundidad: capas lejanas se desplazan poco, cercanas más.
## Con repeat_x, el offset se envuelve módulo el ancho del sprite para tiling continuo.
func update_parallax(yaw_rad: float, pitch_rad: float) -> void:
	if layer_config == null:
		return
	
	var radius := absf(layer_config.distance_z)
	# Negativo: cuando la cámara gira a la derecha (yaw+), las montañas
	# se desplazan a la izquierda en el espacio local de la cámara.
	var raw_x := -(yaw_rad * layer_config.parallax_x * radius) + layer_config.offset.x
	var raw_y := (pitch_rad * layer_config.parallax_y * radius) + layer_config.offset.y
	
	if layer_config.repeat_x and _sprite_width > 0.0:
		# Wrap modular para tiling continuo: el sprite principal siempre queda
		# dentro de [-ancho/2, +ancho/2] y las copias cubren los lados.
		raw_x = fposmod(raw_x + _sprite_width * 0.5, _sprite_width) - _sprite_width * 0.5
	
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
