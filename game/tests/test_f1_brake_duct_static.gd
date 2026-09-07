extends SceneTree

## Brake-duct regression: the scoop must not spin with the wheel, and the
## intake must face forward on both front corners.
##
## Geometry facts (proven by tools/blender/validate_duct_split.py): only the
## front hub models a disconnected brake-scoop shell (105 verts / 144 tris);
## hub rim shells are symmetric and spin. The rear hub models no scoop, so
## rear DuctStatic nodes stay empty by design.
##
## Runs without native DLLs in an empty Godot project:
##   godot --headless --path <empty> --script test_f1_brake_duct_static.gd \
##     -- --source-root=<absolute game directory>
## Part A parses the canonical tscn (scene wiring). Part B drives the real
## F1WheelVisualController against a mock vehicle (controller behavior).

const SCENE_REL := "scenes/vehicles/f1_2026_2008/f1_2026_2008_rust.tscn"
const WHEELS := ["FrontLeftWheel", "FrontRightWheel", "RearLeftWheel", "RearRightWheel"]
const FRONT_WHEELS := ["FrontLeftWheel", "FrontRightWheel"]
const DUCT_FILES := {
	"FrontLeftWheel": "f1_2026_2008_duct_FL.glb",
	"FrontRightWheel": "f1_2026_2008_duct_FR.glb",
}
const SPIN_FILES := {
	"FrontLeftWheel": "f1_2026_2008_wheel_front_spin.glb",
	"FrontRightWheel": "f1_2026_2008_wheel_front_spin.glb",
	"RearLeftWheel": "f1_2026_2008_wheel_rear_spin.glb",
	"RearRightWheel": "f1_2026_2008_wheel_rear_spin.glb",
}

class TelemetryVehicle extends Node3D:
	var initialized := false
	var spins := [0.0, 0.0, 0.0, 0.0]
	var angle := 0.0

	func _ready() -> void:
		initialized = true

	func get_wheel_anchor_local(index: int) -> Vector3:
		return Vector3.ZERO

	func get_wheel_compressions() -> PackedFloat64Array:
		return PackedFloat64Array([51.625, 51.625, 68.9, 68.9])

	func get_linear_velocity() -> Vector3:
		return Vector3.ZERO

	func get_steer_angle_rad() -> float:
		return angle

	func get_brake_state_snapshot() -> Dictionary:
		var result := {}
		for index in range(4):
			result[["FL", "FR", "RL", "RR"][index]] = {"spin_post_rad_s": spins[index]}
		return result

var _failures: Array[String] = []
var _source_root := ""


func _init() -> void:
	call_deferred("_run")


func _argument(prefix: String, fallback: String = "") -> String:
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with(prefix):
			return argument.trim_prefix(prefix)
	return fallback


func _check(condition: bool, message: String) -> void:
	if not condition:
		_failures.append(message)
		printerr("[FAIL] " + message)


func _advance(controller: Node) -> void:
	controller.call("_physics_process", 1.0 / 60.0)
	for frame in range(60):
		controller.call("_process", 1.0 / 60.0)


func _check_scene_wiring() -> void:
	var tscn := FileAccess.open(_source_root.path_join(SCENE_REL), FileAccess.READ)
	_check(tscn != null, "Cannot open " + SCENE_REL)
	if tscn == null:
		return
	var text := tscn.get_as_text()
	# No Y=PI mirrors may remain: each front corner uses its own L/R scoop file.
	_check(not text.contains("3.14159265"), "Mirrored Y=PI rotation remains in scene")
	for wheel in FRONT_WHEELS:
		_check(text.contains(DUCT_FILES[wheel]), "Missing scoop instance " + DUCT_FILES[wheel])
	for wheel in WHEELS:
		_check(text.contains(SPIN_FILES[wheel]), "Missing spin instance " + SPIN_FILES[wheel])
		var spin_section := text.find('Visual" parent="VehicleRigidBody/' + wheel + '/SteerPivot/CamberPivot/Spinner"')
		_check(spin_section >= 0, "Spin Visual misparented at " + wheel)
	for wheel in FRONT_WHEELS:
		var duct_section := text.find('DuctVisual" parent="VehicleRigidBody/' + wheel + '/SteerPivot/CamberPivot/DuctStatic"')
		_check(duct_section >= 0, "Scoop DuctVisual misparented at " + wheel)
	# Each scoop file is per-corner: FL/FR referenced exactly once; no rear ducts.
	for suffix in ["_FL.glb", "_FR.glb"]:
		_check(text.count("f1_2026_2008_duct" + suffix) == 1, "Scoop file must appear once: *" + suffix)
	_check(not text.contains("duct_RL") and not text.contains("duct_RR"),
		"Rear axle models no scoop; rear duct references must not exist")


func _check_controller_leaves_ducts() -> void:
	var controller_path := _source_root.path_join("scripts/vehicle/f1_wheel_visual_controller.gd")
	var controller_script := load(controller_path) as Script
	_check(controller_script != null and controller_script.can_instantiate(), "Controller cannot load")
	if controller_script == null or not controller_script.can_instantiate():
		return
	var vehicle := TelemetryVehicle.new()
	var controller := Node.new()
	controller.set_script(controller_script)
	controller.set("physics_config_path", _source_root.path_join("data/vehicles/f1_2026_2008/f1_2026_2008_physics.json"))
	vehicle.add_child(controller)
	for wheel in WHEELS:
		var hub := Node3D.new()
		hub.name = wheel
		vehicle.add_child(hub)
		var steer := Node3D.new()
		steer.name = "SteerPivot"
		hub.add_child(steer)
		var camber := Node3D.new()
		camber.name = "CamberPivot"
		steer.add_child(camber)
		var duct := Node3D.new()
		duct.name = "DuctStatic"
		duct.transform = Transform3D(Basis.from_euler(Vector3(0.1, 0.2, 0.3)), Vector3(0.01, 0.02, 0.03))
		camber.add_child(duct)
		var spinner := Node3D.new()
		spinner.name = "Spinner"
		camber.add_child(spinner)
	root.add_child(vehicle)
	controller.set_process(false)
	controller.set_physics_process(false)

	vehicle.spins = [0.0, 0.0, 0.0, 0.0]
	_advance(controller)
	var before: Array = []
	for wheel in WHEELS:
		before.append((vehicle.get_node(wheel + "/SteerPivot/CamberPivot/DuctStatic") as Node3D).transform)
	# Spin hard with steering: spinners must turn, ducts must not move.
	vehicle.spins = [40.0, -40.0, 30.0, -30.0]
	vehicle.angle = 0.2
	_advance(controller)
	for index in range(4):
		var spinner := vehicle.get_node(WHEELS[index] + "/SteerPivot/CamberPivot/Spinner") as Node3D
		_check(absf(spinner.rotation.x + vehicle.spins[index] / 60.0) < 0.0001,
			"Spinner must rotate with signed spin at " + WHEELS[index])
		var duct := vehicle.get_node(WHEELS[index] + "/SteerPivot/CamberPivot/DuctStatic") as Node3D
		_check((duct.transform.origin - (before[index] as Transform3D).origin).length() < 0.0001,
			"Duct moved with wheel spin at " + WHEELS[index])
		_check(absf(duct.rotation.x - (before[index] as Transform3D).basis.get_euler().x) < 0.0001
			and absf(duct.rotation.y - (before[index] as Transform3D).basis.get_euler().y) < 0.0001
			and absf(duct.rotation.z - (before[index] as Transform3D).basis.get_euler().z) < 0.0001,
			"Duct rotated with wheel spin at " + WHEELS[index])
	vehicle.free()


func _run() -> void:
	_source_root = _argument("--source-root=", ProjectSettings.globalize_path("res://"))
	_check_scene_wiring()
	_check_controller_leaves_ducts()
	print("[RESULT] Brake duct static regression: %d failure(s)" % _failures.size())
	quit(0 if _failures.is_empty() else 1)
