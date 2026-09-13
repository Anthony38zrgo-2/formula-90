extends SceneTree

## Runtime validation that the F1 2030 chassis -> suspension -> wheel assembly
## is fully animated: the procedural linkage exists per wheel, the baked chassis
## suspension meshes are hidden, and the links move when the physics loads the
## front axle (braking nose dive). Requires the rebuilt native runtime.

const SCENE_PATH := "res://scenes/vehicles/f1_2030_v10/f1_2030_v10_rust.tscn"
const ELEMENT_NAMES := ["LOWER_WISHBONE", "UPPER_WISHBONE", "TRACKROD", "PUSHROD", "DAMPER", "ROCKER", "UPRIGHT"]

var _failures: Array[String] = []

func _init() -> void:
	call_deferred("_run")

func _fail(msg: String) -> void:
	_failures.append(msg)
	printerr("[FAIL] " + msg)

func _check(condition: bool, msg: String) -> void:
	if not condition:
		_fail(msg)

func _run() -> void:
	print("=== F1 2030 Suspension Assembly Runtime ===")
	var packed := load(SCENE_PATH) as PackedScene
	if packed == null:
		_fail("Cannot load " + SCENE_PATH)
		quit(1)
		return
	var car := packed.instantiate()
	root.add_child(car)

	var vehicle := car.get_node_or_null("VehicleRigidBody") as Node3D
	if vehicle == null:
		_fail("VehicleRigidBody missing")
		quit(1)
		return

	var ground := StaticBody3D.new()
	ground.name = "TestGround"
	var col := CollisionShape3D.new()
	var box := BoxShape3D.new()
	box.size = Vector3(100.0, 1.0, 500.0)
	col.shape = box
	col.position = Vector3(0, -0.5, 0)
	ground.add_child(col)
	root.add_child(ground)
	car.position = Vector3(0, 0.40, 0)
	vehicle.enable_player_input = false

	# Settle a few frames so deferred children and native init complete.
	for i in range(6):
		await physics_frame

	var links := vehicle.get_node_or_null("SuspensionLinkVisual") as Node3D
	_check(links != null, "SuspensionLinkVisual node must exist under VehicleRigidBody")
	if links == null:
		vehicle.free()
		quit(1)
		return

	for wheel_index in range(4):
		var key: String = ["FL", "FR", "RL", "RR"][wheel_index]
		var root_node := links.get_node_or_null("Susp_" + key) as Node3D
		_check(root_node != null, "Susp_%s root must exist" % key)
		if root_node == null:
			continue
		for element in ELEMENT_NAMES:
			_check(root_node.get_node_or_null(element) != null,
				"Susp_%s must contain %s" % [key, element])
		# DRIVESHAFT is rear-only: present and visible on the rear, hidden on the front.
		var shaft := root_node.get_node_or_null("DRIVESHAFT") as Node3D
		if wheel_index >= 2:
			_check(shaft != null and shaft.visible, "Susp_%s rear DRIVESHAFT must be visible" % key)
		else:
			_check(shaft == null or not shaft.visible, "Susp_%s front DRIVESHAFT must be hidden" % key)

	# Baked suspension meshes from the chassis GLB must be hidden.
	var chassis := vehicle.get_node_or_null("ChassisVisual") as Node3D
	if chassis != null:
		var baked := _find_first(chassis, "GEO_CHASSIS_FRONT_SUSPENSION")
		_check(baked != null, "baked FRONT_SUSPENSION mesh must exist in the GLB")
		if baked != null:
			_check(not (baked as Node3D).visible, "baked FRONT_SUSPENSION must be hidden")
	else:
		_fail("ChassisVisual node missing")

	# Let the physics run a braking nose dive and confirm the front links move.
	var fl_lower := links.get_node("Susp_FL/LOWER_WISHBONE") as Node3D
	var fl_upright := links.get_node("Susp_FL/UPRIGHT") as MeshInstance3D
	var pos_before: Vector3 = fl_lower.get_child(0).global_position
	var upright_before: Vector3 = fl_upright.global_position

	vehicle.brake_amount = 1.0
	for i in range(30):
		await physics_frame
	await physics_frame

	var pos_after: Vector3 = fl_lower.get_child(0).global_position
	var upright_after: Vector3 = fl_upright.global_position
	var moved := pos_before.distance_to(pos_after)
	var upright_moved := upright_before.distance_to(upright_after)
	_check(moved > 0.001, "FL lower wishbone must animate under braking, moved %.4f m" % moved)
	_check(upright_moved > 0.001, "FL upright must animate under braking, moved %.4f m" % upright_moved)
	print("[INFO] FL lower wishbone moved %.3f mm, upright %.3f mm under braking" % [moved * 1000.0, upright_moved * 1000.0])

	print("[RESULT] Suspension assembly runtime: %d failure(s)" % _failures.size())
	vehicle.free()
	quit(0 if _failures.is_empty() else 1)

func _find_first(node: Node, name_prefix: String) -> Node:
	if node.name.begins_with(name_prefix):
		return node
	for child in node.get_children():
		var found := _find_first(child, name_prefix)
		if found != null:
			return found
	return null