extends SceneTree

var failures: Array[String] = []
var evidence: Dictionary = {}
var physical_keyboard := false
var keyboard_stage_path := ""

func _initialize() -> void:
	for argument in OS.get_cmdline_user_args():
		if argument == "--physical-keyboard":
			physical_keyboard = true
		if argument.begins_with("--keyboard-stage-path="):
			keyboard_stage_path = argument.trim_prefix("--keyboard-stage-path=")
	if physical_keyboard:
		DisplayServer.window_set_title("Formula90s Keyboard Verification")
	call_deferred("verify_runtime_keyboard_controls")

func send_keyboard_action(action_name: String, pressed: bool) -> bool:
	for registered_event in InputMap.action_get_events(action_name):
		if registered_event is InputEventKey:
			var keyboard_event := registered_event.duplicate() as InputEventKey
			keyboard_event.pressed = pressed
			keyboard_event.echo = false
			if physical_keyboard:
				var stage_file := FileAccess.open(keyboard_stage_path, FileAccess.WRITE)
				if stage_file == null:
					failures.append("Cannot write keyboard verification stage")
					return false
				stage_file.store_string(JSON.stringify({"action": action_name, "pressed": pressed, "physical_keycode": keyboard_event.physical_keycode}))
				stage_file.close()
				var deadline := Time.get_ticks_msec() + 3000
				while Input.is_action_pressed(action_name) != pressed and Time.get_ticks_msec() < deadline:
					await create_timer(0.03).timeout
				if Input.is_action_pressed(action_name) != pressed:
					failures.append("Operating system keyboard event was not received: " + action_name)
					return false
				return true
			Input.parse_input_event(keyboard_event)
			Input.flush_buffered_events()
			return true
	failures.append("Action has no keyboard binding: " + action_name)
	return false

func verify_runtime_keyboard_controls() -> void:
	var packed_scene := load("res://scenes/runtime/vehicle_test_session.tscn") as PackedScene
	var runtime := packed_scene.instantiate()
	root.add_child(runtime)
	current_scene = runtime
	await create_timer(0.5).timeout
	var vehicle := runtime.find_child("VehicleRigidBody", true, false)
	var controller := find_input_controller(runtime)
	if vehicle == null or controller == null:
		push_error("Runtime vehicle or input controller is missing")
		quit(1)
		return
	if not controller.get("_has_required_interface"):
		failures.append("Vehicle input controller disabled itself during initialization")
	evidence["maximum_forward_gear"] = controller.maximum_forward_gear
	if controller.maximum_forward_gear != 7:
		failures.append("Active geometric profile must expose seven forward gears")
	for specification in [
		[InputBindings.THROTTLE, "get_throttle_amount", 1.0],
		[InputBindings.BRAKES, "get_brake_amount", 1.0],
		[InputBindings.STEER_LEFT, "get_steering_input", 1.0],
		[InputBindings.STEER_RIGHT, "get_steering_input", -1.0]]:
		var action_name: String = specification[0]
		await send_keyboard_action(action_name, true)
		await create_timer(0.25).timeout
		var input_amount := float(vehicle.call(specification[1]))
		evidence[action_name] = {"action_strength": Input.get_action_strength(action_name), "vehicle_input": input_amount}
		if Input.get_action_strength(action_name) < 0.9 or input_amount * float(specification[2]) < 0.1:
			failures.append("Keyboard input did not reach the native vehicle: " + action_name)
		await send_keyboard_action(action_name, false)
		await create_timer(0.3).timeout
	var initial_gear := int(vehicle.call("get_current_gear"))
	await send_keyboard_action(InputBindings.SHIFT_UP, true)
	await create_timer(0.12).timeout
	await send_keyboard_action(InputBindings.SHIFT_UP, false)
	await create_timer(0.5).timeout
	var shifted_gear := int(vehicle.call("get_current_gear"))
	evidence["shift_up"] = {"before": initial_gear, "after": shifted_gear}
	if shifted_gear != initial_gear + 1:
		failures.append("Keyboard shift up did not change the native gear")
	await send_keyboard_action(InputBindings.SHIFT_DOWN, true)
	await create_timer(0.12).timeout
	await send_keyboard_action(InputBindings.SHIFT_DOWN, false)
	await create_timer(0.5).timeout
	evidence["shift_down"] = {"before": shifted_gear, "after": vehicle.call("get_current_gear")}
	if int(vehicle.call("get_current_gear")) != initial_gear:
		failures.append("Keyboard shift down did not restore the native gear")
	evidence["failures"] = failures
	evidence["event_source"] = "Windows keyboard input" if physical_keyboard else "Godot keyboard input events"
	print("KEYBOARD_RUNTIME_EVIDENCE " + JSON.stringify(evidence))
	for failure in failures:
		push_error(failure)
	if failures.is_empty():
		print("VEHICLE_RUNTIME_KEYBOARD_CONTROLS_PASS")
	quit(0 if failures.is_empty() else 1)

func find_input_controller(node: Node) -> Node:
	if node is VehicleRustInputController:
		return node
	for child in node.get_children():
		var controller := find_input_controller(child)
		if controller != null:
			return controller
	return null
