@tool
extends EditorScenePostImport

const SHARED_TIRE_MATERIAL_PATH := "res://assets/models/vehicles/f1-2030/formula_one_2030_tire_material.tres"

func _post_import(scene: Node) -> Object:
	var shared_tire_material := load(SHARED_TIRE_MATERIAL_PATH) as StandardMaterial3D
	assert(shared_tire_material != null)
	var pending_nodes: Array[Node] = [scene]
	var configured_surfaces := 0
	while not pending_nodes.is_empty():
		var current_node: Node = pending_nodes.pop_back()
		for child in current_node.get_children():
			pending_nodes.append(child)
		if current_node is MeshInstance3D and String(current_node.name).begins_with("GEO_WHEEL_") and String(current_node.name).ends_with("_TIRE"):
			var tire_instance := current_node as MeshInstance3D
			var tire_mesh := tire_instance.mesh.duplicate() as ArrayMesh
			for surface_index in tire_mesh.get_surface_count():
				tire_mesh.surface_set_material(surface_index, shared_tire_material)
				configured_surfaces += 1
			tire_instance.mesh = tire_mesh
	assert(configured_surfaces == 1)
	print("FORMULA_ONE_2030_SHARED_TIRE_MATERIAL ", get_source_file())
	return scene
