extends SceneTree

## Runs without the game's native DLLs: use an empty Godot project and pass
## --source-root=<absolute game directory>. Optional --controller=<script>
## permits replaying the regression against the pre-fix backup.
const WHEELS := ["FrontLeftWheel", "FrontRightWheel", "RearLeftWheel", "RearRightWheel"]
const CENTERS := [Vector3(-0.854, 0, -1.75), Vector3(0.854, 0, -1.75), Vector3(-0.795, 0, 1.75), Vector3(0.795, 0, 1.75)]

class TelemetryVehicle extends Node3D:
	var initialized := false
	var zero_anchors := false
	var throttle_amount := 0.0
	var brake_amount := 0.0
	var angle := 0.0
	var compressions := PackedFloat64Array([51.625, 51.625, 68.9, 68.9])
	var spins := [0.0, 0.0, 0.0, 0.0]

	func _ready() -> void:
		initialized = true

	func get_wheel_anchor_local(index: int) -> Vector3:
		# Native anchors are zero before the parent's NOTIFICATION_READY.
		if not initialized or zero_anchors:
			return Vector3.ZERO
		return CENTERS[index] + Vector3.UP * (0.243375 if index < 2 else 0.1961)

	func get_wheel_compressions() -> PackedFloat64Array:
		return compressions

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
	# Settle render smoothing to the supplied telemetry without changing it.
	for frame in range(60):
		controller.call("_process", 1.0 / 60.0)

func _check_positions(vehicle: Node3D, expected: Array, label: String) -> void:
	for index in range(4):
		var hub := vehicle.get_node(WHEELS[index]) as Node3D
		_check(hub.position.distance_to(expected[index]) < 0.0001,
			"%s %s: expected %s, got %s" % [label, WHEELS[index], expected[index], hub.position])

func _run() -> void:
	_source_root = _argument("--source-root=", ProjectSettings.globalize_path("res://"))
	var controller_path := _argument("--controller=", _source_root.path_join("scripts/vehicle/f1_wheel_visual_controller.gd"))
	var controller_script := load(controller_path) as Script
	if controller_script == null or not controller_script.can_instantiate():
		printerr("[FAIL] Controller cannot load: " + controller_path)
		quit(1)
		return

	var vehicle := TelemetryVehicle.new()
	var controller := Node.new()
	controller.set_script(controller_script)
	# Keep this shared-controller fixture independent of vehicle JSON changes.
	controller.set("physics_config_path", "")
	controller.set("front_spring_length", 0.295)
	controller.set("front_resting_ratio", 0.175)
	controller.set("rear_spring_length", 0.265)
	controller.set("rear_resting_ratio", 0.26)
	vehicle.add_child(controller)
	for index in range(4):
		var hub := Node3D.new()
		hub.name = WHEELS[index]
		hub.position = CENTERS[index]
		vehicle.add_child(hub)
		var parent_node := hub
		for pivot_name in ["SteerPivot", "CamberPivot", "Spinner"]:
			var pivot := Node3D.new()
			pivot.name = pivot_name
			parent_node.add_child(pivot)
			parent_node = pivot
	root.add_child(vehicle)
	controller.set_process(false)
	controller.set_physics_process(false)
	_advance(controller)
	_check_positions(vehicle, CENTERS, "Static ride height after child-before-parent ready")

	# 20 mm bump at FL must raise just that wheel by 0.020 m.
	vehicle.compressions[0] += 20.0
	_advance(controller)
	var bumped := CENTERS.duplicate()
	bumped[0] += Vector3.UP * 0.020
	_check_positions(vehicle, bumped, "Independent FL compression in mm")

	vehicle.compressions = PackedFloat64Array([0, 0, 0, 0])
	_advance(controller)
	var drooped := CENTERS.duplicate()
	for index in range(4):
		drooped[index] -= Vector3.UP * (0.051625 if index < 2 else 0.0689)
	_check_positions(vehicle, drooped, "Full droop")

	vehicle.compressions = PackedFloat64Array([NAN, INF])
	_advance(controller)
	_check_positions(vehicle, CENTERS, "Missing or invalid compression uses static fallback")

	vehicle.compressions = PackedFloat64Array([51.625, 51.625, 68.9, 68.9])
	vehicle.spins = [10.0, -10.0, 0.0, 20.0]
	vehicle.angle = 0.2
	_advance(controller)
	_check_positions(vehicle, CENTERS, "Steering and rotation preserve centers")
	for index in range(4):
		var spinner := vehicle.get_node(WHEELS[index] + "/SteerPivot/CamberPivot/Spinner") as Node3D
		_check(absf(spinner.rotation.x + vehicle.spins[index] / 60.0) < 0.0001, "Signed spin at " + WHEELS[index])

	# A ready vehicle with no valid native anchors must retain the scene layout.
	vehicle.zero_anchors = true
	var late_controller := Node.new()
	late_controller.set_script(controller_script)
	late_controller.set("physics_config_path", controller.get("physics_config_path"))
	for property in ["front_spring_length", "front_resting_ratio", "rear_spring_length", "rear_resting_ratio"]:
		late_controller.set(property, controller.get(property))
	vehicle.add_child(late_controller)
	late_controller.set_process(false)
	late_controller.set_physics_process(false)
	_advance(late_controller)
	_check_positions(vehicle, CENTERS, "Zero native anchors keep scene fallback")
	late_controller.free()

	print("[RESULT] Wheel position/rotation regression: %d failure(s)" % _failures.size())
	vehicle.free()
	quit(0 if _failures.is_empty() else 1)
