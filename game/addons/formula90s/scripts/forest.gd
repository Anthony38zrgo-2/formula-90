extends Node3D

@export var track_path: NodePath
@export var trunk_mesh_path: String = "res://resources/environment/tree_trunk_mesh.tres"
@export var canopy_mesh_path: String = "res://resources/environment/tree_canopy_mesh.tres"
@export var trunk_material_path: String = "res://resources/environment/tree_trunk_material.tres"
@export var leaves_material_path: String = "res://resources/environment/tree_leaves_material.tres"
@export var leaves_dark_material_path: String = "res://resources/environment/tree_leaves_material_dark.tres"
@export var sample_step: float = 6.0
@export var min_dist: float = 15.0
@export var max_dist: float = 25.0
@export var dist_step: float = 2.5
@export var tree_spacing: float = 4.0

var _rng = RandomNumberGenerator.new()
var _placed = []

func _ready():
	_rng.randomize()
	var path_node = get_node(track_path) as Path3D
	if not path_node or not path_node.curve:
		return

	var curve = path_node.curve
	var length = curve.get_baked_length()
	if length <= 0.0:
		return

	var trunk_mesh = load(trunk_mesh_path)
	var canopy_mesh = load(canopy_mesh_path)
	var trunk_mat = load(trunk_material_path)
	var leaves_mat = load(leaves_material_path)
	var leaves_dark_mat = load(leaves_dark_material_path)

	var samples = int(length / sample_step)
	for i in range(samples):
		var offset = i * sample_step
		if offset < 10.0 or offset > length - 10.0:
			continue

		var pos = curve.sample_baked(offset)
		var next_off = minf(offset + 1.0, length)
		var next_pos = curve.sample_baked(next_off)
		var tangent = (next_pos - pos).normalized()

		var left_normal = Vector3(-tangent.z, 0, tangent.x).normalized()
		var right_normal = Vector3(tangent.z, 0, -tangent.x).normalized()

		var d = min_dist
		while d <= max_dist:
			var prob = clamp((d - min_dist) / (max_dist - min_dist) * 0.55 + 0.05, 0.05, 0.6)

			if _rng.randf() < prob:
				_try_place(pos + left_normal * d, trunk_mesh, canopy_mesh, trunk_mat, leaves_mat, leaves_dark_mat)
			if _rng.randf() < prob:
				_try_place(pos + right_normal * d, trunk_mesh, canopy_mesh, trunk_mat, leaves_mat, leaves_dark_mat)

			d += dist_step

func _try_place(world_pos: Vector3, trunk_mesh, canopy_mesh, trunk_mat, leaves_mat, leaves_dark_mat):
	for existing in _placed:
		if world_pos.distance_to(existing) < tree_spacing:
			return

	_placed.append(world_pos)

	var scale_val = _rng.randf_range(0.8, 1.3)
	var use_dark = _rng.randf() > 0.5

	var tree = Node3D.new()
	tree.position = world_pos
	tree.rotation.y = _rng.randf_range(0.0, PI * 2.0)
	tree.scale = Vector3(scale_val, scale_val, scale_val)
	add_child(tree)

	var trunk = MeshInstance3D.new()
	trunk.mesh = trunk_mesh
	trunk.material_override = trunk_mat
	trunk.position = Vector3(0, 1.25, 0)
	tree.add_child(trunk)

	var canopy = MeshInstance3D.new()
	canopy.mesh = canopy_mesh
	canopy.material_override = leaves_dark_mat if use_dark else leaves_mat
	canopy.position = Vector3(0, 3.0, 0)
	tree.add_child(canopy)
