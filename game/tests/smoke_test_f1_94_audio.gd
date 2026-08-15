extends SceneTree

# Smoke: F1-94 decoupled audio integration inside the validated pipeline.
# Validates: VehicleAudioController exists, bank loads, surface mapping, mix.
const SCENE_PATH := "res://scenes/vehicles/f1_94/f1_94.tscn"
const BANK_DIR := "res://sounds/banks/v10_vehicle"
const EXPECTED_BANK_KEYS := ["engine_idle", "engine_mid", "engine_redline", "surf_sand", "surf_grass", "surf_rumble", "impact_barrier", "shift_up"]

func _fail(msg: String, failures: Array[String]) -> void:
	printerr("[FAIL] " + msg)
	failures.append(msg)

func _run() -> void:
	var failures: Array[String] = []
	var packed := load(SCENE_PATH) as PackedScene
	if packed == null:
		_fail("F1-94 scene could not load.", failures)
		quit(1)
		return
	var f194 := packed.instantiate()
	root.add_child(f194)
	for _i in 4:
		await process_frame
	var audio := f194.get_node_or_null("VehicleAudio") as Node
	if audio == null:
		_fail("VehicleAudio node missing under F194 (AUD-003).", failures)
	else:
		if audio.get("vehicle") == null:
			_fail("VehicleAudio.vehicle is not bound to VehicleRigidBody.", failures)
		# Bank load check (thin controller loads AudioStreamWAV per role).
		for k in EXPECTED_BANK_KEYS:
			if audio.get("_players") != null and not audio._players.has(k):
				# tolerate missing if bank not imported; but flag if manifest says should exist
				_fail("Audio bank key not loaded: " + k, failures)
		# Surface mapping smoke (pure logic, no physics needed)
		if audio.has_method("_surface_from_collider"):
			var fake_road := Node.new()
			fake_road.add_to_group("Road")
			if audio._surface_from_collider(fake_road) != "asphalt":
				_fail("surface mapping Road->asphalt failed.", failures)
			var fake_grass := Node.new()
			fake_grass.add_to_group("Grass")
			if audio._surface_from_collider(fake_grass) != "grass":
				_fail("surface Grass->grass failed.", failures)
			var fake_curb := Node.new()
			fake_curb.add_to_group("Curb")
			if audio._surface_from_collider(fake_curb) != "rumble":
				_fail("surface Curb->rumble failed.", failures)
			fake_road.free()
			fake_grass.free()
			fake_curb.free()
		# Mix weights sanity
		if audio.has_method("_band_weights"):
			var idle_weights = audio._band_weights(0.0)
			if idle_weights[0] < 0.9:
				_fail("band weights at idle should favour engine_idle.", failures)
			var red_weights = audio._band_weights(1.0)
			if red_weights[4] < 0.9:
				_fail("band weights at redline should favour engine_redline.", failures)
		# Drive one update tick through the telemetry path (headless-safe):
		# at rest the idle band must receive positive gain.
		audio.update(1.0 / 120.0)
		var idle_player: AudioStreamPlayer = audio._players.get("engine_idle")
		if idle_player == null or idle_player.volume_db <= -70.0:
			_fail("engine_idle loop did not receive positive gain after update tick.", failures)
		# Pitch tracking: at rest rpm~4500, idle native 3941 => pitch ~1.14; the
		# engine tone must follow the tach, so assert it's not stuck at 1.0 native.
		var vehicle_rpm: float = audio.vehicle.get("motor_rpm")
		if vehicle_rpm > 0.0:
			var expected_pitch := clampf(vehicle_rpm / 3941.0, 0.5, 3.5)
			if absf(idle_player.pitch_scale - expected_pitch) > 0.1:
				_fail("engine_idle pitch_scale=%.3f should be %.3f (rpm=%.0f)" % [idle_player.pitch_scale, expected_pitch, vehicle_rpm], failures)
		# Regression guard (mirrors Rust native_rpm_are_measured_not_laddered):
		# high/redline were once cherry-picked to 11208/15342 (2x too high).
		# Override RPM to each band's honest native and expect pitch_scale ~1.0.
		for band_rpm in [5580.0, 7687.0]:
			audio.vehicle.set("motor_rpm", band_rpm)
			audio.update(1.0 / 120.0)
			var band_key := "engine_high" if band_rpm == 5580.0 else "engine_redline"
			var band_player: AudioStreamPlayer = audio._players.get(band_key)
			if band_player == null:
				_fail("pitch band player missing: " + band_key, failures)
			elif absf(band_player.pitch_scale - 1.0) > 0.1:
				_fail("%s pitch_scale=%.3f should be ~1.0 at native rpm=%.0f" % [band_key, band_player.pitch_scale, band_rpm], failures)
		# Loop integrity: loop_end must cover the WHOLE stream in frames
		# (get_length()*mix_rate), not a data-size-derived value (QOA/ADPCM
		# streams have compressed data where data.size() is NOT frames).
		for k in ["engine_idle", "engine_redline", "surf_rumble"]:
			var lp: AudioStreamPlayer = audio._players.get(k)
			if lp == null or lp.stream == null:
				_fail("loop stream missing: " + k, failures)
				continue
			var wav_stream := lp.stream as AudioStreamWAV
			var expected_frames := int(wav_stream.get_length() * wav_stream.mix_rate)
			if wav_stream.loop_end != expected_frames:
				_fail("%s loop_end=%d should be %d frames (full stream)" % [k, wav_stream.loop_end, expected_frames], failures)
			if wav_stream.loop_end < expected_frames * 0.9:
				_fail("%s loop_end truncates the loop (%.1f%% of stream)" % [k, 100.0 * wav_stream.loop_end / float(expected_frames)], failures)
	f194.queue_free()
	if failures.is_empty():
		print("[PASS] F1-94 audio integration present (VehicleAudio + bank keys + surface/map).")
		quit(0)
	else:
		quit(failures.size())

func _init() -> void:
	call_deferred("_run")
