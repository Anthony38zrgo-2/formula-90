extends GdUnitTestSuite


func _pad_button_indices(action_name: String) -> Array[int]:
	var indices: Array[int] = []
	for event in InputMap.action_get_events(action_name):
		if event is InputEventJoypadButton:
			indices.append(event.button_index)
	return indices


func _pad_axis_events(action_name: String) -> Array[InputEventJoypadMotion]:
	var axes: Array[InputEventJoypadMotion] = []
	for event in InputMap.action_get_events(action_name):
		if event is InputEventJoypadMotion:
			axes.append(event)
	return axes


func _physical_key_codes(action_name: String) -> Array[int]:
	var codes: Array[int] = []
	for event in InputMap.action_get_events(action_name):
		if event is InputEventKey:
			codes.append(event.physical_keycode)
	return codes


func test_xbox_table_seven_assignations_are_live() -> void:
	assert_bool(_pad_button_indices("Shift Up").has(JOY_BUTTON_A)).is_true()
	assert_bool(_pad_button_indices("Shift Down").has(JOY_BUTTON_X)).is_true()
	assert_bool(_pad_button_indices("Pit Toggle Menu").has(JOY_BUTTON_Y)).is_true()
	assert_bool(_pad_button_indices("Pit Confirm").has(JOY_BUTTON_B)).is_true()
	assert_bool(_pad_axis_events("Throttle").any(func(axis_event): return axis_event.axis == 5 and axis_event.axis_value > 0.0)).is_true()
	assert_bool(_pad_axis_events("Brakes").any(func(axis_event): return axis_event.axis == 4 and axis_event.axis_value > 0.0)).is_true()
	assert_bool(_pad_axis_events("Steer Left").any(func(axis_event): return axis_event.axis == 0 and axis_event.axis_value < 0.0)).is_true()
	assert_bool(_pad_axis_events("Steer Right").any(func(axis_event): return axis_event.axis == 0 and axis_event.axis_value > 0.0)).is_true()


func test_displaced_gamepad_actions_no_longer_fire_their_old_buttons() -> void:
	assert_bool(_pad_button_indices("Handbrake").is_empty()).is_true()
	assert_bool(_pad_button_indices("Toggle Transmission").is_empty()).is_true()
	assert_bool(_pad_button_indices("Pit Confirm").has(JOY_BUTTON_A)).is_false()
	assert_bool(_pad_button_indices("Shift Up").has(JOY_BUTTON_X)).is_false()
	assert_bool(_pad_button_indices("Shift Down").has(JOY_BUTTON_Y)).is_false()
	assert_bool(_pad_button_indices("Pit Toggle Menu").has(JOY_BUTTON_B)).is_false()


func test_dpad_exclusively_navigates_the_pit_panel() -> void:
	assert_bool(_pad_button_indices("Pit Field Up").has(JOY_BUTTON_DPAD_UP)).is_true()
	assert_bool(_pad_button_indices("Pit Field Down").has(JOY_BUTTON_DPAD_DOWN)).is_true()
	assert_bool(_pad_button_indices("Pit Value Left").has(JOY_BUTTON_DPAD_LEFT)).is_true()
	assert_bool(_pad_button_indices("Pit Value Right").has(JOY_BUTTON_DPAD_RIGHT)).is_true()
	assert_bool(_pad_button_indices("ShowDebug").is_empty()).is_true()
	assert_bool(_pad_button_indices("DebugPrevious").is_empty()).is_true()
	assert_bool(_pad_button_indices("Toggle Traction Control").is_empty()).is_true()


func test_keyboard_bindings_survive_the_new_map() -> void:
	assert_bool(_physical_key_codes("Handbrake").has(KEY_SPACE)).is_true()
	assert_bool(_physical_key_codes("Toggle Transmission").has(KEY_DELETE)).is_true()
	assert_bool(_physical_key_codes("Toggle Traction Control").has(KEY_T)).is_true()
	assert_bool(_physical_key_codes("Throttle").has(KEY_UP)).is_true()
	assert_bool(_physical_key_codes("Brakes").has(KEY_DOWN)).is_true()
	assert_bool(_physical_key_codes("Steer Left").has(KEY_LEFT)).is_true()
	assert_bool(_physical_key_codes("Steer Right").has(KEY_RIGHT)).is_true()
	assert_bool(_physical_key_codes("Shift Up").has(KEY_A)).is_true()
	assert_bool(_physical_key_codes("Shift Down").has(KEY_Z)).is_true()
	assert_bool(_physical_key_codes("Pit Toggle Menu").has(KEY_B)).is_true()
	assert_bool(_physical_key_codes("Pit Confirm").has(KEY_ENTER)).is_true()


func test_action_deadzones_match_the_profile() -> void:
	assert_float(InputMap.action_get_deadzone("Steer Left")).is_between(0.049, 0.051)
	assert_float(InputMap.action_get_deadzone("Steer Right")).is_between(0.049, 0.051)
	assert_float(InputMap.action_get_deadzone("Throttle")).is_between(0.049, 0.051)
	assert_float(InputMap.action_get_deadzone("Brakes")).is_between(0.049, 0.051)
	assert_float(InputMap.action_get_deadzone("Pit Confirm")).is_between(0.19, 0.21)
