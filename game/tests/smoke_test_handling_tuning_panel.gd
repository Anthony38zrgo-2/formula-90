extends SceneTree

const VEHICLE_SCENE := preload("res://scenes/vehicles/jordan_191/jordan_191.tscn")
const TUNER_SCRIPT := preload("res://addons/formula90s/scripts/handling_tuning_panel.gd")


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var fixture := Node3D.new()
	fixture.name = "Fixture"
	var controller := VEHICLE_SCENE.instantiate()
	controller.name = "Jordan191"
	fixture.add_child(controller)

	var tuner := Control.new()
	tuner.name = "HandlingTuningPanel"
	tuner.set_script(TUNER_SCRIPT)
	fixture.add_child(tuner)
	root.add_child(fixture)
	await process_frame

	var failures: Array[String] = []
	var vehicle := controller.get_node("VehicleRigidBody") as Vehicle
	if tuner.call("get_tunable_ids").size() != 7:
		failures.append("expected seven allowlisted live controls")
	if tuner.get_node_or_null("LiveTuningPanel") == null:
		failures.append("runtime tuning UI was not built")
	var panel := tuner.get_node_or_null("LiveTuningPanel") as Control
	var key_event := InputEventKey.new()
	key_event.keycode = KEY_F10
	key_event.pressed = true
	Input.parse_input_event(key_event)
	await process_frame
	if panel == null or not panel.visible:
		failures.append("F10 did not open the live tuning UI")

	if controller.is_physics_processing():
		failures.append("controller remains active while editing")
	tuner.call("_apply_value", "front_brake_bias", 0.61)
	if not is_equal_approx(vehicle.front_axle.brake_bias, 0.61):
		failures.append("front brake bias cache was not updated")
	if not is_equal_approx(vehicle.rear_axle.brake_bias, 0.39):
		failures.append("rear brake bias cache was not updated")
	tuner.call("_apply_value", "max_torque", 455.0)
	if not is_equal_approx(vehicle.max_clutch_torque, 455.0 * vehicle.max_clutch_torque_ratio):
		failures.append("clutch torque cache was not updated")

	tuner.call("_restore_baseline")
	tuner.call("set_panel_visible", false)
	if not controller.is_physics_processing():
		failures.append("controller was not restored after closing")

	fixture.queue_free()
	if failures.is_empty():
		print("PASS: handling tuning panel live bindings")
		quit(0)
	else:
		push_error("; ".join(failures))
		quit(1)
