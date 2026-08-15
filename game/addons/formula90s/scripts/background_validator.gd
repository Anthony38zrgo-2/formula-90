class_name BackgroundValidator
extends RefCounted

## Validador estricto para configuraciones y presets de background multicapa (Formula-90)
## Garantiza que los errores de configuracion sean legibles y explicitos,
## evitando fondos negros silenciosos o comportamientos indefinidos.

class ValidationResult:
	var is_valid: bool = true
	var errors: Array[String] = []
	var warnings: Array[String] = []
	
	func add_error(message: String) -> void:
		is_valid = false
		errors.append(message)
	
	func add_warning(message: String) -> void:
		warnings.append(message)
	
	func get_error_summary() -> String:
		if is_valid:
			return "OK"
		return "\n".join(errors)
	
	func log_diagnostics(preset_id: String = "") -> void:
		var prefix := "[BackgroundValidator]"
		if not preset_id.is_empty():
			prefix = "[BackgroundValidator: %s]" % preset_id
			
		for w in warnings:
			print("%s [WARN] %s" % [prefix, w])
		for e in errors:
			printerr("%s [FAIL] %s" % [prefix, e])


static func validate_preset(preset: BackgroundPreset) -> ValidationResult:
	var result := ValidationResult.new()
	if preset == null:
		result.add_error("El preset de background es nulo (null).")
		return result
	
	if preset.id.is_empty():
		result.add_error("El preset no tiene un 'id' valido o esta vacio.")
	
	if preset.layers.is_empty():
		result.add_error("El preset '%s' no contiene ninguna capa ('layers' esta vacio)." % preset.id)
		return result
	
	var seen_ids := {}
	var seen_depths := {}
	
	for i in range(preset.layers.size()):
		var layer := preset.layers[i]
		var layer_context := "Capa #%d" % i
		
		if layer == null:
			result.add_error("%s en preset '%s' es nula (null)." % [layer_context, preset.id])
			continue
		
		if layer.id.is_empty():
			result.add_error("%s no declara un 'id' obligatorio." % layer_context)
		else:
			layer_context = "Capa '%s' (index %d)" % [layer.id, i]
			if seen_ids.has(layer.id):
				result.add_error("%s duplica el identificador de capa '%s'." % [layer_context, layer.id])
			else:
				seen_ids[layer.id] = true
		
		# Validar depth
		if layer.depth < 0:
			result.add_error("%s tiene un depth negativo (%d). Los depths deben ser >= 0." % [layer_context, layer.depth])
		elif seen_depths.has(layer.depth):
			result.add_error("%s tiene depth duplicado (%d) ya usado por '%s'." % [layer_context, layer.depth, seen_depths[layer.depth]])
		else:
			seen_depths[layer.depth] = layer.id
		
		# Validar existencia de textura
		if layer.texture_path.is_empty():
			result.add_error("%s no especifica 'texture_path' o 'texture'." % layer_context)
		else:
			if not (ResourceLoader.exists(layer.texture_path) or FileAccess.file_exists(layer.texture_path)):
				result.add_error("%s especifica una textura inexistente: '%s'." % [layer_context, layer.texture_path])
		
		# Validar escala positiva
		if layer.scale.x <= 0.0 or layer.scale.y <= 0.0:
			result.add_error("%s tiene escala invalida o no positiva (%s). La escala debe ser > 0.0." % [layer_context, layer.scale])
		
		# Validar pixel size positivo
		if layer.pixel_size <= 0.0:
			result.add_error("%s tiene pixel_size invalido o no positivo (%.4f). Debe ser > 0.0." % [layer_context, layer.pixel_size])
		
		# Validar valores numericos finitos
		if is_nan(layer.parallax_x) or is_inf(layer.parallax_x) or is_nan(layer.parallax_y) or is_inf(layer.parallax_y):
			result.add_error("%s tiene valores no finitos de parallax (x=%.2f, y=%.2f)." % [layer_context, layer.parallax_x, layer.parallax_y])
			
		if is_nan(layer.offset.x) or is_inf(layer.offset.x) or is_nan(layer.offset.y) or is_inf(layer.offset.y):
			result.add_error("%s tiene valores no finitos de offset (x=%.2f, y=%.2f)." % [layer_context, layer.offset.x, layer.offset.y])
	
	return result


static func validate_json_dict(dict: Dictionary) -> ValidationResult:
	var preset := BackgroundPreset.from_dict(dict)
	return validate_preset(preset)
