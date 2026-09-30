extends SceneTree

func _init() -> void:
	call_deferred("_run")


func _v3_brake_data() -> Dictionary:
	var data := {}
	for wheel in ["FL", "FR", "RL", "RR"]:
		data[wheel] = {
			"disc_c": 300.0, "rim_c": 250.0, "efficiency": 0.9,
			"natural_cooling_w_k": 10.0, "speed_cooling_w_k": 4.0,
			"duct_mass_flow_kg_s": 0.3, "duct_drag_n": 12.0,
			"optimal_min_c": 400.0, "optimal_max_c": 800.0,
			"fade_start_c": 900.0, "critical_c": 1100.0,
		}
	return data


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

	var tire_data := {
		"FL": {"pressure_kpa": 145.0, "tread_inner_c": 92.0, "tread_center_c": 96.0, "tread_outer_c": 88.0, "carcass_c": 81.0, "gas_c": 73.0,
			"wear_inner_fraction": 0.42, "wear_center_fraction": 0.31, "wear_outer_fraction": 0.55, "wear_remaining_fraction": 0.57, "wear_grip_scale": 0.94},
		"FR": {"pressure_kpa": 146.0, "tread_inner_c": 91.0, "tread_center_c": 95.0, "tread_outer_c": 87.0, "carcass_c": 80.0, "gas_c": 72.0,
			"wear_inner_fraction": 0.40, "wear_center_fraction": 0.30, "wear_outer_fraction": 0.52, "wear_remaining_fraction": 0.59, "wear_grip_scale": 0.95},
		"RL": {"pressure_kpa": 140.0, "tread_inner_c": 90.0, "tread_center_c": 94.0, "tread_outer_c": 86.0, "carcass_c": 79.0, "gas_c": 71.0,
			"wear_inner_fraction": 0.70, "wear_center_fraction": 0.55, "wear_outer_fraction": 0.80, "wear_remaining_fraction": 0.25, "wear_grip_scale": 0.71},
		"RR": {"pressure_kpa": 141.0, "tread_inner_c": 89.0, "tread_center_c": 93.0, "tread_outer_c": 85.0, "carcass_c": 78.0, "gas_c": 70.0,
			"wear_inner_fraction": 0.68, "wear_center_fraction": 0.53, "wear_outer_fraction": 0.78, "wear_remaining_fraction": 0.34, "wear_grip_scale": 0.72},
	}
	panel.call("set_tire_data", tire_data, _v3_brake_data())

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
			var brake_label: Label = cell["brake"]
			var wear_label: Label = cell["wear"]
			if not pressure_label.text.ends_with(" kPa") or pressure_label.text.begins_with("P "):
				failures.append(wheel + " pressure format is incorrect: " + pressure_label.text)
			if zones_label.text.contains("I ") or zones_label.text.contains("C ") or zones_label.text.contains("O ") or not zones_label.text.ends_with("°"):
				failures.append(wheel + " tread temperature format is incorrect: " + zones_label.text)
			if not carcass_label.text.begins_with("CAR ") or carcass_label.text.contains("GAS"):
				failures.append(wheel + " carcass format is incorrect: " + carcass_label.text)
			if not brake_label.text.begins_with("BRK ") or brake_label.text.contains("RIM"):
				failures.append(wheel + " brake format is incorrect: " + brake_label.text)
			if not wear_label.text.ends_with("%") or wear_label.text.contains("WR"):
				failures.append(wheel + " wear format is incorrect: " + wear_label.text)
		var fl_pressure: Label = cells["FL"]["pressure"]
		if not fl_pressure.text.contains("145"):
			failures.append("FL pressure label did not update: " + fl_pressure.text)
		var fl_brake: Label = cells["FL"]["brake"]
		if fl_brake.text != "BRK 300°":
			failures.append("FL brake label did not render disc temperature: " + fl_brake.text)
		var fl_wear: Label = cells["FL"]["wear"]
		if fl_wear.text != "57%":
			failures.append("FL wear label did not render remaining wear: " + fl_wear.text)
		var front_left_tread_temperatures_label: Label = cells["FL"]["zones"]
		if front_left_tread_temperatures_label.text != "92 96 88°":
			failures.append("FL tread temperatures did not render in order: " + front_left_tread_temperatures_label.text)
		var rl_wear: Label = cells["RL"]["wear"]
		if rl_wear.get_theme_color("font_color") != panel.get("_settings").wear_critical_color:
			failures.append("RL wear label did not switch to the critical color: " + rl_wear.text)

	var transitional := _v3_brake_data()
	for wheel in ["FL", "FR", "RL", "RR"]:
		transitional[wheel]["rim_c"] = null
	panel.call("set_tire_data", tire_data, transitional)
	await process_frame
	for wheel in ["FL", "FR", "RL", "RR"]:
		var cell: Dictionary = cells[wheel]
		var brake_label: Label = cell["brake"]
		if brake_label.text != "BRK 300°":
			failures.append(wheel + " brake label depended on rim temperature: " + brake_label.text)

	for wheel in ["FL", "FR", "RL", "RR"]:
		transitional[wheel]["disc_c"] = null
	panel.call("set_tire_data", tire_data, transitional)
	await process_frame
	for wheel in ["FL", "FR", "RL", "RR"]:
		var cell: Dictionary = cells[wheel]
		var brake_label: Label = cell["brake"]
		if brake_label.text != "BRK --°":
			failures.append(wheel + " missing brake temperature did not show a placeholder: " + brake_label.text)

	panel.queue_free()
	if failures.is_empty():
		print("[PASS] TireStatusPanel renders the compact pressure, tread, carcass, brake, and wear readout.")
	else:
		for failure in failures:
			printerr("[FAIL] " + failure)
	quit(failures.size())
