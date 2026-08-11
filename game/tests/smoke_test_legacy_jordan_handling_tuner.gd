extends SceneTree

const SCENE := preload("res://scenes/tracks/test_field/jordan_handling_test.tscn")


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var scene_root := SCENE.instantiate()
	root.add_child(scene_root)
	await process_frame

	var tuner := scene_root.get_node_or_null("DebugHud/HandlingTuningPanel") as Control
	var panel := tuner.get_node_or_null("LiveTuningPanel") as Control if tuner != null else null
	if tuner == null or panel == null or tuner.get("_vehicle") == null:
		printerr("[FAIL] Legacy Jordan launcher route did not bind the handling tuner.")
		quit(1)
		return

	var key_event := InputEventKey.new()
	key_event.physical_keycode = KEY_F10
	key_event.pressed = true
	Input.parse_input_event(key_event)
	await process_frame
	if not panel.visible:
		printerr("[FAIL] F10 did not open the handling tuner in the legacy Jordan launcher route.")
		quit(1)
		return

	scene_root.queue_free()
	print("[PASS] F10 opens the handling tuner in the legacy Jordan launcher route.")
	quit(0)
