extends GdUnitTestSuite

const SURFACE_GROUPS_SCRIPT := preload("res://scripts/track/generated_track_surface_groups.gd")


func test_tags_generated_collision_bodies_recursively() -> void:
	var root := auto_free(Node3D.new()) as Node3D
	var nested := Node3D.new()
	root.add_child(nested)

	var road := _body("RoadCollision-colonly", nested)
	var curb := _body("Turn03_Curb-colonly", nested)
	var grass := _body("GrassSafetyFloor-colonly", nested)
	var wall := _body("OuterGuardrail-colonly", nested)
	var unknown := _body("TireBarrier-colonly", nested)

	var tagger := auto_free(SURFACE_GROUPS_SCRIPT.new()) as Node
	tagger._tag_recursive(root)

	assert_bool(road.is_in_group(&"Road")).is_true()
	assert_bool(curb.is_in_group(&"Curb")).is_true()
	assert_bool(grass.is_in_group(&"Grass")).is_true()
	assert_bool(wall.is_in_group(&"Wall")).is_true()
	assert_bool(unknown.is_in_group(&"Road")).is_false()
	assert_bool(unknown.is_in_group(&"Curb")).is_false()
	assert_bool(unknown.is_in_group(&"Grass")).is_false()
	assert_bool(unknown.is_in_group(&"Wall")).is_false()


func test_surface_name_precedence_is_deterministic() -> void:
	var root := auto_free(Node3D.new()) as Node3D
	var ambiguous := _body("Road_Grass_Curb", root)
	var tagger := auto_free(SURFACE_GROUPS_SCRIPT.new()) as Node

	tagger._tag_recursive(root)

	assert_bool(ambiguous.is_in_group(&"Road")).is_true()
	assert_bool(ambiguous.is_in_group(&"Grass")).is_false()
	assert_bool(ambiguous.is_in_group(&"Curb")).is_false()


func _body(body_name: String, parent: Node) -> StaticBody3D:
	var body := StaticBody3D.new()
	body.name = body_name
	parent.add_child(body)
	return body
