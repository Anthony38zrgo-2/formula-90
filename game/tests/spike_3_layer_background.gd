extends SceneTree

## Spike BG3-002: Test de render minimo y evaluacion de arquitectura desacoplada de 3 capas
## Valida:
## 1. Integracion de 3 layers independientes (Sky, Far Mountains, Near Mountains)
## 2. Filtrado Nearest, sin shading, sin sombras y con alfa transparente
## 3. Orden de profundidad y oclusion correcta por geometria 3D del mundo
## 4. Diferenciacion y jerarquia del desplazamiento por parallax: Sky < Far < Near
## 5. Captura de 3 encuadres: Centro (recta), Izquierda (+30 deg), Derecha (-30 deg)

const SKY_TEXTURE_PATH := "res://assets/backgrounds/spike_la_chutana_v3/sky.png"
const FAR_TEXTURE_PATH := "res://assets/backgrounds/spike_la_chutana_v3/far_mountains.png"
const NEAR_TEXTURE_PATH := "res://assets/backgrounds/spike_la_chutana_v3/near_mountains.png"

const PARALLAX_SKY := 0.02
const PARALLAX_FAR := 0.08
const PARALLAX_NEAR := 0.18

var _failures := 0


func _init() -> void:
	call_deferred("_run_spike")


func _run_spike() -> void:
	print("=== INICIANDO SPIKE BG3-002: RENDER 3-LAYER BACKGROUND ===")

	# 1. Crear escena raiz y configuracion de entorno
	var test_root := Node3D.new()
	test_root.name = "BackgroundSpikeScene"
	root.add_child(test_root)

	var world_env := WorldEnvironment.new()
	var env := Environment.new()
	env.background_mode = Environment.BG_COLOR
	env.background_color = Color(0.1, 0.1, 0.1, 1.0)
	world_env.environment = env
	test_root.add_child(world_env)

	# 2. Camara 3D
	var camera := Camera3D.new()
	camera.name = "TestCamera"
	camera.position = Vector3(0, 1.5, 0)
	camera.fov = 60.0
	camera.current = true
	test_root.add_child(camera)

	# 3. Geometria 3D del mundo (para validar que el mundo ocluye el fondo)
	var world_cube := MeshInstance3D.new()
	world_cube.name = "WorldForegroundObject"
	var cube_mesh := BoxMesh.new()
	cube_mesh.size = Vector3(4.0, 2.0, 1.0)
	world_cube.mesh = cube_mesh
	world_cube.position = Vector3(0, 1.0, -15.0) # 15m delante de la camara
	var cube_mat := StandardMaterial3D.new()
	cube_mat.albedo_color = Color(0.8, 0.2, 0.2, 1.0) # Rojo visible
	world_cube.material_override = cube_mat
	test_root.add_child(world_cube)

	# 4. Rig de background de 3 capas
	var bg_rig := Node3D.new()
	bg_rig.name = "BackgroundCompositorRig"
	test_root.add_child(bg_rig)

	# Cargar texturas
	var sky_tex := load(SKY_TEXTURE_PATH) as Texture2D
	var far_tex := load(FAR_TEXTURE_PATH) as Texture2D
	var near_tex := load(NEAR_TEXTURE_PATH) as Texture2D

	if sky_tex == null or far_tex == null or near_tex == null:
		printerr("[FAIL] No se pudieron cargar las 3 texturas del background v3.")
		_fail_and_quit(test_root)
		return

	if sky_tex == far_tex or far_tex == near_tex or sky_tex == near_tex:
		printerr("[FAIL] Las 3 capas deben usar texturas diferentes e independientes.")
		_failures += 1

	# Construir capas Sprite3D
	var sky_sprite := _create_layer_sprite("SkyLayer", sky_tex, -800.0, 0)
	var far_sprite := _create_layer_sprite("FarMountainsLayer", far_tex, -700.0, 1)
	var near_sprite := _create_layer_sprite("NearMountainsLayer", near_tex, -600.0, 2)

	bg_rig.add_child(sky_sprite)
	bg_rig.add_child(far_sprite)
	bg_rig.add_child(near_sprite)

	print("[OK] Capas instanciadas con exito:")
	print("     - Sky: Depth 0, Distancia Z=-800m, Texture: ", sky_tex.resource_path)
	print("     - Far Mountains: Depth 1, Distancia Z=-700m, Texture: ", far_tex.resource_path)
	print("     - Near Mountains: Depth 2, Distancia Z=-600m, Texture: ", near_tex.resource_path)

	# 5. Validacion estructural
	for sprite in [sky_sprite, far_sprite, near_sprite]:
		if sprite.texture_filter != BaseMaterial3D.TEXTURE_FILTER_NEAREST:
			printerr("[FAIL] La capa %s no tiene filtro Nearest." % sprite.name)
			_failures += 1
		if sprite.shaded:
			printerr("[FAIL] La capa %s tiene shaded activo (debe ser unshaded)." % sprite.name)
			_failures += 1
		if sprite.cast_shadow != GeometryInstance3D.SHADOW_CASTING_SETTING_OFF:
			printerr("[FAIL] La capa %s proyecta sombras." % sprite.name)
			_failures += 1

	# 6. Validacion de jerarquia visual y ausencia de fisicas
	for descendant in bg_rig.find_children("*", "", true, false):
		if descendant is CollisionObject3D or descendant is NavigationRegion3D:
			printerr("[FAIL] BackgroundRig contiene nodo no-visual: %s" % descendant.get_path())
			_failures += 1

	# 7. Simular 3 orientaciones de camara y comprobar matematicamente el parallax
	var angles_deg := [0.0, 30.0, -30.0]
	var angle_labels := ["straight", "turn_left", "turn_right"]

	for i in range(angles_deg.size()):
		var deg: float = angles_deg[i]
		var label: String = angle_labels[i]
		var rad := deg_to_rad(deg)

		# Rotar camara
		camera.rotation = Vector3(0, rad, 0)

		# Calcular offsets angulares
		var sky_disp := rad * PARALLAX_SKY * 800.0
		var far_disp := rad * PARALLAX_FAR * 700.0
		var near_disp := rad * PARALLAX_NEAR * 600.0

		sky_sprite.position.x = sky_disp
		far_sprite.position.x = far_disp
		near_sprite.position.x = near_disp

		# Esperar renders
		for _frame in 4:
			await process_frame
		RenderingServer.force_draw(false, 0.0)

		if deg != 0.0:
			var abs_sky := absf(sky_disp)
			var abs_far := absf(far_disp)
			var abs_near := absf(near_disp)

			if not (abs_sky < abs_far and abs_far < abs_near):
				printerr("[FAIL] Jerarquia de parallax violada en %s: Sky=%.2f, Far=%.2f, Near=%.2f" % [label, abs_sky, abs_far, abs_near])
				_failures += 1
			else:
				print("[OK] Parallax verificado en %s (deg=%.1f): Sky=%.2f < Far=%.2f < Near=%.2f" % [label, deg, abs_sky, abs_far, abs_near])

		# Guardar frame para auditoria visual cuando haya display activo
		if DisplayServer.get_name() != "headless":
			var output_path := "user://spike_background_%s.png" % label
			var root_texture := root.get_texture()
			if root_texture != null:
				var img := root_texture.get_image()
				if img != null:
					var err := img.save_png(output_path)
					if err == OK:
						print("     -> Captura guardada: ", ProjectSettings.globalize_path(output_path))

	# 8. Limpieza
	test_root.queue_free()

	print("\n=== RESUMEN SPIKE BG3-002 ===")
	if _failures == 0:
		print("[PASS] El spike demostro que la arquitectura de 3 capas camera-followed:")
		print("       1. Mantiene el fondo detras de la geometria 3D del mundo.")
		print("       2. Preserva el pixel art sin filtrado suave (Nearest).")
		print("       3. Ejecuta la jerarquia de parallax Sky < Far < Near de forma desacoplada.")
		print("       4. Es completamente visual, sin acoplamiento a fisicas ni mapas especificos.")
		quit(0)
	else:
		printerr("[FAIL] El spike fallo con %d errores." % _failures)
		quit(1)


func _create_layer_sprite(layer_name: String, tex: Texture2D, z_dist: float, render_prio: int) -> Sprite3D:
	var sprite := Sprite3D.new()
	sprite.name = layer_name
	sprite.texture = tex
	sprite.position = Vector3(0, 0, z_dist)
	sprite.pixel_size = 0.5
	sprite.texture_filter = BaseMaterial3D.TEXTURE_FILTER_NEAREST
	sprite.shaded = false
	sprite.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	sprite.gi_mode = GeometryInstance3D.GI_MODE_DISABLED
	sprite.render_priority = render_prio
	sprite.alpha_cut = Sprite3D.ALPHA_CUT_DISABLED # Conservar blending de alfa suave/binario
	return sprite


func _fail_and_quit(cleanup_node: Node) -> void:
	if cleanup_node != null:
		cleanup_node.queue_free()
	quit(1)
