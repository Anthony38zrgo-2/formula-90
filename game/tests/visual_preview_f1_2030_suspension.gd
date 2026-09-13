extends SceneTree

## Deterministic visual-only review. No native vehicle, forces or runtime DLLs.
## Copy this script, the visual scripts, JSON and five GLBs to an isolated Godot
## project to review without loading the game's GDExtensions.
const CONFIG := "res://data/vehicles/f1_2030/f1_2030_v10_physics.json"
const ASSETS := "res://assets/models/vehicles/f1-2030/"

func _init() -> void:
	call_deferred("run")

func run() -> void:
	root.size = Vector2i(1440, 1000)
	var geometry := SuspensionGeometry.from_json_path(CONFIG)
	var car := Node3D.new()
	root.add_child(car)
	var chassis := (load(ASSETS + "f1_2030_v10_chassis.glb") as PackedScene).instantiate()
	car.add_child(chassis)
	hide_baked(chassis)
	var links := SuspensionLinkVisual.new()
	links.setup(geometry)
	car.add_child(links)
	var wheels: Array[Node3D] = []
	for key in ["FL", "FR", "RL", "RR"]:
		var wheel := (load(ASSETS + "f1_2030_v10_wheel_%s.glb" % key) as PackedScene).instantiate() as Node3D
		car.add_child(wheel)
		wheels.append(wheel)
	var world := WorldEnvironment.new()
	var environment := Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color(0.085, 0.10, 0.13)
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	environment.ambient_light_color = Color.WHITE
	environment.ambient_light_energy = 0.65
	world.environment = environment
	car.add_child(world)
	var light := DirectionalLight3D.new()
	light.rotation_degrees = Vector3(-40, -35, 0)
	light.light_energy = 1.7
	car.add_child(light)
	var camera := Camera3D.new()
	car.add_child(camera)
	camera.make_current()
	camera.position = Vector3(-1.6, 0.85, -2.5)
	camera.look_at(Vector3(-0.55, 0.12, -1.46))
	camera.fov = 43.0
	var output := OS.get_cmdline_user_args()
	var directory := output[0] if output.size() > 0 else OS.get_user_data_dir()
	DirAccess.make_dir_recursive_absolute(directory)
	for scenario in ["rest", "bump", "steer", "rear_detail"]:
		for i in range(4):
			var corner := geometry.get_corner(i)
			var compression: float = corner["spring_len"] * corner["resting_ratio"] + (0.055 if scenario == "bump" else 0.0)
			var pose := geometry.solve(i, compression, 0.35 if scenario == "steer" and i < 2 else 0.0, 0.0, 0.0)
			links.update_wheel(i, pose)
			wheels[i].transform = Transform3D(pose["wheel_basis"], pose["hub"])
			# Detail captures expose attachments normally hidden inside the wheel.
			wheels[i].visible = scenario == "rest"
		if scenario == "rear_detail":
			camera.position = Vector3(-1.5, 0.85, 2.5)
			camera.look_at(Vector3(-0.55, 0.12, 1.48))
		await process_frame
		await RenderingServer.frame_post_draw
		var result := root.get_texture().get_image().save_png(directory.path_join("suspension_%s.png" % scenario))
		if result != OK:
			printerr("Capture failed: ", result)
			quit(1)
			return
	if output.size() > 1 and output[1] == "animate":
		root.size = Vector2i(960, 668)
		camera.position = Vector3(-1.6, 0.85, -2.5)
		camera.look_at(Vector3(-0.55, 0.12, -1.46))
		var frames := directory.path_join("frames")
		DirAccess.make_dir_recursive_absolute(frames)
		for frame in range(48):
			var phase := TAU * float(frame) / 48.0
			for i in range(4):
				var corner := geometry.get_corner(i)
				var rest: float = corner["spring_len"] * corner["resting_ratio"]
				var pose := geometry.solve(i, rest + 0.025 * (1.0 - cos(phase)), 0.35 * sin(phase) if i < 2 else 0.0, 0.0, phase)
				links.update_wheel(i, pose)
			await process_frame
			await RenderingServer.frame_post_draw
			root.get_texture().get_image().save_png(frames.path_join("frame_%03d.png" % frame))
	print("[PASS] Visual-only captures: ", directory)
	car.free()
	quit(0)

func hide_baked(node: Node) -> void:
	if node is Node3D and (node.name.begins_with("GEO_CHASSIS_FRONT_SUSPENSION") or node.name.begins_with("GEO_CHASSIS_REAR_SUSPENSION")):
		node.visible = false
	for child in node.get_children():
		hide_baked(child)
