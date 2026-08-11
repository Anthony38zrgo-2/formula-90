extends SceneTree

const DEFAULT_SCENE := "res://scenes/tracks/test_field/jordan_handling_test.tscn"
const VEHICLE_PATH := NodePath("VehicleController/VehicleRigidBody")
const WHEEL_NAMES := [
	"WheelFrontLeft",
	"WheelFrontRight",
	"WheelRearLeft",
	"WheelRearRight",
]
const DEFAULT_PHYSICS_FRAMES := 300
const MAX_ALLOWED_DROP_METERS := 2.0


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var arguments := OS.get_cmdline_user_args()
	var scene_path := DEFAULT_SCENE
	var physics_frames := DEFAULT_PHYSICS_FRAMES
	if not arguments.is_empty():
		scene_path = arguments[0]
	if arguments.size() > 1:
		physics_frames = maxi(1, int(arguments[1]))

	print("PHYSICS_PROBE_PHASE load ", scene_path)
	var packed_scene := load(scene_path) as PackedScene
	if packed_scene == null:
		_fail("Unable to load scene: %s" % scene_path)
		return

	print("PHYSICS_PROBE_PHASE instantiate")
	var scene_root := packed_scene.instantiate()
	root.add_child(scene_root)
	await process_frame

	var vehicle := scene_root.get_node_or_null(VEHICLE_PATH) as RigidBody3D
	if vehicle == null:
		_fail("Missing RigidBody3D at %s" % VEHICLE_PATH)
		return

	var wheels: Array[RayCast3D] = []
	for wheel_name in WHEEL_NAMES:
		var wheel := vehicle.get_node_or_null(wheel_name) as RayCast3D
		if wheel == null:
			_fail("Missing wheel raycast: %s" % wheel_name)
			return
		wheels.append(wheel)

	var initial_y := vehicle.global_position.y
	var minimum_y := initial_y
	var maximum_downward_speed := 0.0
	var frames_with_any_contact := 0
	var frames_with_all_contacts := 0
	var maximum_contacts := 0

	print("PHYSICS_PROBE_PHASE simulate ", physics_frames)
	for _frame_index in range(physics_frames):
		await physics_frame
		minimum_y = minf(minimum_y, vehicle.global_position.y)
		maximum_downward_speed = maxf(maximum_downward_speed, -vehicle.linear_velocity.y)
		var contact_count := _count_wheel_contacts(wheels)
		maximum_contacts = maxi(maximum_contacts, contact_count)
		if contact_count > 0:
			frames_with_any_contact += 1
		if contact_count == wheels.size():
			frames_with_all_contacts += 1

	var final_y := vehicle.global_position.y
	var final_contacts := _count_wheel_contacts(wheels)
	var maximum_drop := initial_y - minimum_y
	var passed := (
		maximum_drop <= MAX_ALLOWED_DROP_METERS
		and minimum_y > -MAX_ALLOWED_DROP_METERS
		and frames_with_any_contact > 0
		and final_contacts > 0
	)
	var result := {
		"scene": scene_path,
		"physics_frames": physics_frames,
		"initial_y": initial_y,
		"minimum_y": minimum_y,
		"final_y": final_y,
		"maximum_drop": maximum_drop,
		"maximum_downward_speed": maximum_downward_speed,
		"maximum_wheel_contacts": maximum_contacts,
		"final_wheel_contacts": final_contacts,
		"frames_with_any_contact": frames_with_any_contact,
		"frames_with_all_contacts": frames_with_all_contacts,
		"passed": passed,
	}
	print("PHYSICS_PROBE_RESULT ", JSON.stringify(result))

	scene_root.queue_free()
	await process_frame
	quit(0 if passed else 2)


func _count_wheel_contacts(wheels: Array[RayCast3D]) -> int:
	var count := 0
	for wheel in wheels:
		if wheel.is_colliding():
			count += 1
	return count


func _fail(message: String) -> void:
	printerr("PHYSICS_PROBE_ERROR ", message)
	quit(2)
