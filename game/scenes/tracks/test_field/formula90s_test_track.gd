extends Node3D

## Formula90s handling-development circuit.
##
## This is intentionally outside addons/gevp so the vendor demo track remains
## untouched. The layout keeps longitudinal elevation flat to remove artificial
## ramp effects; selected corners use Curve3D tilt for controlled banking.
## Curbs and barriers are local features instead of full-lap extrusions.

const ROAD_HALF_WIDTH := 6.0
const ROAD_TOP := 0.005
const ROAD_DEPTH := 0.15
const EDGE_LINE_WIDTH := 0.12
const CURB_WIDTH := 0.55
const CURB_RISE := 0.025
const BARRIER_OFFSET := 14.0
const BARRIER_THICKNESS := 0.20
const BARRIER_HEIGHT := 1.10
const HANDLE_SCALE := 0.22

# Preserves the useful plan-view character of the former GEVP test circuit but
# removes the +3 m longitudinal ramp near the final sector. All centreline
# points are Y=0; banking is authored separately through Curve3D tilt.
const CENTERLINE: Array[Vector3] = [
	Vector3(0.0503, 0.0, 0.2512),
	Vector3(-0.0711, 0.0, 64.0364),
	Vector3(-0.0711, 0.0, 127.9390),
	Vector3(0.0, 0.0, 194.0),
	Vector3(63.5462, 0.0, 255.3610),
	Vector3(98.7882, 0.0, 192.3750),
	Vector3(65.0459, 0.0, 148.8850),
	Vector3(102.5370, 0.0, 133.1390),
	Vector3(169.8500, 0.0, 172.6990),
	Vector3(251.3170, 0.0, 209.2390),
	Vector3(336.3780, 0.0, 197.2590),
	Vector3(362.6440, 0.0, 139.5270),
	Vector3(363.8420, 0.0, 96.9970),
	Vector3(363.4770, 0.0, 46.9086),
	Vector3(298.0980, 0.0, 13.2871),
	Vector3(276.6300, 0.0, -66.6769),
	Vector3(199.8360, 0.0, -97.6944),
	Vector3(148.1050, 0.0, -157.5150),
	Vector3(76.5890, 0.0, -186.4110),
	Vector3(25.3000, 0.0, -177.0200),
	Vector3(2.1930, 0.0, -126.6630),
	Vector3(-0.0566, 0.0, -47.1814),
]

# Moderate bank angles only. The old test track used roughly -28.6 degrees in
# the final sector, which behaved more like a ramp/wall than a normal circuit.
const BANK_DEGREES := {
	3: 3.0,
	4: 7.0,
	5: 5.0,
	9: -3.0,
	10: -6.0,
	11: -5.0,
	17: 4.0,
	18: 9.0,
	19: 7.0,
}

# Curbs are deliberately local. Convention: side -1 = left, +1 = right.
# They cover apex/exit zones, never long straights or every metre of track.
const CURB_SECTIONS := [
	{"name": "T4Inside", "indices": [3, 4, 5], "side": 1},
	{"name": "T4Exit", "indices": [4, 5], "side": -1},
	{"name": "T6Inside", "indices": [5, 6, 7], "side": -1},
	{"name": "T6Exit", "indices": [6, 7], "side": 1},
	{"name": "LongRightInside", "indices": [9, 10, 11], "side": 1},
	{"name": "LongRightExit", "indices": [10, 11], "side": -1},
	{"name": "ComplexRight", "indices": [12, 13, 14], "side": 1},
	{"name": "ComplexLeft", "indices": [13, 14, 15], "side": -1},
	{"name": "FinalBankedInside", "indices": [17, 18, 19], "side": 1},
	{"name": "FinalBankedExit", "indices": [18, 19, 20], "side": -1},
]

# Protection exists only where it adds a useful high-speed boundary test.
# The large gaps are intentional grass/run-off test zones.
const BARRIER_SECTIONS := [
	{"name": "MainStraightLeft", "indices": [0, 1, 2, 3], "side": -1},
	{"name": "MainStraightRight", "indices": [0, 1, 2, 3], "side": 1},
	{"name": "FastSweeperOuter", "indices": [9, 10, 11, 12], "side": 1},
	{"name": "FinalBankedOuter", "indices": [17, 18, 19, 20], "side": 1},
]

var _road_material: StandardMaterial3D
var _grass_material: StandardMaterial3D
var _curb_material: StandardMaterial3D
var _line_material: StandardMaterial3D
var _guardrail_material: StandardMaterial3D

func _ready() -> void:
	_build_materials()
	_build_ground()
	_build_track_surface()
	_build_curbs()
	_build_barriers()

func _build_materials() -> void:
	_road_material = StandardMaterial3D.new()
	_road_material.albedo_color = Color(0.26, 0.27, 0.28)
	_road_material.roughness = 0.93

	_grass_material = StandardMaterial3D.new()
	_grass_material.albedo_color = Color(0.14, 0.34, 0.12)
	_grass_material.roughness = 1.0

	_line_material = StandardMaterial3D.new()
	_line_material.albedo_color = Color(0.92, 0.92, 0.90)
	_line_material.roughness = 0.85

	_guardrail_material = StandardMaterial3D.new()
	_guardrail_material.albedo_color = Color(0.52, 0.54, 0.57)
	_guardrail_material.metallic = 0.82
	_guardrail_material.roughness = 0.35

	var gradient := Gradient.new()
	gradient.offsets = PackedFloat32Array([0.0, 0.18, 0.18, 0.36, 0.36, 0.54, 0.54, 0.72, 0.72, 1.0])
	gradient.colors = PackedColorArray([
		Color(0.86, 0.05, 0.04), Color(0.86, 0.05, 0.04),
		Color(0.95, 0.95, 0.92), Color(0.95, 0.95, 0.92),
		Color(0.86, 0.05, 0.04), Color(0.86, 0.05, 0.04),
		Color(0.95, 0.95, 0.92), Color(0.95, 0.95, 0.92),
		Color(0.86, 0.05, 0.04), Color(0.86, 0.05, 0.04),
	])
	var curb_texture := GradientTexture1D.new()
	curb_texture.gradient = gradient
	curb_texture.width = 256

	_curb_material = StandardMaterial3D.new()
	_curb_material.albedo_texture = curb_texture
	_curb_material.uv1_scale = Vector3(2.5, 1.0, 1.0)
	_curb_material.roughness = 0.72

func _build_ground() -> void:
	var body := StaticBody3D.new()
	body.name = "GrassRunoff"
	body.add_to_group("Grass")
	add_child(body)

	var mesh := BoxMesh.new()
	mesh.size = Vector3(900.0, 0.40, 900.0)
	mesh.material = _grass_material
	var visual := MeshInstance3D.new()
	visual.name = "GrassVisual"
	visual.position.y = -0.20
	visual.mesh = mesh
	body.add_child(visual)

	var shape := BoxShape3D.new()
	shape.size = Vector3(900.0, 0.40, 900.0)
	var collision := CollisionShape3D.new()
	collision.name = "GrassCollision"
	collision.position.y = -0.20
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
		3.0
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
			Vector2(x0, ROAD_TOP + 0.004),
			Vector2(x0, ROAD_TOP),
			Vector2(x1, ROAD_TOP),
			Vector2(x1, ROAD_TOP + 0.004),
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

		var side := int(definition["side"])
		var edge := float(side) * ROAD_HALF_WIDTH
		var outside := float(side) * (ROAD_HALF_WIDTH + CURB_WIDTH)
		var x0 := minf(edge, outside)
		var x1 := maxf(edge, outside)
		_create_path_shape(
			path,
			"Curb_%s" % definition["name"],
			PackedVector2Array([
				Vector2(x0, ROAD_TOP + CURB_RISE),
				Vector2(x0, -0.03),
				Vector2(x1, -0.03),
				Vector2(x1, ROAD_TOP + CURB_RISE),
			]),
			_curb_material,
			["Curb"],
			true,
			0.45
		)

func _build_barriers() -> void:
	for definition in BARRIER_SECTIONS:
		var path := Path3D.new()
		path.name = "BarrierPath_%s" % definition["name"]
		path.curve = _build_curve(definition["indices"], false)
		add_child(path)

		var side := int(definition["side"])
		var center_x := float(side) * BARRIER_OFFSET
		var x0 := center_x - BARRIER_THICKNESS * 0.5
		var x1 := center_x + BARRIER_THICKNESS * 0.5
		_create_path_shape(
			path,
			"Guardrail_%s" % definition["name"],
			PackedVector2Array([
				Vector2(x0, 0.0),
				Vector2(x0, BARRIER_HEIGHT),
				Vector2(x1, BARRIER_HEIGHT),
				Vector2(x1, 0.0),
			]),
			_guardrail_material,
			["Wall"],
			true,
			2.0
		)

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
	shape.path_interval = 1.0
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
	curve.bake_interval = 0.50
	for raw_index in indices:
		var index := int(raw_index)
		var previous := CENTERLINE[(index - 1 + CENTERLINE.size()) % CENTERLINE.size()]
		var following := CENTERLINE[(index + 1) % CENTERLINE.size()]
		var tangent := (following - previous) * HANDLE_SCALE
		curve.add_point(CENTERLINE[index], -tangent, tangent)
		if BANK_DEGREES.has(index):
			curve.set_point_tilt(curve.point_count - 1, deg_to_rad(float(BANK_DEGREES[index])))
	return curve

func _all_indices() -> Array[int]:
	var result: Array[int] = []
	for index in range(CENTERLINE.size()):
		result.append(index)
	return result
