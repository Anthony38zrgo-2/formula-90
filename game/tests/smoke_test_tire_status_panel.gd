extends SceneTree

# Verifies the TireStatusPanel renders per-wheel pressure + 5-node temperatures
# from the native/GDScript snapshot contract used by the HUD.

func _init() -> void:
	call_deferred("_run")


func _v3_brake_data() -> Dictionary:
	# Two-node compact brake contract (post-BRAKE-1000): disc + rim only.
	# Removed states (caliper_c, hub_c) are intentionally absent.
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
			if pressure_label.text.is_empty() or not pressure_label.text.contains("kPa"):
				failures.append(wheel + " pressure label missing: " + pressure_label.text)
			if not zones_label.text.contains("I ") or not zones_label.text.contains("C ") or not zones_label.text.contains("O "):
				failures.append(wheel + " zones label missing I/C/O: " + zones_label.text)
			if not carcass_label.text.contains("CAR") or not carcass_label.text.contains("GAS"):
				failures.append(wheel + " carcass/gas label missing: " + carcass_label.text)
			if not brake_label.text.contains("BRK") or not brake_label.text.contains("RIM"):
				failures.append(wheel + " brake label missing BRK/RIM: " + brake_label.text)
			if brake_label.text.contains("caliper"):
				failures.append(wheel + " brake label references removed caliper state: " + brake_label.text)
			if not wear_label.text.contains("WR") or not wear_label.text.contains("I") or not wear_label.text.contains("O"):
				failures.append(wheel + " wear label missing WR/I/C/O: " + wear_label.text)
		var fl_pressure: Label = cells["FL"]["pressure"]
		if not fl_pressure.text.contains("145"):
			failures.append("FL pressure label did not update: " + fl_pressure.text)
		var fl_brake: Label = cells["FL"]["brake"]
		if not fl_brake.text.contains("300") or not fl_brake.text.contains("250"):
			failures.append("FL brake label did not render two-node disc/rim: " + fl_brake.text)
		var fl_wear: Label = cells["FL"]["wear"]
		if not fl_wear.text.contains("57%") or not fl_wear.text.contains("I42") or not fl_wear.text.contains("O55"):
			failures.append("FL wear label did not render remaining/zones: " + fl_wear.text)
		var rl_wear: Label = cells["RL"]["wear"]
		if rl_wear.get_theme_color("font_color") != panel.get("_settings").wear_critical_color:
			failures.append("RL wear label did not switch to the critical color: " + rl_wear.text)

	# Transitional snapshot: rim_c/critical absent must not throw or render stale
	# caliper state (HUD-1205: no errors on absent legacy state).
	var transitional := _v3_brake_data()
	for wheel in ["FL", "FR", "RL", "RR"]:
		transitional[wheel]["rim_c"] = null
	panel.call("set_tire_data", tire_data, transitional)
	await process_frame
	for wheel in ["FL", "FR", "RL", "RR"]:
		var cell: Dictionary = cells[wheel]
		var brake_label: Label = cell["brake"]
		if not brake_label.text.contains("RIM---"):
			failures.append(wheel + " brake label without rim_c did not degrade: " + brake_label.text)
		if brake_label.text.contains("caliper"):
			failures.append(wheel + " brake label references removed caliper state: " + brake_label.text)

	panel.queue_free()
	if failures.is_empty():
		print("[PASS] TireStatusPanel renders per-wheel pressure + 5-node temperatures and the two-node brake contract.")
	else:
		for failure in failures:
			printerr("[FAIL] " + failure)
	quit(failures.size())
