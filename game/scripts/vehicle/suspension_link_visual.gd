class_name SuspensionLinkVisual
extends Node3D

## Procedural visual builder for the suspension linkage. Reads the pose data
## produced by SuspensionGeometry and drives simple rod/box meshes so the whole
## chassis -> suspension -> wheel assembly stays connected under physics.

const WHEEL_KEYS := ["FL", "FR", "RL", "RR"]

var _geometry: SuspensionGeometry
var _wishbone_lower: Array = []
var _wishbone_upper: Array = []
var _trackrod: Array = []
var _pushrod: Array = []
var _damper: Array = []
var _driveshaft: Array = []
var _rocker: Array = []
var _upright: Array = []

var _rod_material: Material
var _arm_material: Material
var _shaft_material: Material
var _rocker_material: Material


func setup(geometry: SuspensionGeometry) -> void:
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

	for wheel_index in range(4):
		var root := Node3D.new()
		root.name = "Susp_%s" % WHEEL_KEYS[wheel_index]
		add_child(root)

		var lower := Node3D.new()
		lower.name = "LOWER_WISHBONE"
		root.add_child(lower)
		lower.add_child(_make_rod(_arm_material, 0.028))
		lower.add_child(_make_rod(_arm_material, 0.028))
		_wishbone_lower.append(lower)

		var upper := Node3D.new()
		upper.name = "UPPER_WISHBONE"
		root.add_child(upper)
		upper.add_child(_make_rod(_arm_material, 0.022))
		upper.add_child(_make_rod(_arm_material, 0.022))
		_wishbone_upper.append(upper)

		_trackrod.append(_make_rod_node(root, "TRACKROD", _rod_material, 0.012))
		_pushrod.append(_make_rod_node(root, "PUSHROD", _rod_material, 0.016))
		_damper.append(_make_rod_node(root, "DAMPER", _shaft_material, 0.018))
		_driveshaft.append(_make_rod_node(root, "DRIVESHAFT", _shaft_material, 0.012, true))
		_rocker.append(_make_box_node(root, "ROCKER", _rocker_material, Vector3(0.09, 0.03, 0.045)))
		_upright.append(_make_box_node(root, "UPRIGHT", _arm_material, Vector3(0.10, 0.24, 0.08)))

	if not _geometry.is_valid(0):
		visible = false


func set_links_visible(enabled: bool) -> void:
	visible = enabled


func update_wheel(wheel_index: int, data: Dictionary) -> void:
	if data.is_empty() or not data.get("present", false):
		return

	var lower: Array = data["lower"]
	_set_rod(_wishbone_lower[wheel_index].get_child(0), lower[0], lower[2], 0.0, false)
	_set_rod(_wishbone_lower[wheel_index].get_child(1), lower[1], lower[2], 0.0, false)

	var upper: Array = data["upper"]
	_set_rod(_wishbone_upper[wheel_index].get_child(0), upper[0], upper[2], 0.0, false)
	_set_rod(_wishbone_upper[wheel_index].get_child(1), upper[1], upper[2], 0.0, false)

	var trackrod: Array = data["trackrod"]
	_set_rod(_trackrod[wheel_index], trackrod[0], trackrod[1], 0.0, false)

	var pushrod: Array = data["pushrod"]
	_set_rod(_pushrod[wheel_index], pushrod[0], pushrod[1], 0.0, false)

	var damper: Array = data["damper"]
	_set_rod(_damper[wheel_index], damper[0], damper[1], 0.0, false)

	var rocker: Dictionary = data["rocker"]
	_set_mesh(_rocker[wheel_index], rocker["origin"], rocker["basis"])

	var upright_origin: Vector3 = data["upright_origin"]
	var upright_basis: Basis = data["upright_basis"]
	_set_mesh(_upright[wheel_index], upright_origin, upright_basis)

	if data.has("driveshaft") and not (data["driveshaft"] is Dictionary and data["driveshaft"].is_empty()):
		var ds: Dictionary = data["driveshaft"]
		_set_rod(_driveshaft[wheel_index], ds["inner"], ds["outer"], float(ds["spin"]), true)


func _make_rod_node(parent: Node, name: String, material: Material, radius: float, flat: bool = false) -> MeshInstance3D:
	var rod := _make_rod(material, radius, flat)
	rod.name = name
	parent.add_child(rod)
	return rod


func _make_rod(material: Material, radius: float, flat: bool = false) -> MeshInstance3D:
	var mesh := BoxMesh.new()
	if flat:
		mesh.size = Vector3(0.018, 1.0, 0.05)
	else:
		mesh.size = Vector3(radius, 1.0, radius)
	var instance := MeshInstance3D.new()
	instance.mesh = mesh
	instance.material_override = material
	return instance


func _make_box_node(parent: Node, name: String, material: Material, size: Vector3) -> MeshInstance3D:
	var mesh := BoxMesh.new()
	mesh.size = size
	var instance := MeshInstance3D.new()
	instance.mesh = mesh
	instance.material_override = material
	instance.name = name
	parent.add_child(instance)
	return instance


func _make_material(albedo: Color, metallic: float, roughness: float) -> Material:
	var material := StandardMaterial3D.new()
	material.albedo_color = albedo
	material.metallic = metallic
	material.roughness = roughness
	return material


func _set_rod(node: MeshInstance3D, a: Vector3, b: Vector3, spin: float, flat: bool) -> void:
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