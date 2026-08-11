extends GdUnitTestSuite

const TRACK_SCENE := "res://scenes/tracks/test_field/la_chutana_generated.tscn"
const EXPECTED_COLLISION_SHAPES := 15


func test_physical_runtime_preserves_all_collision_shapes(_timeout := 10000) -> void:
	var runner := scene_runner(TRACK_SCENE)
	var scene := runner.scene()
	var physical_root := scene.get_node_or_null("GeneratedTrack")

	assert_object(physical_root).is_not_null()
	var bodies := _descendants_of_type(physical_root, StaticBody3D)
	var shapes := _descendants_of_type(physical_root, CollisionShape3D)
	assert_int(bodies.size()).is_equal(EXPECTED_COLLISION_SHAPES)
	assert_int(shapes.size()).is_equal(EXPECTED_COLLISION_SHAPES)
	for shape_node in shapes:
		var collision_shape := shape_node as CollisionShape3D
		assert_object(collision_shape.shape).is_not_null()


func test_vegetation_runtime_is_visual_only(_timeout := 10000) -> void:
	var runner := scene_runner(TRACK_SCENE)
	var vegetation_root := runner.scene().get_node_or_null("GeneratedVegetation")

	assert_object(vegetation_root).is_not_null()
	assert_int(_descendants_of_type(vegetation_root, MeshInstance3D).size()).is_greater(0)
	assert_int(_descendants_of_type(vegetation_root, CollisionObject3D).size()).is_equal(0)
	assert_int(_descendants_of_type(vegetation_root, CollisionShape3D).size()).is_equal(0)


func test_track_environment_keeps_glow_disabled(_timeout := 10000) -> void:
	var runner := scene_runner(TRACK_SCENE)
	var world_environment := runner.scene().get_node_or_null("WorldEnvironment") as WorldEnvironment

	assert_object(world_environment).is_not_null()
	assert_object(world_environment.environment).is_not_null()
	assert_bool(world_environment.environment.glow_enabled).is_false()


func _descendants_of_type(root: Node, expected_type: Variant) -> Array[Node]:
	var matches: Array[Node] = []
	if root == null:
		return matches
	for child in root.get_children():
		if is_instance_of(child, expected_type):
			matches.append(child)
		matches.append_array(_descendants_of_type(child, expected_type))
	return matches
