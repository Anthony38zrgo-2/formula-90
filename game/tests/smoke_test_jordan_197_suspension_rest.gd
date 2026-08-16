extends SceneTree

const SCENE := preload("res://scenes/tests/vehicle_track_combinations/jordan_197_handling_test.tscn")

const EXPECTED_RESTING := {"FL": 50.0, "FR": 50.0, "RL": 60.0, "RR": 60.0}
const REST_TOLERANCE_MM := 25.0
const BOTTOM_OUT_MM := 110.0
const WHEEL_NODES := {
	"FL": "WheelFrontLeft",
	"FR": "WheelFrontRight",
	"RL": "WheelRearLeft",
	"RR": "WheelRearRight",
}


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var scene_root := SCENE.instantiate()
	root.add_child(scene_root)
	await process_frame

	var vehicle := scene_root.get_node("Jordan197/VehicleRigidBody") as RigidBody3D
	vehicle.can_sleep = false
	vehicle.sleeping = false

	for _i in 600:
		await physics_frame

	var failures: Array[String] = []
	for wheel_name in EXPECTED_RESTING:
		var wheel := vehicle.get_node(WHEEL_NODES[wheel_name])
		var spring_length: float = wheel.get("spring_length")
		var current_length: float = wheel.get("spring_current_length")
		var compression := (spring_length - current_length) * 1000.0
		var expected: float = EXPECTED_RESTING[wheel_name]
		if absf(compression - expected) > REST_TOLERANCE_MM:
			failures.append(
				"%s rest compression %.1fmm outside %.0f +/- %.0fmm"
				% [wheel_name, compression, expected, REST_TOLERANCE_MM]
			)
		if compression > BOTTOM_OUT_MM:
			failures.append("%s bottomed at rest: %.1fmm" % [wheel_name, compression])

	scene_root.queue_free()
	if failures.is_empty():
		print("PASS: jordan_197 suspension rest compression on La Chutana")
		quit(0)
	else:
		push_error("; ".join(failures))
		quit(1)
