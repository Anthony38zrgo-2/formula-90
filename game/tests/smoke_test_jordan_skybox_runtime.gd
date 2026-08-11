extends SceneTree


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var failures := 0
	var test_scene := load("res://scenes/tracks/test_field/jordan_handling_test.tscn") as PackedScene
	if test_scene == null:
		printerr("[FAIL] Jordan handling scene could not load.")
		quit(1)
		return

	var test_root := test_scene.instantiate()
	root.add_child(test_root)
	await process_frame

	var source_rig := test_root.get_node_or_null("Track/SourceSkyboxRig") as Node3D
	var debug_hud := test_root.get_node_or_null("DebugHud")
	var vehicle := test_root.get_node_or_null("VehicleController/VehicleRigidBody")
	var minimap := test_root.get_node_or_null("DebugHud/Minimap") as TrackMinimapController
	if source_rig == null or debug_hud == null or vehicle == null:
		printerr("[FAIL] Jordan scene lost the source skybox, HUD, or vehicle hierarchy.")
		failures += 1
	else:
		if minimap == null or minimap.get_node_or_null(minimap.target_path) != vehicle:
			printerr("[FAIL] Standalone Jordan HUD minimap did not resolve its local vehicle target.")
			failures += 1
		else:
			await process_frame
			var player_before := minimap.get_player_map_position()
			var teleport_transform: Transform3D = vehicle.global_transform
			teleport_transform.origin = Vector3(150.0, teleport_transform.origin.y, -250.0)
			PhysicsServer3D.body_set_state(vehicle.get_rid(), PhysicsServer3D.BODY_STATE_TRANSFORM, teleport_transform)
			await physics_frame
			await physics_frame
			var player_after := minimap.get_player_map_position()
			if player_before.distance_to(player_after) < 1.0:
				printerr("[FAIL] Standalone Jordan HUD minimap player marker did not move with the vehicle.")
				failures += 1

		var camera: Camera3D = root.get_camera_3d()
		if camera == null:
			printerr("[FAIL] Active Camera3D is unavailable.")
			failures += 1
		else:
			var before_y := source_rig.global_position.y
			var target_position := camera.global_position + Vector3(37.0, 11.0, -29.0)
			camera.global_position = target_position
			await process_frame
			await process_frame
			if not is_equal_approx(source_rig.global_position.x, target_position.x) or not is_equal_approx(source_rig.global_position.z, target_position.z):
				printerr("[FAIL] SourceSkyboxRig did not follow the active camera on X/Z.")
				failures += 1
			if not is_equal_approx(source_rig.global_position.y, before_y):
				printerr("[FAIL] SourceSkyboxRig must not follow the active camera height.")
				failures += 1

		var waterfall := source_rig.find_child("WaterfallLeft", true, false) as Sprite3D
		if waterfall == null:
			printerr("[FAIL] SourceSkyboxRig does not expose an animated waterfall sprite.")
			failures += 1
		else:
			var frame_before_animation := waterfall.frame
			await create_timer(0.2).timeout
			if waterfall.frame == frame_before_animation:
				printerr("[FAIL] Source skybox waterfall atlas did not advance.")
				failures += 1

	test_root.queue_free()
	if failures == 0:
		print("[PASS] Jordan handling scene preserves vehicle/HUD while the 3D skybox follows camera X/Z only.")
	quit(failures)
