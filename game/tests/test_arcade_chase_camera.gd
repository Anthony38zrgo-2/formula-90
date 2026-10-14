extends SceneTree

class MovingVehicle:
	extends Node3D
	var speed := 100.0
	var linear_velocity := Vector3(0, 0, -100)
	var true_steering_amount := 0.0
	var elapsed_frames := 0
	func _process(delta: float) -> void:
		elapsed_frames += 1
		if elapsed_frames == 101:
			position.y += 1.0
		if elapsed_frames >= 130 and elapsed_frames < 230:
			rotation.y += 0.01
			true_steering_amount = 0.25
		else:
			true_steering_amount = 0.0
		if elapsed_frames == 230:
			position += Vector3(500, 2, 0)
		if elapsed_frames >= 250 and elapsed_frames < 300:
			speed = maxf(0.0, speed - 2.0)
		if elapsed_frames >= 300 and elapsed_frames < 350:
			speed = minf(100.0, speed + 2.0)
		linear_velocity = -basis.z * speed
		position += linear_velocity * delta

var vehicle: MovingVehicle
var camera_rig: Node3D
var checked_height := false
var checked_reset := false
var minimum_alignment := 1.0

func _initialize() -> void:
	call_deferred("prepare_case")

func prepare_case() -> void:
	vehicle = MovingVehicle.new()
	vehicle.name = "MovingVehicle"
	vehicle.process_priority = -20
	root.add_child(vehicle)
	camera_rig = load("res://scenes/runtime/arcade_chase_camera_rig.tscn").instantiate()
	camera_rig.car_path = NodePath("../MovingVehicle")
	root.add_child(camera_rig)

func _process(_delta: float) -> bool:
	if camera_rig == null:
		return false
	var frame := vehicle.elapsed_frames
	if frame == 101:
		var height_gap := vehicle.global_position.y + float(camera_rig.height) - camera_rig.global_position.y
		if height_gap <= 0.01 or height_gap >= 1.0:
			return fail_case("Vertical smoothing did not retain an intermediate height")
		checked_height = true
	if frame == 230:
		var desired := vehicle.global_position + vehicle.global_basis.z * float(camera_rig.distance)
		desired.y = vehicle.global_position.y + float(camera_rig.height)
		if camera_rig.global_position.distance_to(desired) > 0.01:
			return fail_case("Teleport did not reset the camera immediately")
		checked_reset = true
	if frame > 60:
		minimum_alignment = minf(minimum_alignment, (-camera_rig.global_basis.z).dot(-vehicle.global_basis.z))
		if minimum_alignment < 0.5:
			return fail_case("Camera reversed during forward movement")
	if frame >= 380:
		if not checked_height or not checked_reset:
			return fail_case("Height and reset checks did not execute")
		print("ARCADE_CHASE_CAMERA_PASS: high speed, turn, height, teleport, braking and acceleration; minimum alignment=", minimum_alignment)
		quit(0)
	return false

func fail_case(message: String) -> bool:
	push_error(message)
	quit(1)
	return false
