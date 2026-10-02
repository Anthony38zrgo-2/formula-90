extends SceneTree

const DEFAULT_SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"
const DEFAULT_VEHICLE_ID := "f1_2030_v10"
const EXPECTED_TCAM_CONFIG := "res://data/cameras/f1_2030_v10_tcam.json"
const EXPECTED_SPAWN := Vector3(-297.652, 0.024, -244.251)
const REQUIRED_GROUPS := [&"Road", &"Curb", &"Grass", &"Gravel", &"Sand", &"Wall", &"Metal"]

func _init() -> void:
	call_deferred("_run")

func _fail(message: String, failures: Array[String]) -> void:
	printerr("[FAIL] " + message)
	failures.append(message)

func _argument(prefix: String, fallback: String) -> String:
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with(prefix):
			return argument.trim_prefix(prefix)
	return fallback

func cycle_camera_with_keyboard() -> void:
	var camera_key_event := InputEventKey.new()
	camera_key_event.physical_keycode = KEY_C
	camera_key_event.pressed = true
	Input.parse_input_event(camera_key_event)
	await process_frame
	await process_frame
	camera_key_event = InputEventKey.new()
	camera_key_event.physical_keycode = KEY_C
	camera_key_event.pressed = false
	Input.parse_input_event(camera_key_event)
	await process_frame

func _run() -> void:
	var failures: Array[String] = []
	var scene_path := _argument("--scene=", DEFAULT_SCENE_PATH)
	var expected_vehicle_id := _argument("--vehicle-id=", DEFAULT_VEHICLE_ID)
	var packed := load(scene_path) as PackedScene
	if packed == null:
		_fail("Fuji runtime scene could not load.", failures)
		quit(1)
		return

	var compositor := packed.instantiate()
	root.add_child(compositor)
	for _frame in 8:
		await process_frame

	var track := compositor.find_child("ActiveTrack", true, false) as Node3D
	var vehicle_root := compositor.find_child("ActiveVehicle", true, false) as Node3D
	var vehicle := compositor.find_child("VehicleRigidBody", true, false) as RigidBody3D
	var race_session := compositor.find_child("RaceSession", true, false)
	var camera_rig := compositor.find_child("CameraRig", true, false) as Node3D
	var camera := compositor.find_child("Camera3D", true, false) as Camera3D
	var tcam_rig := compositor.find_child("CameraRigTCam", true, false) as Node3D
	var minimap := compositor.get_node_or_null("DisplayAspect/DisplayStage/HudLayer/DebugHud/Minimap")
	if track == null or vehicle == null:
		_fail("Fuji track or vehicle was not composed.", failures)
	else:
		var marker := track.find_child("VehicleSpawn", true, false) as Marker3D
		if marker == null or marker.global_position.distance_to(EXPECTED_SPAWN) > 0.05:
			_fail("VehicleSpawn does not match Fuji package metadata.", failures)
	if vehicle_root == null or vehicle_root.scene_file_path.find(expected_vehicle_id) < 0:
		_fail("Expected vehicle '%s' is not the active model." % expected_vehicle_id, failures)
	if camera_rig == null or camera == null or not camera.current:
		_fail("Canonical chase camera is missing or inactive.", failures)
	if tcam_rig == null:
		_fail("Vehicle-specific T-cam rig is missing.", failures)
	elif String(tcam_rig.get("config_path")) != EXPECTED_TCAM_CONFIG:
		_fail("T-cam does not use the F1 2030 V10 configuration.", failures)
	else:
		var tcam_camera := tcam_rig.find_child("Camera3D", true, false) as Camera3D
		if tcam_camera == null:
			_fail("Vehicle-specific T-cam camera is missing.", failures)
		else:
			if race_session == null or not race_session.has_method("toggle_camera"):
				_fail("RaceSession cannot toggle the vehicle cameras.", failures)
			else:
				await cycle_camera_with_keyboard()
			await process_frame
			if not tcam_camera.current or camera.current:
				_fail("T-cam cannot become the active camera.", failures)
			if race_session != null and race_session.has_method("toggle_camera"):
				await cycle_camera_with_keyboard()
			await process_frame
			var driver_controller := vehicle.get_node_or_null("DriverVisualController") if vehicle != null else null
			if driver_controller != null:
				var cockpit_camera_rig := compositor.find_child("CockpitCameraRig", true, false) as Node3D
				var cockpit_camera := cockpit_camera_rig.get_node_or_null("Camera3D") as Camera3D if cockpit_camera_rig != null else null
				if cockpit_camera == null or not cockpit_camera.current or camera.current or tcam_camera.current:
					_fail("C does not cycle from T-cam to the driver's cockpit camera.", failures)
				var head_and_neck := driver_controller.get("driver_head_and_neck") as MeshInstance3D
				if head_and_neck == null or head_and_neck.visible:
					_fail("Cockpit view does not hide the driver's head and neck.", failures)
				if cockpit_camera != null:
					await (driver_controller.get("driver_skeleton") as Skeleton3D).skeleton_updated
					var driver_eye_point := driver_controller.call("get_driver_eye_point") as Node3D
					if driver_eye_point == null:
						_fail("Runtime cockpit camera has no driver eye point.", failures)
					else:
						var cockpit_configuration := cockpit_camera_rig.get("configuration") as CockpitCameraConfiguration
						var rendered_eye_transform: Transform3D = cockpit_camera_rig.call("get_rendered_eye_transform")
						var elevation_direction: Vector3 = cockpit_camera_rig.call("get_viewpoint_elevation_direction")
						var expected_camera_position := rendered_eye_transform.origin + elevation_direction * cockpit_configuration.viewpoint_elevation_meters
						var positional_correction: Vector3 = cockpit_camera_rig.get("positional_correction_world")
						positional_correction *= cockpit_configuration.positional_stabilization_strength
						var lateral_direction := Vector3(vehicle.global_basis.x.x, 0.0, vehicle.global_basis.x.z).normalized()
						if absf(positional_correction.y) > cockpit_configuration.maximum_vertical_correction_meters + 0.000001 or absf(positional_correction.dot(lateral_direction)) > cockpit_configuration.maximum_lateral_correction_meters + 0.000001 or absf(positional_correction.dot(lateral_direction.cross(Vector3.UP))) > 0.000001:
							_fail("Runtime cockpit positional stabilization exceeds its permitted transverse correction.", failures)
						expected_camera_position += positional_correction
						if cockpit_camera.global_position.distance_to(expected_camera_position) > 0.001:
							_fail("Runtime cockpit camera does not follow the elevated driver eye point with its bounded stabilization: " + str(cockpit_camera.global_position.distance_to(expected_camera_position)), failures)
				var preferences_panel := cockpit_camera_rig.get("preferences_panel") as CanvasLayer
				var preference_key := InputEventKey.new()
				preference_key.physical_keycode = KEY_F9
				preference_key.pressed = true
				Input.parse_input_event(preference_key)
				await process_frame
				if preferences_panel == null or not preferences_panel.visible or preferences_panel.get_viewport() != root:
					_fail("F9 does not open cockpit preferences above the root HUD viewport.", failures)
				else:
					var sliders: Dictionary = preferences_panel.get("sliders")
					var longitudinal_slider := sliders["longitudinal_force_response_strength"] as HSlider
					var cockpit_configuration := cockpit_camera_rig.get("configuration") as CockpitCameraConfiguration
					var initial_strength := cockpit_configuration.longitudinal_force_response_strength
					var slider_rectangle := longitudinal_slider.get_global_rect()
					var mouse_event := InputEventMouseButton.new()
					mouse_event.button_index = MOUSE_BUTTON_LEFT
					mouse_event.pressed = true
					mouse_event.position = slider_rectangle.position + Vector2(slider_rectangle.size.x * 0.8, slider_rectangle.size.y * 0.5)
					root.push_input(mouse_event, true)
					await process_frame
					if absf(cockpit_configuration.longitudinal_force_response_strength - 1.6) > 0.15:
						_fail("Root viewport mouse input does not reach cockpit preference sliders: " + str(cockpit_configuration.longitudinal_force_response_strength), failures)
					mouse_event.pressed = false
					root.push_input(mouse_event, true)
					longitudinal_slider.value = initial_strength
				preference_key.pressed = false
				Input.parse_input_event(preference_key)
				await cycle_camera_with_keyboard()
				if preferences_panel != null and preferences_panel.visible:
					_fail("Leaving cockpit does not close motion preferences.", failures)
				if not camera.current or (cockpit_camera != null and cockpit_camera.current) or (head_and_neck != null and not head_and_neck.visible):
					_fail("C does not return from cockpit to chase and restore the complete driver.", failures)
			elif not camera.current or compositor.find_child("CockpitCameraRig", true, false) != null:
				_fail("A vehicle without a driver must retain its two existing cameras.", failures)

	for group in REQUIRED_GROUPS:
		var found := false
		for node in get_nodes_in_group(group):
			if track != null and track.is_ancestor_of(node):
				found = true
				break
		if not found:
			_fail("Missing Fuji collision group: %s" % group, failures)

	if minimap == null or minimap.get("map_data") == null or not minimap.get("map_data").is_valid_map():
		_fail("Fuji minimap data is missing or invalid.", failures)

	for _frame in 180:
		await physics_frame
	if vehicle != null:
		var contacts := 0
		for ray_name in [
			"RayCast_FL_In", "RayCast_FL_Mid", "RayCast_FL_Out",
			"RayCast_FR_In", "RayCast_FR_Mid", "RayCast_FR_Out",
			"RayCast_RL_In", "RayCast_RL_Mid", "RayCast_RL_Out",
			"RayCast_RR_In", "RayCast_RR_Mid", "RayCast_RR_Out",
		]:
			var ray := vehicle.find_child(ray_name, true, false) as RayCast3D
			if ray != null and ray.is_colliding():
				contacts += 1
		if contacts < 4:
			_fail("Only %d/12 wheel rays contact Fuji after settling." % contacts, failures)
		if vehicle.global_position.y < -5.0:
			_fail("Vehicle fell through Fuji collision.", failures)
		if camera_rig != null:
			var vertical_offset := camera_rig.global_position.y - vehicle.global_position.y
			if vertical_offset < 2.0 or vertical_offset > 7.0:
				_fail("Chase camera vertical tracking is outside its valid range: %.2f m." % vertical_offset, failures)
			var vehicle_forward := -vehicle.global_transform.basis.z.normalized()
			var camera_delta := camera_rig.global_position - vehicle.global_position
			if camera_delta.dot(vehicle_forward) >= -2.0:
				_fail("Chase camera is not positioned behind the active vehicle.", failures)
			var lifted_position := vehicle.global_position + Vector3(0.0, 10.0, 0.0)
			vehicle.reset_vehicle(lifted_position, vehicle.global_rotation.y)
			for _camera_frame in 3:
				await process_frame
			vertical_offset = camera_rig.global_position.y - vehicle.global_position.y
			if vertical_offset < 2.0 or vertical_offset > 7.0:
				_fail("Chase camera did not follow a 10 m elevation change: %.2f m." % vertical_offset, failures)

	if vehicle != null and vehicle.has_method("get_fuel_state_snapshot"):
		var fuel_state: Dictionary = vehicle.call("get_fuel_state_snapshot")
		var remaining_kg := float(fuel_state.get("remaining_kg", -1.0))
		if remaining_kg <= 0.0 or remaining_kg > 7.6 or float(fuel_state.get("capacity_kg", 0.0)) != 110.0:
			_fail("Fuel state is not the configured three-lap Fuji load: %s" % [fuel_state], failures)
	else:
		_fail("Vehicle does not expose the fuel state snapshot.", failures)

	var lap_timing = race_session.get("lap_timing") if race_session != null else null
	if lap_timing == null:
		_fail("RaceSession lap timing authority is missing.", failures)
	elif not bool(lap_timing.get("is_configured")):
		_fail("Lap timing is not configured from Fuji start/finish metadata.", failures)

	if compositor.get_node_or_null("DisplayAspect/DisplayStage/HudLayer/DebugHud/LapTimingPanel") == null:
		_fail("Lap timing HUD panel is missing.", failures)
	if compositor.get_node_or_null("DisplayAspect/DisplayStage/HudLayer/DebugHud/EngineTemperaturePanel") == null:
		_fail("Engine HUD panel is missing.", failures)

	compositor.queue_free()
	if failures.is_empty():
		print("[PASS] %s loads on Fuji 76-77 with spawn, surfaces, camera, HUD, fuel and lap timing." % expected_vehicle_id)
	quit(failures.size())
