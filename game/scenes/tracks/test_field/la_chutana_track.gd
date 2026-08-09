extends Node3D

## Formula90s handling-development reconstruction of Autodromo La Chutana.
##
## Geometry source:
## - user-provided top-down reference layout
## - public dimensions used as scale anchors:
##   ~2.420 km lap, 7 turns, ~800 m main straight
##
## This is a gameplay/physics reference reconstruction, not survey-grade CAD.
## Elevation is intentionally deferred; the current version stays flat except
## for very mild local banking so handling changes are not confused with ramps.

const TRACK_LENGTH_TARGET_M := 2420.0
const MAIN_STRAIGHT_TARGET_M := 800.0
const TURN_COUNT_REFERENCE := 7

const ROAD_HALF_WIDTH := 6.0
const ROAD_TOP := 0.006
const ROAD_DEPTH := 0.16
const EDGE_LINE_WIDTH := 0.12

# Low crowned positive curb. Maximum rise is only ~22 mm and both entry sides
# are ramped. This deliberately avoids the near-vertical curb geometry that can
# launch a low Formula chassis or bypass GEVP suspension raycasts.
const CURB_WIDTH := 0.58
const CURB_BASE_DEPTH := 0.045
const CURB_PEAK_HEIGHT := 0.022

const HANDLE_SCALE := 0.14

# The supplied map was traced in image pixels. Pixel X follows the long main
# straight; pixel Y is lateral. These two calibration factors make the traced
# polyline approximately 2.420 km total while the main straight is ~800 m.
const REFERENCE_FINISH_PIXEL := Vector2(480.0, 50.0)
const METERS_PER_PIXEL_X := 1.1594202898550725
const METERS_PER_PIXEL_Y := 1.0844612924722714

const REFERENCE_TRACE_PIXELS: Array[Vector2] = [
	Vector2(480, 50),  # start / finish
	Vector2(650, 52),
	Vector2(760, 60),
	Vector2(835, 90),
	Vector2(860, 135),
	Vector2(850, 180),
	Vector2(810, 215),
	Vector2(755, 225),
	Vector2(720, 245),
	Vector2(715, 275),
	Vector2(735, 300),
	Vector2(750, 325),
	Vector2(750, 355),
	Vector2(735, 390),
	Vector2(695, 420),
	Vector2(650, 450),
	Vector2(615, 455),
	Vector2(580, 440),
	Vector2(540, 410),
	Vector2(500, 375),
	Vector2(460, 340),
	Vector2(420, 300),
	Vector2(390, 265),
	Vector2(385, 235),
	Vector2(390, 210),
	Vector2(370, 190),
	Vector2(330, 180),
	Vector2(280, 180),
	Vector2(220, 178),
	Vector2(160, 165),
	Vector2(100, 150),
	Vector2(55, 125),
	Vector2(35, 95),
	Vector2(45, 65),
	Vector2(70, 45),
	Vector2(130, 42),
	Vector2(250, 45),
	Vector2(360, 47),
]

# Mild banking only. These are intentionally small because the current purpose
# is steering/mechanical-grip validation, not elevation reconstruction.
const BANK_DEGREES := {
	4: 2.0,
	5: 3.0,
	14: -2.0,
	15: -2.5,
	31: 2.0,
	32: 2.5,
}

# Curbs are local, matching real circuit practice: apexes and selected exits.
# side -1/+1 selects the two local edges of the Path3D profile.
const CURB_SECTIONS := [
	{"name": "T1Inside", "indices": [2, 3, 4, 5, 6], "side": 1},
	{"name": "T1Exit", "indices": [5, 6, 7], "side": -1},
	{"name": "T2Inside", "indices": [7, 8, 9, 10], "side": -1},
	{"name": "T3Inside", "indices": [10, 11, 12, 13], "side": 1},
	{"name": "T4Inside", "indices": [14, 15, 16, 17], "side": 1},
	{"name": "T4Exit", "indices": [17, 18, 19], "side": -1},
	{"name": "T5Inside", "indices": [22, 23, 24, 25], "side": 1},
	{"name": "T6Inside", "indices": [29, 30, 31, 32, 33], "side": -1},
	{"name": "T6Exit", "indices": [32, 33, 34, 35], "side": 1},
]

var _centerline: Array[Vector3] = []

var _road_material: StandardMaterial3D
var _grass_material: StandardMaterial3D
var _curb_material: StandardMaterial3D
var _line_material: StandardMaterial3D
var _finish_material: StandardMaterial3D

func _ready() -> void:
	_centerline = _trace_to_world()
	_build_materials()
	_build_ground()
	_build_track_surface()
	_build_curbs()
	_build_start_finish()
	_build_spawn_marker()
	_report_reference_metrics()

func _trace_to_world() -> Array[Vector3]:
	var result: Array[Vector3] = []
	for pixel in REFERENCE_TRACE_PIXELS:
		# Clockwise on the supplied map moves left->right across start/finish.
		# Map that direction to Godot vehicle forward (-Z) so the Jordan can use
		# rotation=0 at spawn.
		var lateral_x := (pixel.y - REFERENCE_FINISH_PIXEL.y) * METERS_PER_PIXEL_Y
		var longitudinal_z := -(pixel.x - REFERENCE_FINISH_PIXEL.x) * METERS_PER_PIXEL_X
		result.append(Vector3(lateral_x, 0.0, longitudinal_z))
	return result

func _build_materials() -> void:
	_road_material = StandardMaterial3D.new()
	_road_material.albedo_color = Color(0.18, 0.18, 0.19)
	_road_material.roughness = 0.96

	_grass_material = StandardMaterial3D.new()
	_grass_material.albedo_color = Color(0.18, 0.34, 0.16)
	_grass_material.roughness = 1.0

	_line_material = StandardMaterial3D.new()
	_line_material.albedo_color = Color(0.95, 0.95, 0.93)
	_line_material.roughness = 0.88

	var curb_gradient := Gradient.new()
	curb_gradient.offsets = PackedFloat32Array([
		0.0, 0.16, 0.16, 0.32, 0.32, 0.48, 0.48, 0.64, 0.64, 0.80, 0.80, 1.0
	])
	curb_gradient.colors = PackedColorArray([
		Color(0.87, 0.05, 0.04), Color(0.87, 0.05, 0.04),
		Color(0.96, 0.96, 0.94), Color(0.96, 0.96, 0.94),
		Color(0.87, 0.05, 0.04), Color(0.87, 0.05, 0.04),
		Color(0.96, 0.96, 0.94), Color(0.96, 0.96, 0.94),
		Color(0.87, 0.05, 0.04), Color(0.87, 0.05, 0.04),
		Color(0.96, 0.96, 0.94), Color(0.96, 0.96, 0.94),
	])
	var curb_texture := GradientTexture1D.new()
	curb_texture.gradient = curb_gradient
	curb_texture.width = 256

	_curb_material = StandardMaterial3D.new()
	_curb_material.albedo_texture = curb_texture
	_curb_material.uv1_scale = Vector3(2.4, 1.0, 1.0)
	_curb_material.roughness = 0.74

	_finish_material = StandardMaterial3D.new()
	_finish_material.albedo_texture = _make_finish_texture()
	_finish_material.roughness = 0.65

func _make_finish_texture() -> ImageTexture:
	var image := Image.create(24, 8, false, Image.FORMAT_RGBA8)
	for y in range(8):
		for x in range(24):
			var color := Color.WHITE if ((x + y) % 2 == 0) else Color.BLACK
			image.set_pixel(x, y, color)
	return ImageTexture.create_from_image(image)

func _build_ground() -> void:
	var body := StaticBody3D.new()
	body.name = "GrassRunoff"
	body.add_to_group("Grass")
	add_child(body)

	var ground_mesh := BoxMesh.new()
	ground_mesh.size = Vector3(750.0, 0.50, 1250.0)
	ground_mesh.material = _grass_material

	var visual := MeshInstance3D.new()
	visual.name = "GrassVisual"
	visual.position.y = -0.25
	visual.mesh = ground_mesh
	body.add_child(visual)

	var shape := BoxShape3D.new()
	shape.size = Vector3(750.0, 0.50, 1250.0)

	var collision := CollisionShape3D.new()
	collision.name = "GrassCollision"
	collision.position.y = -0.25
	collision.shape = shape
	body.add_child(collision)

func _build_track_surface() -> void:
	var path := Path3D.new()
	path.name = "Centerline"
	path.curve = _build_curve(_all_indices(), true)
	add_child(path)

	_create_path_shape(
		path,
		"RoadSurface",
		PackedVector2Array([
			Vector2(-ROAD_HALF_WIDTH, ROAD_TOP),
			Vector2(-ROAD_HALF_WIDTH, -ROAD_DEPTH),
			Vector2(ROAD_HALF_WIDTH, -ROAD_DEPTH),
			Vector2(ROAD_HALF_WIDTH, ROAD_TOP),
		]),
		_road_material,
		["Road"],
		true,
		4.0
	)

	_create_edge_line(path, -1)
	_create_edge_line(path, 1)

func _create_edge_line(path: Path3D, side: int) -> void:
	var inner := float(side) * (ROAD_HALF_WIDTH - EDGE_LINE_WIDTH)
	var outer := float(side) * ROAD_HALF_WIDTH
	var x0 := minf(inner, outer)
	var x1 := maxf(inner, outer)

	_create_path_shape(
		path,
		"LeftEdgeLine" if side < 0 else "RightEdgeLine",
		PackedVector2Array([
			Vector2(x0, ROAD_TOP + 0.003),
			Vector2(x0, ROAD_TOP),
			Vector2(x1, ROAD_TOP),
			Vector2(x1, ROAD_TOP + 0.003),
		]),
		_line_material,
		[],
		false,
		2.0
	)

func _build_curbs() -> void:
	for definition in CURB_SECTIONS:
		var path := Path3D.new()
		path.name = "CurbPath_%s" % definition["name"]
		path.curve = _build_curve(definition["indices"], false)
		add_child(path)

		_create_path_shape(
			path,
			"Curb_%s" % definition["name"],
			_build_curb_profile(int(definition["side"])),
			_curb_material,
			["Curb"],
			true,
			0.55
		)

func _build_curb_profile(side: int) -> PackedVector2Array:
	var edge := float(side) * ROAD_HALF_WIDTH
	var p1 := edge + float(side) * 0.12
	var p2 := edge + float(side) * 0.29
	var p3 := edge + float(side) * 0.46
	var outer := edge + float(side) * CURB_WIDTH

	# A low crown rather than a hard rectangular step:
	# road -> +5 mm -> +22 mm -> +12 mm -> +2 mm -> grass.
	var top_points := PackedVector2Array([
		Vector2(edge, ROAD_TOP),
		Vector2(p1, ROAD_TOP + 0.005),
		Vector2(p2, ROAD_TOP + CURB_PEAK_HEIGHT),
		Vector2(p3, ROAD_TOP + 0.012),
		Vector2(outer, ROAD_TOP + 0.002),
	])

	# CSGPolygon3D requires a closed profile. Reverse order automatically by
	# placing the base below the visible curb.
	return PackedVector2Array([
		top_points[0],
		top_points[1],
		top_points[2],
		top_points[3],
		top_points[4],
		Vector2(outer, -CURB_BASE_DEPTH),
		Vector2(edge, -CURB_BASE_DEPTH),
	])

func _build_start_finish() -> void:
	var line := MeshInstance3D.new()
	line.name = "StartFinishLine"

	var mesh := BoxMesh.new()
	mesh.size = Vector3(ROAD_HALF_WIDTH * 2.0, 0.012, 2.2)
	mesh.material = _finish_material
	line.mesh = mesh
	line.position = Vector3(0.0, ROAD_TOP + 0.008, 0.0)
	add_child(line)

func _build_spawn_marker() -> void:
	var marker := Marker3D.new()
	marker.name = "PlayerSpawn"
	marker.position = Vector3(0.0, ROAD_TOP, 18.0)
	marker.rotation = Vector3.ZERO
	add_child(marker)

func _create_path_shape(
	path: Path3D,
	shape_name: String,
	profile: PackedVector2Array,
	material: Material,
	groups: Array,
	collision_enabled: bool,
	u_distance: float
) -> void:
	var shape := CSGPolygon3D.new()
	shape.name = shape_name
	shape.mode = CSGPolygon3D.MODE_PATH
	shape.polygon = profile
	shape.path_node = NodePath("..")
	shape.path_interval_type = CSGPolygon3D.PATH_INTERVAL_DISTANCE
	shape.path_interval = 0.75
	shape.path_simplify_angle = 0.4
	shape.path_rotation = CSGPolygon3D.PATH_ROTATION_PATH_FOLLOW
	shape.path_rotation_accurate = true
	shape.path_local = false
	shape.path_continuous_u = true
	shape.path_u_distance = u_distance
	shape.path_joined = path.curve.closed
	shape.smooth_faces = false
	shape.material = material
	shape.use_collision = collision_enabled
	path.add_child(shape)

	for group_name in groups:
		shape.add_to_group(StringName(group_name))

func _build_curve(indices: Array, closed: bool) -> Curve3D:
	var curve := Curve3D.new()
	curve.closed = closed
	curve.bake_interval = 0.40

	for raw_index in indices:
		var index := int(raw_index)
		var previous := _centerline[(index - 1 + _centerline.size()) % _centerline.size()]
		var following := _centerline[(index + 1) % _centerline.size()]
		var tangent := (following - previous) * HANDLE_SCALE

		curve.add_point(_centerline[index], -tangent, tangent)

		if BANK_DEGREES.has(index):
			curve.set_point_tilt(
				curve.point_count - 1,
				deg_to_rad(float(BANK_DEGREES[index]))
			)

	return curve

func _all_indices() -> Array[int]:
	var result: Array[int] = []
	for index in range(_centerline.size()):
		result.append(index)
	return result

func _report_reference_metrics() -> void:
	var curve := _build_curve(_all_indices(), true)
	var lap_length := curve.get_baked_length()
	var straight_length := 0.0
	var straight_indices := [34, 35, 36, 37, 0, 1, 2]

	for i in range(straight_indices.size() - 1):
		straight_length += _centerline[straight_indices[i]].distance_to(
			_centerline[straight_indices[i + 1]]
		)

	print(
		"[LaChutana] baked lap=%.1fm target=%.1fm | main straight ~=%.1fm target=%.1fm"
		% [lap_length, TRACK_LENGTH_TARGET_M, straight_length, MAIN_STRAIGHT_TARGET_M]
	)
