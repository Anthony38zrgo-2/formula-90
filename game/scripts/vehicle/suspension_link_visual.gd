class_name SuspensionLinkVisual
extends Node3D

## Procedural visual builder for the suspension linkage. Reads the pose data
## produced by SuspensionGeometry and drives attachment-based rigid meshes so the whole
## chassis -> suspension -> wheel assembly stays connected under physics.

const WHEEL_KEYS := ["FL", "FR", "RL", "RR"]

var _rack_housing: MeshInstance3D
var _rack_bar: MeshInstance3D
var _rack_points: Array = []
var _source_parts: Array = []
var _geometry: SuspensionGeometry
var _wishbone_lower: Array = []
var _wishbone_upper: Array = []
var _trackrod: Array = []
var _pushrod: Array = []
var _damper: Array = []
var _driveshaft: Array = []
var _rocker: Array = []
var _upright: Array = []
var _joints: Array = []
var _visual_corners: Array = []

var _rod_material: Material
var _arm_material: Material
var _shaft_material: Material
var _rocker_material: Material


func setup(geometry: SuspensionGeometry) -> void:
	for child in get_children():
		remove_child(child)
		child.queue_free()
	_geometry = geometry
	_rod_material = _make_material(Color(0.60, 0.60, 0.63), 0.9, 0.32)
	_arm_material = _make_material(Color(0.18, 0.18, 0.20), 0.65, 0.45)
	_shaft_material = _make_material(Color(0.72, 0.72, 0.76), 0.9, 0.28)
	_rocker_material = _make_material(Color(0.30, 0.05, 0.05), 0.5, 0.5)

	_wishbone_lower.clear()
	_wishbone_upper.clear()
	_trackrod.clear()
	_pushrod.clear()
	_damper.clear()
	_driveshaft.clear()
	_rocker.clear()
	_upright.clear()
	_joints.clear()
	_source_parts.clear()
	_visual_corners.clear()
	var source_data := {}
	if not geometry.visual_meshes_path.is_empty():
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(geometry.visual_meshes_path))
		if parsed is Dictionary:
			source_data = parsed.get("corners", {})

	for wheel_index in range(4):
		var corner := _geometry.get_corner(wheel_index).duplicate()
		var packaging: Dictionary = geometry.visual_packaging.get("front", {}) if wheel_index < 2 else {}
		if not packaging.is_empty():
			for pair in [["pivot", "rocker_pivot"], ["pushrod_arm", "rocker_arm_rest"], ["damper_arm", "damper_arm_rest"], ["damper_chassis", "damper_chassis"]]:
				var value: Array = packaging[pair[0]]
				corner[pair[1]] = Vector3(float(value[0]) * (-1.0 if wheel_index == 0 else 1.0), value[1], value[2])
			corner["L_pushrod"] = corner["pushrod_outer_rest"].distance_to(corner["rocker_arm_rest"])
		_visual_corners.append(corner)
		var root := Node3D.new()
		root.name = "Susp_%s" % WHEEL_KEYS[wheel_index]
		root.visible = false
		var authored := {}
		var meshes: Dictionary = source_data.get(WHEEL_KEYS[wheel_index], {})
		for role in meshes:
			var part := MeshInstance3D.new()
			part.name = "Authored_" + role
			var surface := SurfaceTool.new()
			surface.begin(Mesh.PRIMITIVE_TRIANGLES)
			for index in meshes[role]["indices"]:
				var vertex: Array = meshes[role]["vertices"][int(index)]
				var point := Vector3(vertex[0], vertex[1], vertex[2])
				if role == "pushrod" and not packaging.is_empty():
					# Resize once in the rest frame. Animation remains rigid.
					var source := geometry.get_corner(wheel_index)
					var origin: Vector3 = source["pushrod_outer_rest"]
					var old_axis: Vector3 = source["rocker_arm_rest"] - origin
					var new_axis: Vector3 = corner["rocker_arm_rest"] - origin
					var along := (point - origin).dot(old_axis.normalized())
					var radial := point - origin - old_axis.normalized() * along
					var rotation := Basis(Quaternion(old_axis.normalized(), new_axis.normalized()))
					point = origin + new_axis * (along / old_axis.length()) + rotation * radial * float(packaging["pushrod_section_scale"])
				surface.add_vertex(point)
			surface.generate_normals()
			part.mesh = surface.commit()
			part.material_override = _arm_material
			root.add_child(part)
			authored[role] = part
		_source_parts.append(authored)
		add_child(root)

		var lower := Node3D.new()
		lower.name = "LOWER_WISHBONE"
		root.add_child(lower)
		lower.add_child(_make_rod(_arm_material, 0.028))
		lower.add_child(_make_rod(_arm_material, 0.028))
		lower.add_child(_make_rod(_arm_material, 0.018))
		lower.add_child(_make_rod(_arm_material, 0.018))
		_wishbone_lower.append(lower)

		var upper := Node3D.new()
		upper.name = "UPPER_WISHBONE"
		root.add_child(upper)
		upper.add_child(_make_rod(_arm_material, 0.022))
		upper.add_child(_make_rod(_arm_material, 0.022))
		upper.add_child(_make_rod(_arm_material, 0.018))
		upper.add_child(_make_rod(_arm_material, 0.018))
		_wishbone_upper.append(upper)

		_trackrod.append(_make_rod_node(root, "TRACKROD", _rod_material, 0.012))
		_pushrod.append(_make_rod_node(root, "PUSHROD", _rod_material, 0.016))
		var damper := Node3D.new()
		damper.name = "DAMPER"
		root.add_child(damper)
		_make_rod_node(damper, "Body", _arm_material, 0.024 if not packaging.is_empty() else 0.040)
		_make_rod_node(damper, "Piston", _shaft_material, 0.010 if not packaging.is_empty() else 0.014)
		_damper.append(damper)
		if _geometry.is_valid(wheel_index) and _geometry.get_corner(wheel_index).get("has_driveshaft", false):
			_driveshaft.append(_make_rod_node(root, "DRIVESHAFT", _shaft_material, 0.012, true))
		else:
			# Placeholder keeps per-wheel array indexing stable.
			var placeholder := MeshInstance3D.new()
			placeholder.name = "DRIVESHAFT"
			placeholder.visible = false
			root.add_child(placeholder)
			_driveshaft.append(placeholder)
		if not geometry.is_valid(wheel_index):
			root.visible = false
			_rocker.append(null)
			_upright.append(null)
			_joints.append([])
			continue
		var pivot: Vector3 = corner["rocker_pivot"]
		_rocker.append(_make_plate(root, "ROCKER", [Vector3.ZERO, corner["rocker_arm_rest"] - pivot, corner["damper_arm_rest"] - pivot], 0.006 if not packaging.is_empty() else 0.012, _rocker_material))
		var hub: Vector3 = corner["hub_center"]
		var upright := _make_plate(root, "UPRIGHT", [Vector3.ZERO, corner["lbj_rest"] - hub, corner["ubj_rest"] - hub], 0.024, _arm_material)
		var steering_arm := _make_rod_node(upright, "SteeringArm", _arm_material, 0.026)
		_set_rod(steering_arm, Vector3.ZERO, corner["trackrod_outer"] - hub, 0.0, false)
		var side := 1.0 if wheel_index % 2 == 0 else -1.0
		var alignment := Basis(Vector3.UP, float(corner["toe"]) * side) * Basis(Vector3.BACK, float(corner["camber"]) * side)
		var bearing := _make_rod_node(upright, "WheelBearing", _shaft_material, 0.07)
		_set_rod(bearing, -alignment.x * 0.035, alignment.x * 0.035, 0.0, false)
		_upright.append(upright)
		var joints := []
		for joint_index in range(14):
			var joint := MeshInstance3D.new()
			joint.name = "Joint_%02d" % joint_index
			var sphere := SphereMesh.new()
			sphere.radius = float(packaging.get("joint_radius", 0.018))
			sphere.height = sphere.radius * 2.0
			sphere.radial_segments = 12
			sphere.rings = 6
			joint.mesh = sphere
			joint.material_override = _shaft_material
			root.add_child(joint)
			joints.append(joint)
		_joints.append(joints)

	if geometry.is_valid(0) and geometry.is_valid(1):
		_rack_points = [geometry.get_corner(0)["trackrod_inner"], geometry.get_corner(1)["trackrod_inner"]]
		_rack_housing = _make_rod_node(self, "SteeringRackHousing", _arm_material, 0.045)
		_rack_bar = _make_rod_node(self, "SteeringRack", _shaft_material, 0.020)
		var center: Vector3 = (_rack_points[0] + _rack_points[1]) * 0.5
		_set_rod(_rack_housing, center.lerp(_rack_points[0], 0.75), center.lerp(_rack_points[1], 0.75), 0.0, false)
	visible = true


func set_links_visible(enabled: bool) -> void:
	visible = enabled


func update_wheel(wheel_index: int, data: Dictionary) -> void:
	if data.is_empty() or not data.get("present", false):
		return
	data = visual_pose(wheel_index, data)

	(_wishbone_lower[wheel_index].get_parent() as Node3D).visible = true
	var authored: Dictionary = _source_parts[wheel_index]
	for role in authored:
		authored[role].transform = data[role + "_pose"]
	_wishbone_lower[wheel_index].visible = not authored.has("lower")
	_wishbone_upper[wheel_index].visible = not authored.has("upper")
	var lower: Array = data["lower"]
	_set_rod(_wishbone_lower[wheel_index].get_child(0), lower[0], lower[2], 0.0, false)
	_set_rod(_wishbone_lower[wheel_index].get_child(1), lower[1], lower[2], 0.0, false)

	var upper: Array = data["upper"]
	_set_rod(_wishbone_upper[wheel_index].get_child(0), upper[0], upper[2], 0.0, false)
	_set_rod(_wishbone_upper[wheel_index].get_child(1), upper[1], upper[2], 0.0, false)

	var trackrod: Array = data["trackrod"]
	if wheel_index < 2 and _rack_bar != null:
		_rack_points[wheel_index] = trackrod[0]
		_set_rod(_rack_bar, _rack_points[0], _rack_points[1], 0.0, false)
	_set_rod(_trackrod[wheel_index], trackrod[0], trackrod[1], 0.0, false)

	var pushrod: Array = data["pushrod"]
	_set_rod(_pushrod[wheel_index], pushrod[0], pushrod[1], 0.0, false)
	# The offset pushrod lug is supported by both legs of the lower A-arm.
	var mount: Array = upper if data["pushrod_mount"] == "upper" else lower
	var mount_node: Node3D = _wishbone_upper[wheel_index] if data["pushrod_mount"] == "upper" else _wishbone_lower[wheel_index]
	for i in range(2):
		var foot := Geometry3D.get_closest_point_to_segment(pushrod[0], mount[i], mount[2])
		_set_rod(mount_node.get_child(i + 2), foot, pushrod[0], 0.0, false)

	_trackrod[wheel_index].visible = not authored.has("trackrod")
	_pushrod[wheel_index].visible = not authored.has("pushrod")
	var damper: Array = data["damper"]
	var corner: Dictionary = _visual_corners[wheel_index]
	var rest_length: float = (corner["damper_chassis"] - corner["damper_arm_rest"]).length()
	var direction: Vector3 = (damper[1] - damper[0]).normalized()
	# Fixed body and piston lengths; only their overlap changes with travel.
	_set_rod(_damper[wheel_index].get_child(0), damper[0], damper[0] + direction * rest_length * 0.60, 0.0, false)
	_set_rod(_damper[wheel_index].get_child(1), damper[1] - direction * rest_length * 0.65, damper[1], 0.0, false)

	var rocker: Dictionary = data["rocker"]
	_set_mesh(_rocker[wheel_index], rocker["origin"], rocker["basis"])

	var upright_origin: Vector3 = data["upright_origin"]
	var upright_basis: Basis = data["upright_basis"]
	_set_mesh(_upright[wheel_index], upright_origin, upright_basis)
	var points := [lower[0], lower[1], lower[2], upper[0], upper[1], upper[2], trackrod[0], trackrod[1], pushrod[0], pushrod[1], rocker["origin"], damper[0], damper[1], upright_origin]
	for i in range(points.size()):
		_joints[wheel_index][i].position = points[i]

	if data.has("driveshaft") and not (data["driveshaft"] is Dictionary and data["driveshaft"].is_empty()):
		var ds: Dictionary = data["driveshaft"]
		_set_rod(_driveshaft[wheel_index], ds["inner"], ds["outer"], float(ds["spin"]), true)


## Cosmetic inboard mechanism only: wheel, wishbones and native forces retain
## their physical geometry. Solve the smaller lever with a fixed-length rod,
## rather than scaling animated links or moving their wheel-side attachment.
func visual_pose(wheel_index: int, physical_pose: Dictionary) -> Dictionary:
	if wheel_index >= 2 or not _geometry.visual_packaging.has("front"):
		return physical_pose
	var corner: Dictionary = _visual_corners[wheel_index]
	var outer: Vector3 = physical_pose["pushrod"][0]
	var solved := _geometry._rocker_solve(corner, outer)
	var basis := Basis(corner["rocker_axis"], float(solved["angle"]))
	var pivot: Vector3 = corner["rocker_pivot"]
	var damper_end: Vector3 = pivot + basis * (corner["damper_arm_rest"] - pivot)
	var pose := physical_pose.duplicate()
	pose["pushrod"] = [outer, solved["point"]]
	pose["pushrod_pose"] = _geometry._link_pose(corner["pushrod_outer_rest"], corner["rocker_arm_rest"], outer, solved["point"])
	pose["rocker"] = {"origin": pivot, "basis": basis, "pushrod": solved["point"], "damper": damper_end}
	pose["damper"] = [corner["damper_chassis"], damper_end]
	pose["rocker_clamped"] = solved["clamped"]
	return pose


func _make_rod_node(parent: Node, name: String, material: Material, radius: float, flat: bool = false) -> MeshInstance3D:
	var rod := _make_rod(material, radius, flat)
	rod.name = name
	parent.add_child(rod)
	return rod


func _make_rod(material: Material, radius: float, _flat: bool = false) -> MeshInstance3D:
	# Existing dimensions describe outside diameter, not radius.
	var mesh := CylinderMesh.new()
	mesh.top_radius = radius * 0.5
	mesh.bottom_radius = radius * 0.5
	mesh.height = 1.0
	mesh.radial_segments = 16
	var instance := MeshInstance3D.new()
	instance.mesh = mesh
	instance.material_override = material
	return instance


## Extruded triangular forging whose vertices are the attachment centers.
## Built once in the component rest frame; animation only applies rigid poses.
func _make_plate(parent: Node, label: String, points: Array, thickness: float, material: Material) -> MeshInstance3D:
	var a: Vector3 = points[0]
	var b: Vector3 = points[1]
	var c: Vector3 = points[2]
	var normal := (b - a).cross(c - a).normalized()
	var vertices := PackedVector3Array()
	for sign_value in [-1.0, 1.0]:
		for point in points:
			vertices.append(point + normal * thickness * 0.5 * sign_value)
	var indices := [0, 2, 1, 3, 4, 5, 0, 1, 4, 0, 4, 3, 1, 2, 5, 1, 5, 4, 2, 0, 3, 2, 3, 5]
	var surface := SurfaceTool.new()
	surface.begin(Mesh.PRIMITIVE_TRIANGLES)
	for index in indices:
		surface.add_vertex(vertices[index])
	surface.generate_normals()
	var instance := MeshInstance3D.new()
	instance.name = label
	instance.mesh = surface.commit()
	instance.material_override = material
	parent.add_child(instance)
	return instance


func _make_material(albedo: Color, metallic: float, roughness: float) -> Material:
	var material := StandardMaterial3D.new()
	material.albedo_color = albedo
	material.metallic = metallic
	material.roughness = roughness
	return material


func _set_rod(node: MeshInstance3D, a: Vector3, b: Vector3, spin: float, _flat: bool) -> void:
	var dir := b - a
	var length := dir.length()
	if length < 1e-6 or not (a.is_finite() and b.is_finite()):
		node.visible = false
		return
	node.visible = true
	var y := dir / length
	var x := y.cross(Vector3.UP).normalized()
	if x.length() < 1e-6:
		x = y.cross(Vector3.FORWARD).normalized()
	if x.length() < 1e-6:
		x = Vector3.RIGHT
	var z := x.cross(y).normalized()
	var basis := Basis(x, y, z)
	if spin != 0.0:
		basis = basis * Basis(Vector3.UP, spin)
	node.position = (a + b) * 0.5
	node.basis = basis
	node.scale = Vector3(1.0, length, 1.0)


func _set_mesh(node: MeshInstance3D, origin: Vector3, basis: Basis) -> void:
	if not origin.is_finite() or not basis.is_finite():
		node.visible = false
		return
	node.visible = true
	node.position = origin
	node.basis = basis
	node.scale = Vector3.ONE
