extends SceneTree

const VEHICLE_SCENE := "res://scenes/vehicles/f1_2030_v10/f1_2030_v10_rust.tscn"


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var packed := load(VEHICLE_SCENE) as PackedScene
	if packed == null:
		printerr("[FAIL] Could not load F1 2030 V10 scene.")
		quit(1)
		return
	var instance := packed.instantiate()
	root.add_child(instance)
	var vehicle := instance.get_node("VehicleRigidBody") as RigidBody3D
	vehicle.freeze = true

	var environment := Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color(0.12, 0.15, 0.19)
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	environment.ambient_light_color = Color(0.9, 0.92, 1.0)
	environment.ambient_light_energy = 1.2
	var world_environment := WorldEnvironment.new()
	world_environment.environment = environment
	instance.add_child(world_environment)

	var light := DirectionalLight3D.new()
	light.rotation_degrees = Vector3(-48.0, -32.0, 0.0)
	light.light_energy = 1.5
	light.shadow_enabled = true
	instance.add_child(light)

	var camera := Camera3D.new()
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	instance.add_child(camera)
	camera.make_current()

	await physics_frame
	await physics_frame
	var views := [
		["front", Vector3(0.0, 0.65, -6.0), Vector3(0.0, 0.05, 0.0), Vector3.UP, 2.4],
		["rear", Vector3(0.0, 0.65, 6.0), Vector3(0.0, 0.05, 0.0), Vector3.UP, 2.4],
		["left", Vector3(-7.0, 0.65, 0.0), Vector3(0.0, 0.05, 0.0), Vector3.UP, 3.2],
		["top", Vector3(0.0, 8.0, 0.0), Vector3.ZERO, Vector3(0.0, 0.0, -1.0), 5.4],
	]
	for view in views:
		camera.position = view[1]
		camera.look_at(view[2], view[3])
		camera.size = view[4]
		await process_frame
		await RenderingServer.frame_post_draw
		var output := "user://f1_2030_alignment_%s.png" % view[0]
		var error := root.get_texture().get_image().save_png(output)
		if error != OK:
			printerr("[FAIL] Could not save ", output)
			quit(1)
			return
		print("[PASS] Captured ", ProjectSettings.globalize_path(output))

	instance.queue_free()
	quit(0)
