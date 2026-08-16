extends SceneTree

const SCENE_PATH := "res://scenes/vehicles/f1_94/f1_94_rust.tscn"

func _init() -> void:
	call_deferred("_run_test")

func _fail(msg: String, failures: Array[String]) -> void:
	printerr("[FAIL] " + msg)
	failures.append(msg)

func _run_test() -> void:
	var failures: Array[String] = []
	print("=== Running F1-94 Rust Physics Integration Test ===")
	
	var packed = load(SCENE_PATH) as PackedScene
	if packed == null:
		_fail("Could not load f1_94_rust.tscn", failures)
		quit(1)
		return
	
	var car = packed.instantiate()
	root.add_child(car)
	
	var vehicle = car.get_node_or_null("VehicleRigidBody")
	if vehicle == null:
		_fail("VehicleRigidBody node is missing", failures)
		quit(1)
		return
	
	var audio = car.get_node_or_null("VehicleAudio")
	if audio == null:
		_fail("VehicleAudio node missing on F194Rust", failures)
	
	var fl_wheel = car.get_node_or_null("VehicleRigidBody/FrontLeftWheel")
	var fr_wheel = car.get_node_or_null("VehicleRigidBody/FrontRightWheel")
	var rl_wheel = car.get_node_or_null("VehicleRigidBody/RearLeftWheel")
	var rr_wheel = car.get_node_or_null("VehicleRigidBody/RearRightWheel")
	
	if fl_wheel == null or fr_wheel == null or rl_wheel == null or rr_wheel == null:
		_fail("One or more visual wheel nodes missing", failures)
	
	# Create flat ground plane
	var ground = StaticBody3D.new()
	ground.name = "TestGround"
	var col_shape = CollisionShape3D.new()
	var box = BoxShape3D.new()
	box.size = Vector3(100.0, 1.0, 500.0)
	col_shape.shape = box
	col_shape.position = Vector3(0, -0.5, 0)
	ground.add_child(col_shape)
	root.add_child(ground)
	
	car.position = Vector3(0, 0.40, 0)
	vehicle.reset_vehicle(Vector3(0, 0.40, 0), 0.0)
	vehicle.enable_player_input = false
	
	# Simulate 120 frames of full throttle
	for frame in range(120):
		vehicle.throttle_amount = 1.0
		await physics_frame
	
	var spd_kmh = vehicle.get_speed_kmh()
	var spd_ms = vehicle.get_speed()
	var rpm = vehicle.get_motor_rpm()
	var gear = vehicle.get_current_gear()
	print("[PASS] 120 physics frames executed successfully.")
	print("Final State: Speed=%.1f km/h (%.1f m/s), RPM=%.0f, Gear=%d" % [spd_kmh, spd_ms, rpm, gear])
	var comp = vehicle.get_wheel_compressions()
	print("Compressions: FL=%.1fmm, FR=%.1fmm, RL=%.1fmm, RR=%.1fmm" % [comp[0], comp[1], comp[2], comp[3]])
	
	if spd_kmh < 15.0:
		_fail("Vehicle did not accelerate on full throttle (Speed=%.1f km/h)" % spd_kmh, failures)
	if rpm < 5000.0:
		_fail("Engine RPM did not increase (RPM=%.0f)" % rpm, failures)
	
	if failures.size() == 0:
		print("=== F1-94 Rust Physics Integration Test: PASSED ===")
		quit(0)
	else:
		printerr("=== F1-94 Rust Physics Integration Test: FAILED (%d errors) ===" % failures.size())
		quit(1)
