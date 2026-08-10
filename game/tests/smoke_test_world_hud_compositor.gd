extends SceneTree


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var failures := 0
	var compositor_scene := load("res://scenes/runtime/world_hud_compositor.tscn") as PackedScene
	if compositor_scene == null:
		printerr("[FAIL] World/HUD compositor scene could not load.")
		quit(1)
		return

	var compositor := compositor_scene.instantiate()
	compositor.world_scene_path = "res://scenes/tracks/test_field/jordan_handling_test.tscn"
	root.add_child(compositor)
	await process_frame
	await process_frame

	var world_viewport := compositor.get_node_or_null("WorldViewport") as SubViewport
	var world_presenter := compositor.get_node_or_null("WorldPresenter") as TextureRect
	var world_content := compositor.get_node_or_null("WorldViewport/WorldContent")
	var hud_layer := compositor.get_node_or_null("HudLayer") as CanvasLayer
	var debug_hud := compositor.get_node_or_null("HudLayer/DebugHud")
	var wheel_diagnostics := compositor.get_node_or_null("HudLayer/WheelDiagnostics")
	var vehicle := compositor.get_node_or_null("WorldViewport/WorldContent/VehicleController/VehicleRigidBody")
	var driving_aids := compositor.get_node_or_null("WorldViewport/WorldContent/DrivingAids")

	if world_viewport == null or world_viewport.size != Vector2i(640, 360) or world_presenter == null or world_presenter.texture != world_viewport.get_texture():
		printerr("[FAIL] World presentation does not expose the fixed-resolution SubViewport texture.")
		failures += 1
	if world_content == null or vehicle == null or world_viewport.get_camera_3d() == null:
		printerr("[FAIL] 3D world or active camera did not enter the SubViewport.")
		failures += 1
	if hud_layer == null or hud_layer.layer != 1 or debug_hud == null or wheel_diagnostics == null:
		printerr("[FAIL] HUD was not extracted to the composition layer.")
		failures += 1
	if world_content != null and world_content.get_node_or_null("DebugHud") != null:
		printerr("[FAIL] DebugHud remained inside the filtered world viewport.")
		failures += 1
	if debug_hud != null and debug_hud.get_node_or_null(debug_hud.get("vehicle_path")) != vehicle:
		printerr("[FAIL] Extracted DebugHud lost its vehicle NodePath.")
		failures += 1
	if debug_hud != null and debug_hud.get_node_or_null(debug_hud.get("aids_path")) != driving_aids:
		printerr("[FAIL] Extracted DebugHud lost its DrivingAids NodePath.")
		failures += 1
	var minimap: Node = null
	if debug_hud != null:
		minimap = debug_hud.get_node_or_null("Minimap")
	if minimap == null or minimap.get_node_or_null(minimap.get("target_path")) != vehicle:
		printerr("[FAIL] Extracted minimap lost its vehicle NodePath.")
		failures += 1
	if wheel_diagnostics != null and wheel_diagnostics.get_node_or_null(wheel_diagnostics.get("vehicle_path")) != vehicle:
		printerr("[FAIL] Extracted wheel diagnostics lost its vehicle NodePath.")
		failures += 1
	if world_content != null and world_content.get_node_or_null("Track/SourceSkyboxRig") == null:
		printerr("[FAIL] Source skybox was not preserved inside the rendered world.")
		failures += 1

	compositor.queue_free()
	if failures == 0:
		print("[PASS] World renders in a SubViewport while HUD and diagnostics remain in the root canvas.")
	quit(failures)
