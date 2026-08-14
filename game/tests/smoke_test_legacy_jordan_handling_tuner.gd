extends SceneTree

const SCENE := preload("res://scenes/tests/vehicle_track_combinations/jordan_handling_test.tscn")


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var scene_root := SCENE.instantiate()
	root.add_child(scene_root)
	await process_frame

	var tuner := scene_root.get_node_or_null("DebugHud/HandlingTuningPanel") as Control
	var panel := tuner.get_node_or_null("LiveTuningPanel") as Control if tuner != null else null
	var controller := scene_root.get_node_or_null("VehicleController") as FormulaVehicleController
	var vehicle := scene_root.get_node_or_null("VehicleController/VehicleRigidBody") as Vehicle
	var chassis := scene_root.get_node_or_null("VehicleController/VehicleRigidBody/ChassisVisual")
	var front_wheel_visual := scene_root.get_node_or_null("VehicleController/VehicleRigidBody/WheelFrontLeft/FrontLeftWheel/Visual")
	var rear_wheel_visual := scene_root.get_node_or_null("VehicleController/VehicleRigidBody/WheelRearLeft/RearLeftWheel/Visual")
	if tuner == null or panel == null or tuner.get("_vehicle") == null:
		printerr("[FAIL] La Chutana Jordan route did not bind the handling tuner.")
		quit(1)
		return
	if controller == null or vehicle == null or scene_root.get_node_or_null("Track") == null:
		printerr("[FAIL] La Chutana route is missing FormulaVehicleController, VehicleRigidBody, or Track.")
		quit(1)
		return
	if chassis == null or not chassis.scene_file_path.ends_with("candidate_k3_historical/jordan_191_candidate_chassis.glb"):
		printerr("[FAIL] La Chutana route is not using the K3 historical Jordan 191 chassis.")
		quit(1)
		return
	if front_wheel_visual == null or not front_wheel_visual.scene_file_path.ends_with("candidate_k3_historical/jordan_191_candidate_wheel_front.glb"):
		printerr("[FAIL] La Chutana route is not using the K3 historical front wheel.")
		quit(1)
		return
	if rear_wheel_visual == null or not rear_wheel_visual.scene_file_path.ends_with("candidate_k3_historical/jordan_191_candidate_wheel_rear.glb"):
		printerr("[FAIL] La Chutana route is not using the K3 historical rear wheel.")
		quit(1)
		return
	if not is_equal_approx(vehicle.front_brake_bias, 0.57) or not is_equal_approx(vehicle.max_torque, 340.0):
		printerr("[FAIL] Jordan 191 Phase B did not preserve the former car configuration.")
		quit(1)
		return

	var key_event := InputEventKey.new()
	key_event.physical_keycode = KEY_F10
	key_event.pressed = true
	Input.parse_input_event(key_event)
	await process_frame
	if not panel.visible:
		printerr("[FAIL] F10 did not open the handling tuner in the La Chutana Jordan route.")
		quit(1)
		return

	scene_root.queue_free()
	print("[PASS] La Chutana uses the K3 historical Jordan 191 with FormulaVehicleController/VehicleRigidBody and Phase B configuration.")
	quit(0)
