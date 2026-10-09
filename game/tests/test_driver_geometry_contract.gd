extends SceneTree

func _init() -> void:
	var controller: Node = (load("res://scripts/vehicle/driver_visual_controller.gd") as GDScript).new()
	var canonical_model := (load("res://assets/models/drivers/driver.glb") as PackedScene).instantiate()
	controller.set("driver_instance", canonical_model)
	var canonical_accepted := bool(controller.call("validate_driver_geometry_contract"))
	var geometry_contract := canonical_model.find_child("DriverOriginalUniformGeometryContract", true, false)
	if not canonical_accepted or geometry_contract == null:
		printerr("The canonical driver does not satisfy the original geometry contract.")
		canonical_model.free()
		controller.free()
		quit(1)
		return
	var contract_parent := geometry_contract.get_parent()
	contract_parent.remove_child(geometry_contract)
	var missing_contract_rejected := not bool(controller.call("validate_driver_geometry_contract"))
	contract_parent.add_child(geometry_contract)
	var original_properties: Dictionary = geometry_contract.get_meta("extras")
	var wrong_source_properties := original_properties.duplicate()
	wrong_source_properties["source_sha256"] = "deprecated_source"
	geometry_contract.set_meta("extras", wrong_source_properties)
	var wrong_source_rejected := not bool(controller.call("validate_driver_geometry_contract"))
	geometry_contract.set_meta("extras", original_properties)
	var restored_contract_accepted := bool(controller.call("validate_driver_geometry_contract"))
	canonical_model.free()
	controller.free()
	print("DRIVER_CANONICAL_GEOMETRY_ACCEPTED=", canonical_accepted)
	print("DRIVER_MISSING_CONTRACT_REJECTED=", missing_contract_rejected)
	print("DRIVER_WRONG_SOURCE_REJECTED=", wrong_source_rejected)
	print("DRIVER_RESTORED_CONTRACT_ACCEPTED=", restored_contract_accepted)
	quit(0 if canonical_accepted and missing_contract_rejected and wrong_source_rejected and restored_contract_accepted else 1)
