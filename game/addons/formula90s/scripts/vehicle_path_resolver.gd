extends RefCounted
class_name VehiclePathResolver

## Single source of truth for canonical Vehicle discovery.
## Replaces hardcoded NodePath literals scattered in HUD/compositor/tests.

const CANDIDATE_PATHS: Array[NodePath] = [
	NodePath("F194/VehicleRigidBody"),
	NodePath("Jordan197/VehicleRigidBody"),
	NodePath("Jordan191/VehicleRigidBody"),
	NodePath("VehicleController/VehicleRigidBody"),
]

static func find_canonical(world_content: Node) -> Vehicle:
	for path in CANDIDATE_PATHS:
		var found := world_content.get_node_or_null(path) as Vehicle
		if found != null:
			return found
	# Recursive fallback: find any Vehicle in subtree
	return _find_recursive(world_content)

static func find_canonical_path(world_content: Node) -> NodePath:
	for path in CANDIDATE_PATHS:
		if world_content.get_node_or_null(path) != null:
			return path
	var v := find_canonical(world_content)
	if v != null:
		return world_content.get_path_to(v)
	return CANDIDATE_PATHS[0]

static func world_vehicle_path(vehicle: Vehicle, world_content_name: String = "WorldContent") -> NodePath:
	if vehicle == null:
		return NodePath("../../%s/%s" % [world_content_name, CANDIDATE_PATHS[0]])
	var path := vehicle.get_parent().name + "/VehicleRigidBody" if vehicle.get_parent() else String(CANDIDATE_PATHS[0])
	# Use canonical candidate if matches
	for candidate in CANDIDATE_PATHS:
		if String(candidate) == path:
			return NodePath("../../%s/%s" % [world_content_name, candidate])
	return NodePath("../../%s/%s" % [world_content_name, path])

static func _find_recursive(node: Node) -> Vehicle:
	if node is Vehicle:
		return node as Vehicle
	for child in node.get_children():
		var found := _find_recursive(child)
		if found != null:
			return found
	return null
