extends SceneTree

## Visual capture of the F1 2030 suspension assembly: front-3/4 view at rest and
## under braking (nose dive) so the linkage motion is visible. Run non-headless;
## PNGs are written to user://f1_2030_suspension_*.png.

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

	var ground := StaticBody3D.new()
	var col := CollisionShape3D.new()
	var box := BoxShape3D.new()
	box.size = Vector3(100.0, 1.0, 500.0)
	col.shape = box
	col.position = Vector3(0.0, -0.5, 0.0)
	ground.add_child(col)
	root.add_child(ground)
	instance.position = Vector3(0.0, 0.40, 0.0)

	var vehicle := instance.get_node("VehicleRigidBody") as RigidBody3D
	vehicle.enable_player_input = false

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
	camera.projection = Camera3D.PROJECTION_PERSPECTIVE
	instance.add_child(camera)
	camera.make_current()

	# Settle so deferred children and the physics core initialize.
	for i in range(12):
		await physics_frame

	# Front-left 3/4 close-up on the FL corner.
	camera.position = Vector3(-2.4, 0.9, -0.6)
	camera.look_at(Vector3(-0.75, 0.05, -1.45), Vector3.UP)

	await process_frame
	await RenderingServer.frame_post_draw
	var error := root.get_texture().get_image().save_png("user://f1_2030_suspension_rest.png")
	if error != OK:
		printerr("[FAIL] Could not save rest capture ", error)
		quit(1)
		return

	# Braking nose dive: front suspension compresses, links animate.
	vehicle.brake_amount = 1.0
	for i in range(50):
		await physics_frame
	await process_frame
	await RenderingServer.frame_post_draw
	error = root.get_texture().get_image().save_png("user://f1_2030_suspension_brake.png")
	if error != OK:
		printerr("[FAIL] Could not save brake capture ", error)
		quit(1)
		return

	print("[PASS] Suspension captures written to user://")
	vehicle.freeze = true
	vehicle.free()
	quit(0)