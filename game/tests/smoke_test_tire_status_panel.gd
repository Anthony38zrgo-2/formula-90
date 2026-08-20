extends SceneTree

# Verifies the TireStatusPanel renders per-wheel pressure + 5-node temperatures
# from the native/GDScript snapshot contract used by the HUD.

func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var failures: Array[String] = []
	var panel_script := load("res://scripts/hud/tire_status_panel.gd") as GDScript
	if panel_script == null:
		printerr("[FAIL] tire_status_panel.gd could not be loaded")
		quit(1)
		return
	var panel := panel_script.new() as Control
	if panel == null:
		printerr("[FAIL] TireStatusPanel could not be instantiated")
		quit(1)
		return
	root.add_child(panel)
	await process_frame

	panel.call("set_tire_data", {
		"FL": {"pressure_kpa": 145.0, "tread_inner_c": 92.0, "tread_center_c": 96.0, "tread_outer_c": 88.0, "carcass_c": 81.0, "gas_c": 73.0},
		"FR": {"pressure_kpa": 146.0, "tread_inner_c": 91.0, "tread_center_c": 95.0, "tread_outer_c": 87.0, "carcass_c": 80.0, "gas_c": 72.0},
		"RL": {"pressure_kpa": 140.0, "tread_inner_c": 90.0, "tread_center_c": 94.0, "tread_outer_c": 86.0, "carcass_c": 79.0, "gas_c": 71.0},
		"RR": {"pressure_kpa": 141.0, "tread_inner_c": 89.0, "tread_center_c": 93.0, "tread_outer_c": 85.0, "carcass_c": 78.0, "gas_c": 70.0},
	})

	var cells: Dictionary = panel.get("_cells")
	if cells.is_empty():
		failures.append("panel cells were not built")
	else:
		for wheel in ["FL", "FR", "RL", "RR"]:
			if not cells.has(wheel):
				failures.append("missing cell " + wheel)
				continue
			var cell: Dictionary = cells[wheel]
			var pressure_label: Label = cell["pressure"]
			var zones_label: Label = cell["zones"]
			var carcass_label: Label = cell["carcass"]
			if pressure_label.text.is_empty() or not pressure_label.text.contains("kPa"):
				failures.append(wheel + " pressure label missing: " + pressure_label.text)
			if not zones_label.text.contains("I ") or not zones_label.text.contains("C ") or not zones_label.text.contains("O "):
				failures.append(wheel + " zones label missing I/C/O: " + zones_label.text)
			if not carcass_label.text.contains("CAR") or not carcass_label.text.contains("GAS"):
				failures.append(wheel + " carcass/gas label missing: " + carcass_label.text)
		var fl_pressure: Label = cells["FL"]["pressure"]
		if not fl_pressure.text.contains("145"):
			failures.append("FL pressure label did not update: " + fl_pressure.text)

	panel.queue_free()
	if failures.is_empty():
		print("[PASS] TireStatusPanel renders per-wheel pressure + 5-node temperatures.")
	else:
		for failure in failures:
			printerr("[FAIL] " + failure)
	quit(failures.size())
