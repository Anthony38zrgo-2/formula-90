extends SceneTree

const SCENE_PATH := "res://scenes/runtime/f1_94_la_chutana.tscn"
const EXPECTED_WHEELS := {
	"WheelFrontLeft": Vector3(-0.796250492, 0.143972749, -1.46032548),
	"WheelFrontRight": Vector3(0.796250492, 0.143972749, -1.46032548),
	"WheelRearLeft": Vector3(-0.762306511, 0.186027251, 1.46032548),
	"WheelRearRight": Vector3(0.762306511, 0.186027251, 1.46032548),
}


func _init() -> void:
	call_deferred("_run")


func _fail(message: String, failures: Array[String]) -> void:
	printerr("[FAIL] " + message)
	failures.append(message)


func _run() -> void:
	var failures: Array[String] = []
	var packed := load(SCENE_PATH) as PackedScene
	if packed == null:
		_fail("F1-94 La Chutana scene could not load.", failures)
		quit(1)
		return

	var compositor := packed.instantiate()
	root.add_child(compositor)
	for _frame in 4:
		await process_frame

	var world := compositor.get_node_or_null("WorldViewport/WorldContent")
	var vehicle := compositor.get_node_or_null("WorldViewport/WorldContent/F194/VehicleRigidBody") as RigidBody3D
	var hud := compositor.get_node_or_null("HudLayer/DebugHud")
	var minimap := compositor.get_node_or_null("HudLayer/DebugHud/Minimap")
	var speed_gauge := compositor.get_node_or_null("HudLayer/DebugHud/SpeedGauge")
	var aids := compositor.get_node_or_null("WorldViewport/WorldContent/DrivingAids")

	if world == null or vehicle == null or compositor.get_node_or_null("WorldViewport").get_camera_3d() == null:
		_fail("3D world, F194 vehicle, or active camera is missing from the compositor.", failures)
	if hud == null or minimap == null or speed_gauge == null:
		_fail("HUD, minimap, or speed gauge was not extracted into HudLayer.", failures)
	if hud != null and hud.get_node_or_null(hud.get("vehicle_path")) != vehicle:
		_fail("HUD vehicle_path does not resolve to F194/VehicleRigidBody.", failures)
	if hud != null and hud.get_node_or_null(hud.get("aids_path")) != aids:
		_fail("HUD aids_path does not resolve to DrivingAids.", failures)
	if minimap != null:
		if minimap.get("map_data") == null or not minimap.get("map_data").is_valid_map():
			_fail("La Chutana minimap data is missing or invalid.", failures)
		if minimap.get_node_or_null(minimap.get("target_path")) != vehicle:
			_fail("Minimap target_path does not resolve to F194/VehicleRigidBody.", failures)
	if world != null and world.get_node_or_null("DebugHud") != null:
		_fail("DebugHud remained inside the filtered 3D world.", failures)

	if vehicle != null:
		for wheel_name in EXPECTED_WHEELS:
			var wheel := vehicle.get_node_or_null(wheel_name) as RayCast3D
			if wheel == null:
				_fail("Missing wheel RayCast: " + wheel_name, failures)
				continue
			if wheel.position.distance_to(EXPECTED_WHEELS[wheel_name]) > 0.00001:
				_fail("Wheel RayCast datum mismatch: " + wheel_name, failures)
			if wheel.get_node_or_null("Pivot/Visual") == null:
				_fail("Missing wheel visual: " + wheel_name, failures)

	for _frame in 180:
		await physics_frame
	if vehicle != null:
		var contacts := 0
		for wheel_name in EXPECTED_WHEELS:
			var wheel := vehicle.get_node_or_null(wheel_name) as RayCast3D
			if wheel != null and wheel.is_colliding():
				contacts += 1
		if contacts != 4:
			_fail("Only %d/4 wheel RayCasts contact La Chutana after settling." % contacts, failures)

	if minimap != null and vehicle != null:
		var map_before: Vector2 = minimap.call("get_player_map_position")
		vehicle.freeze = true
		vehicle.global_position.x += 2.0
		await process_frame
		var map_after: Vector2 = minimap.call("get_player_map_position")
		if map_before.distance_to(map_after) < 0.01:
			_fail("Minimap player marker does not react to vehicle movement.", failures)

	compositor.queue_free()
	if failures.is_empty():
		print("[PASS] F1-94 loads on La Chutana with 4/4 contacts, HUD, speed gauge, and live minimap.")
	quit(failures.size())
