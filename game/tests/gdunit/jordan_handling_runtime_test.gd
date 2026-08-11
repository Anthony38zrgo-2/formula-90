extends GdUnitTestSuite

const VEHICLE_SCENE := "res://scenes/vehicles/jordan_191/jordan_191_phase_b.tscn"
const VEHICLE_PATH := NodePath("VehicleRigidBody")
const WHEEL_NAMES := [&"WheelFrontLeft", &"WheelFrontRight", &"WheelRearLeft", &"WheelRearRight"]


func test_phase_b_preserves_vehicle_collision_and_wheel_contract(_timeout := 10000) -> void:
	var runner := scene_runner(VEHICLE_SCENE)
	var controller := runner.scene()
	var vehicle := controller.get_node_or_null(VEHICLE_PATH) as RigidBody3D

	assert_object(vehicle).is_not_null()
	assert_object(controller.get("vehicle_node")).is_instanceof(RigidBody3D)
	var chassis_shapes := _direct_children_of_type(vehicle, CollisionShape3D)
	assert_int(chassis_shapes.size()).is_equal(3)
	for shape_node in chassis_shapes:
		assert_object((shape_node as CollisionShape3D).shape).is_not_null()
	for wheel_name in WHEEL_NAMES:
		var wheel := vehicle.get_node_or_null(NodePath(wheel_name))
		assert_object(wheel).is_instanceof(RayCast3D)
		assert_bool((wheel as RayCast3D).enabled).is_true()


func test_phase_b_wheel_layout_has_positive_track_and_wheelbase(_timeout := 10000) -> void:
	var runner := scene_runner(VEHICLE_SCENE)
	var vehicle := runner.scene().get_node(VEHICLE_PATH) as RigidBody3D
	var front_left := vehicle.get_node("WheelFrontLeft") as RayCast3D
	var front_right := vehicle.get_node("WheelFrontRight") as RayCast3D
	var rear_left := vehicle.get_node("WheelRearLeft") as RayCast3D
	var rear_right := vehicle.get_node("WheelRearRight") as RayCast3D

	var front_track := absf(front_right.position.x - front_left.position.x)
	var rear_track := absf(rear_right.position.x - rear_left.position.x)
	var left_wheelbase := absf(rear_left.position.z - front_left.position.z)
	var right_wheelbase := absf(rear_right.position.z - front_right.position.z)
	assert_float(front_track).is_greater(1.0)
	assert_float(rear_track).is_greater(1.0)
	assert_float(left_wheelbase).is_greater(2.0)
	assert_float(right_wheelbase).is_equal_approx(left_wheelbase, 0.001)


func test_phase_b_keeps_canonical_handling_setup(_timeout := 10000) -> void:
	var runner := scene_runner(VEHICLE_SCENE)
	var vehicle := runner.scene().get_node(VEHICLE_PATH) as RigidBody3D

	assert_float(vehicle.get("front_brake_bias")).is_equal_approx(0.57, 0.0001)
	assert_float(vehicle.get("max_torque")).is_equal_approx(340.0, 0.0001)
	assert_float(vehicle.get("vehicle_mass")).is_equal_approx(505.0, 0.0001)
	assert_float(vehicle.get("front_weight_distribution")).is_equal_approx(0.45, 0.0001)
	assert_float(vehicle.get("front_spring_length")).is_equal_approx(0.15, 0.0001)
	assert_float(vehicle.get("rear_spring_length")).is_equal_approx(0.15, 0.0001)
	assert_float(vehicle.get("front_tire_radius")).is_equal_approx(0.3473, 0.0001)
	assert_float(vehicle.get("rear_tire_radius")).is_equal_approx(0.3473, 0.0001)


func _direct_children_of_type(root: Node, expected_type: Variant) -> Array[Node]:
	var matches: Array[Node] = []
	for child in root.get_children():
		if is_instance_of(child, expected_type):
			matches.append(child)
	return matches
