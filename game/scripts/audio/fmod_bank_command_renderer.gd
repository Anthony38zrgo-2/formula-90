## Thin Godot renderer for Rust-authoritative AudioCommandFrame v2.
## No bank logic belongs here: it only creates/updates/stops sample players.
extends Node

@export var core_path: NodePath = NodePath("../F90Core")
@export var bus: StringName = &"Vehicle"
@export_range(0.005, 0.25, 0.005) var gain_smoothing_tau := 0.035
@export_range(0.005, 0.25, 0.005) var pitch_smoothing_tau := 0.045
@export_range(4, 64, 1) var max_continuous_voices := 48
@export_range(1, 32, 1) var max_one_shots := 12
## Diagnostic A/B mute mask (handoff section 7 paso 2): bit0=engine_int,
## bit1=engine_ext, bit2=transmission, bit3=wind, bit4=wheel, bit5=surfaces,
## bit6=collisions, bit7=ambience. Muted families keep voices alive at -80 dB.
@export_range(0, 255, 1) var debug_mute_mask := 0
## Optional per-command capture log (absolute or user:// path), one line per
## event: tick|ms|kind|detail. Leave empty to disable.
@export var debug_capture_path := ""
## Optional per-tick telemetry record (absolute or user:// path, CSV) for the
## offline A/B replay (Fase 6): same driving data rendered through both
## backends by vehicle-audio-engine/examples/live_replay.rs.
@export var debug_record_path := ""

var _core: Node
var _voices: Dictionary = {}
var _last_tick := -1
var _last_shot_id := 0
var _target_db: Dictionary = {}
var _current_db: Dictionary = {}
var _target_pitch: Dictionary = {}
var _current_pitch: Dictionary = {}
var _restart_on_finish: Dictionary = {}
var _pending_stop: Dictionary = {}
var _active_one_shots := 0
var _capture: FileAccess = null
var _did_print_backend := false
var _listener_distance_cache := 0.0
var _warned_no_listener := false
var _last_info_print_ms := 0

func _ready() -> void:
	if AudioServer.get_bus_index(bus) < 0:
		AudioServer.add_bus()
		AudioServer.set_bus_name(AudioServer.bus_count - 1, bus)
	var bus_index := AudioServer.get_bus_index(bus)
	var has_limiter := false
	for effect_index in AudioServer.get_bus_effect_count(bus_index):
		if AudioServer.get_bus_effect(bus_index, effect_index) is AudioEffectLimiter:
			has_limiter = true
			break
	if not has_limiter:
		var limiter := AudioEffectLimiter.new()
		limiter.threshold_db = -3.0
		limiter.ceiling_db = -0.5
		AudioServer.add_bus_effect(bus_index, limiter)
	_core = get_node_or_null(core_path)
	if _core != null and _core.has_method("set_audio_mute_mask") and debug_mute_mask != 0:
		_core.set_audio_mute_mask(debug_mute_mask)
	if _core != null and _core.has_method("set_audio_record_path") and not debug_record_path.is_empty():
		_core.set_audio_record_path(debug_record_path)
	if not debug_capture_path.is_empty():
		_capture = FileAccess.open(debug_capture_path, FileAccess.WRITE)
		if _capture != null:
			_capture.store_line("tick|ms|kind|detail")
		else:
			push_warning("AudioCommandRenderer: cannot open capture path %s" % debug_capture_path)
	set_process(_core != null)

func _process(delta: float) -> void:
	if _core == null or not _core.has_method("get_audio_command_frame"):
		return
	_update_listener_fact()
	var parsed: Variant = JSON.parse_string(_core.get_audio_command_frame())
	if not parsed is Dictionary or int(parsed.get("schema_version", 0)) not in [1, 2]:
		return
	var tick := int(parsed.get("tick", -1))
	if not _did_print_backend:
		_did_print_backend = true
		print("[AudioCommandRenderer] backend=%s schema=%s tick=%d mutes=%d capture=%s" % [
			str(parsed.get("backend", "?")), str(parsed.get("schema_version", "?")),
			tick, debug_mute_mask, debug_capture_path])
	if tick != _last_tick:
		_last_tick = tick
		_capture_tick(tick, parsed)
		for id: Variant in parsed.get("stops", []):
			_stop_voice(str(id))
		for command: Variant in parsed.get("voices", []):
			if command is Dictionary:
				_apply_voice(command)
		for command: Variant in parsed.get("one_shots", []):
			if command is Dictionary and int(command.get("id", 0)) > _last_shot_id:
				# Advance the dedupe mark only when the shot was actually played:
				# the Rust journal replays every shot for 120 ticks, so a shot
				# refused by the budget here is retried on the next frame.
				if _play_one_shot(command):
					_last_shot_id = int(command.id)
	_smooth_voices(delta)
	var now_ms := Time.get_ticks_msec()
	if now_ms - _last_info_print_ms >= 2000:
		_last_info_print_ms = now_ms
		print("[AudioCommandRenderer] tick=%d voices=%d one_shots=%d listener_distance=%.2f" % [
			_last_tick, _voices.size(), _active_one_shots, _listener_distance_cache])

func _apply_voice(command: Dictionary) -> void:
	var id := str(command.get("id", ""))
	var sample := str(command.get("sample", ""))
	if id.is_empty() or sample.is_empty():
		return
	var player: Node = _voices.get(id)
	if player == null:
		if _voices.size() >= max_continuous_voices:
			push_warning("Audio continuous voice budget exhausted; rejected %s" % id)
			return
		var source_stream := load(sample) as AudioStream
		if source_stream == null:
			push_warning("Audio command sample unavailable: %s" % sample)
			return
		var stream := source_stream.duplicate() as AudioStream
		player = _create_player(str(command.get("emitter", "cockpit")))
		player.name = "Voice_" + id.validate_node_name()
		player.set("bus", bus)
		var looped := bool(command.get("looped", true))
		if stream is AudioStreamWAV:
			stream.loop_mode = AudioStreamWAV.LOOP_FORWARD if looped else AudioStreamWAV.LOOP_DISABLED
			if looped and command.get("loop_begin_s") != null:
				stream.loop_begin = int(float(command.loop_begin_s) * stream.mix_rate)
			if looped and command.get("loop_end_s") != null:
				stream.loop_end = int(float(command.loop_end_s) * stream.mix_rate)
		elif stream is AudioStreamOggVorbis:
			stream.loop = looped
		player.set("stream", stream)
		if player.get_parent() == null:
			add_child(player)
		_voices[id] = player
		_current_db[id] = -80.0
		_target_db[id] = -80.0
		var initial_pitch := pow(2.0, float(command.get("pitch_semitones", 0.0)) / 12.0)
		_current_pitch[id] = initial_pitch
		_target_pitch[id] = initial_pitch
		_restart_on_finish[id] = bool(command.get("restart_on_finish", false))
		_pending_stop.erase(id)
		player.set("volume_db", -80.0)
		player.call("play")
	_target_db[id] = float(command.get("gain_db", -80.0))
	_target_pitch[id] = pow(2.0, float(command.get("pitch_semitones", 0.0)) / 12.0)
	_pending_stop.erase(id)
	if not bool(player.get("playing")) and bool(_restart_on_finish.get(id, false)):
		player.call("play")

func _capture_tick(tick: int, parsed: Dictionary) -> void:
	if _capture == null:
		return
	var now_ms := Time.get_ticks_msec()
	for id: Variant in parsed.get("stops", []):
		_capture.store_line("%d|%d|STOP|%s" % [tick, now_ms, str(id)])
	for command: Variant in parsed.get("voices", []):
		if command is Dictionary:
			_capture.store_line("%d|%d|V|%s|%s|%s|%.2f|%.2f|%s|%s|%s|%s|%s" % [
				tick, now_ms,
				str(command.get("id", "")), str(command.get("event", "")),
				str(command.get("sample", "")), float(command.get("gain_db", -80.0)),
				float(command.get("pitch_semitones", 0.0)),
				str(bool(command.get("looped", true))),
				str(bool(command.get("restart_on_finish", false))),
				str(command.get("loop_begin_s", "")), str(command.get("loop_end_s", "")),
				str(command.get("emitter", ""))])
	for command: Variant in parsed.get("one_shots", []):
		if command is Dictionary:
			_capture.store_line("%d|%d|SHOT|%d|%s|%s|%.2f|%.2f|%s" % [
				tick, now_ms, int(command.get("id", 0)),
				str(command.get("event", "")), str(command.get("sample", "")),
				float(command.get("gain_db", 0.0)), float(command.get("pitch_semitones", 0.0)),
				str(command.get("emitter", ""))])

func _stop_voice(id: String) -> void:
	var player: Node = _voices.get(id)
	if player != null:
		_target_db[id] = -80.0
		_pending_stop[id] = true

func _play_one_shot(command: Dictionary) -> bool:
	if _active_one_shots >= max_one_shots:
		return false
	var stream := load(str(command.get("sample", ""))) as AudioStream
	if stream == null:
		return false
	var player := _create_player(str(command.get("emitter", "body")))
	player.set("bus", bus)
	player.set("stream", stream)
	player.set("volume_db", float(command.get("gain_db", 0.0)))
	player.set("pitch_scale", pow(2.0, float(command.get("pitch_semitones", 0.0)) / 12.0))
	if player.get_parent() == null:
		add_child(player)
	_active_one_shots += 1
	player.connect("finished", _on_one_shot_finished.bind(player))
	player.call("play")
	return true

func _on_one_shot_finished(player: Node) -> void:
	_active_one_shots = maxi(0, _active_one_shots - 1)
	player.queue_free()

func _smooth_voices(delta: float) -> void:
	var gain_factor := 1.0 - exp(-delta / gain_smoothing_tau)
	var pitch_factor := 1.0 - exp(-delta / pitch_smoothing_tau)
	for id: Variant in _voices.keys():
		var key := str(id)
		var player: Node = _voices[key]
		var db := float(_current_db.get(key, -80.0))
		db += (float(_target_db.get(key, -80.0)) - db) * gain_factor
		_current_db[key] = db
		player.set("volume_db", db)
		var pitch := float(_current_pitch.get(key, 1.0))
		pitch += (float(_target_pitch.get(key, 1.0)) - pitch) * pitch_factor
		_current_pitch[key] = pitch
		player.set("pitch_scale", pitch)
		if bool(_pending_stop.get(key, false)) and db <= -72.0:
			_free_voice(key)

func _free_voice(id: String) -> void:
	var player: Node = _voices.get(id)
	if player != null:
		player.call("stop")
		player.queue_free()
	_voices.erase(id)
	_target_db.erase(id)
	_current_db.erase(id)
	_target_pitch.erase(id)
	_current_pitch.erase(id)
	_restart_on_finish.erase(id)
	_pending_stop.erase(id)

func _create_player(emitter: String) -> Node:
	if emitter in ["cockpit", "world"]:
		return AudioStreamPlayer.new()
	var spatial := AudioStreamPlayer3D.new()
	spatial.max_distance = 45.0
	spatial.attenuation_model = AudioStreamPlayer3D.ATTENUATION_INVERSE_DISTANCE
	spatial.unit_size = 3.0
	match emitter:
		"wheels": spatial.position = Vector3(0.0, -0.25, 0.0)
		"underfloor": spatial.position = Vector3(0.0, -0.35, 0.0)
		"rear": spatial.position = Vector3(0.0, 0.0, 1.35)
		"front": spatial.position = Vector3(0.0, 0.0, -1.8)
		_: spatial.position = Vector3.ZERO
	var vehicle := _find_vehicle(get_tree().root)
	if vehicle != null:
		vehicle.add_child(spatial)
	return spatial

func _find_vehicle(node: Node) -> Node3D:
	if node is Node3D and node.get_class() == "F194RustVehicle":
		return node as Node3D
	for child in node.get_children():
		var found := _find_vehicle(child)
		if found != null:
			return found
	return null

func _update_listener_fact() -> void:
	if not _core.has_method("set_audio_listener_distance"):
		return
	var vehicle := _find_vehicle(get_tree().root)
	var camera := get_viewport().get_camera_3d()
	if camera == null:
		# The race camera lives in the World SubViewport, not the UI viewport.
		camera = _find_camera(get_tree().root)
	if vehicle != null and camera != null:
		_listener_distance_cache = vehicle.global_position.distance_to(camera.global_position)
		_core.set_audio_listener_distance(_listener_distance_cache)
	elif not _warned_no_listener:
		_warned_no_listener = true
		push_warning("AudioCommandRenderer: no camera/vehicle found; listener distance stays 0 (engine_ext silent)")

func _find_camera(node: Node) -> Camera3D:
	if node is Camera3D and (node as Camera3D).current:
		return node as Camera3D
	for child in node.get_children():
		var found := _find_camera(child)
		if found != null:
			return found
	return null

func _exit_tree() -> void:
	if _capture != null:
		_capture.flush()
		_capture = null
	for id: Variant in _voices.keys():
		_free_voice(str(id))
