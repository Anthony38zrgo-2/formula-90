extends SceneTree

const BODY := "res://assets/models/vehicles/f1_2026_b/f1_2026_b_body.glb"
const WHEELS := [
	"res://assets/models/vehicles/f1_2026_b/wheel_fl.glb",
	"res://assets/models/vehicles/f1_2026_b/wheel_fr.glb",
	"res://assets/models/vehicles/f1_2026_b/wheel_rl.glb",
	"res://assets/models/vehicles/f1_2026_b/wheel_rr.glb",
]

var _viewport: SubViewport


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	_build_scene(false)
	await _render()
	_await_frame()
	root.get_texture().get_image().save_png("user://audit_orientation_identity.png")

	for child in root.get_children():
		child.queue_free()
	await process_frame
	_build_scene(true)
	await _render()
	_await_frame()
	root.get_texture().get_image().save_png("user://audit_orientation_rotx90.png")
	print("SAVED")
	quit(0)


func _build_scene(rot_x: bool) -> void:
	_viewport = SubViewport.new()
	_viewport.size = Vector2i(640, 360)
	_viewport.own_world_3d = true
	root.add_child(_viewport)
	var cam := Camera3D.new()
	cam.current = true
	cam.position = Vector3(4.2, 2.6, 5.5)
	cam.look_at(Vector3(0, -0.2, 0))
	_viewport.add_child(cam)
	var body := (load(BODY) as PackedScene).instantiate()
	if rot_x:
		(body as Node3D).rotate_object_local(Vector3.RIGHT, PI / 2.0)
	_viewport.add_child(body)
	var positions := [
		Vector3(-0.784, 0.0, -1.62),
		Vector3(0.789, 0.0, -1.62),
		Vector3(-0.739, 0.0, 1.62),
		Vector3(0.734, 0.0, 1.62),
	]
	for i in WHEELS.size():
		var w := (load(WHEELS[i]) as PackedScene).instantiate() as Node3D
		if rot_x:
			w.rotate_object_local(Vector3.RIGHT, PI / 2.0)
		w.position = positions[i]
		_viewport.add_child(w)


func _render() -> void:
	await RenderingServer.frame_post_draw
	await RenderingServer.frame_post_draw


func _await_frame() -> void:
	await process_frame
