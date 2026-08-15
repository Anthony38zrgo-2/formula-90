## VehicleAudioController — thin Godot glue for the pure-Rust audio core.
##
## This controller is deliberately thin: it reads GEVP telemetry (motor_rpm,
## throttle_amount, current_gear, speed, wheel slip) and drives AudioStreamWAV
## players per role. All mixing rules (5-band weights, surface beds, one-shot
## triggers) are owned by the Rust core at `game/audio/engine/`
## (`VehicleAudioState` / `VehicleSoundBank`), whose deterministic contract this
## file mirrors only as a presentation layer. No audio rules live in GDScript.
##
## Bank: game/sounds/banks/v10_vehicle (built by tools/audio/bank_generator.py).

extends Node

const BANK_DIR := "res://sounds/banks/v10_vehicle"

## GEVP Vehicle node (duck-typed: motor_rpm, throttle_amount, current_gear, speed).
@export var vehicle: Node

## RPM range used to normalize the engine bands (matches v10_vehicle sample set).
@export var idle_rpm := 1000.0
@export var max_rpm := 15000.0

# Engine band keys in increasing RPM order (must match Rust ENGINE_BAND_KEYS).
const ENGINE_BANDS := ["engine_idle", "engine_low", "engine_mid", "engine_high", "engine_redline"]
const ENGINE_BAND_NATIVE_RPM := [3941.0, 7429.0, 8196.0, 5580.0, 7687.0]
const BAND_CENTERS := [0.0, 0.25, 0.5, 0.75, 1.0]
const BAND_WIDTH := 0.25
const PITCH_MIN := 0.5
const PITCH_MAX := 3.5

# Surface token -> bank bed key (asphalt has no bed).
const SURFACE_KEYS := {
	"asphalt": null,
	"sand": "surf_sand",
	"grass": "surf_grass",
	"rumble": "surf_rumble",
}

# Current surface token, set by the scene (e.g. from generated_track_surface_groups).
var surface := "asphalt"

# One-shot triggers pending playback this frame.
var _pending_triggers: Array[String] = []

# role -> AudioStreamWAV.
var _players := {}

var _last_gear := 0
var _active_bed := ""
# Telemetry snapshot (updated each update(); read by audio_telemetry.gd)
var last_norm := 0.0
var last_weights: Array = [0.0, 0.0, 0.0, 0.0, 0.0]
var last_pitches: Array = [1.0, 1.0, 1.0, 1.0, 1.0]
var last_engine_gain := 0.0
var last_rpm := 0.0
var last_throttle := 0.0
var last_speed_kph := 0.0
var last_slip := 0.0
var last_trigger: String = ""


func _ready() -> void:
	_load_bank()


func _physics_process(_delta: float) -> void:
	update(_delta)


## Load every bank WAV into an AudioStreamWAV player, one per role.
## Prefiere el recurso importado (gestión correcta de mix_rate/bytes por Godot) y
## fija loop_end explícitamente para evitar el buzz de un loop degenerado.
func _load_bank() -> void:
	var dir := DirAccess.open(BANK_DIR)
	if dir == null:
		push_warning("VehicleAudioController: bank not found at %s" % BANK_DIR)
		return
	for file_name in dir.get_files():
		if not file_name.ends_with(".wav"):
			continue
		var key := file_name.get_basename()
		var res_path := BANK_DIR.path_join(file_name)
		var imported := load(res_path) as AudioStreamWAV
		var wav: AudioStreamWAV = null
		if imported != null and imported.data.size() > 0:
			wav = imported.duplicate() as AudioStreamWAV
		else:
			wav = AudioStreamWAV.new()
			wav.format = AudioStreamWAV.FORMAT_16_BITS
			wav.mix_rate = 44100
			wav.stereo = false
			var bytes := FileAccess.get_file_as_bytes(res_path)
			wav.data = _extract_pcm16(bytes)
		var is_loop := key in ENGINE_BANDS or key.begins_with("surf_")
		wav.loop_mode = AudioStreamWAV.LOOP_FORWARD if is_loop else AudioStreamWAV.LOOP_DISABLED
		if is_loop and wav.get_length() > 0.0:
			# loop_end está en SAMPLES (frames), no en bytes. Para streams QOA/ADPCM
			# data.size() NO es PCM: calcular frames desde la duración real.
			wav.loop_begin = 0
			wav.loop_end = int(wav.get_length() * wav.mix_rate)
		var player := AudioStreamPlayer.new()
		player.stream = wav
		add_child(player)
		_players[key] = player


## Extract PCM16 payload from a WAV file (RIFF/WAVE), mirroring the Rust bank loader.
func _extract_pcm16(bytes: PackedByteArray) -> PackedByteArray:
	if bytes.size() < 44:
		return PackedByteArray()
	var pos := 12
	while pos + 8 <= bytes.size():
		var size := bytes.decode_u32(pos + 4)
		var chunk := bytes.slice(pos, pos + 4).get_string_from_ascii()
		if chunk == "data":
			return bytes.slice(pos + 8, pos + 8 + size)
		pos += 8 + size + (size & 1)
	return PackedByteArray()


## Map a GEVP wheel surface_type and Godot collision groups to an audio surface.
func _surface_from_collider(collider: Object) -> String:
	if collider == null:
		return "asphalt"
	if collider.is_in_group("Grass"):
		return "grass"
	if collider.is_in_group("Curb"):
		return "rumble"
	if collider.is_in_group("Gravel") or collider.is_in_group("Dirt") or collider.is_in_group("Sand"):
		return "sand"
	if collider.is_in_group("Road"):
		return "asphalt"
	return "asphalt"

func _surface_from_wheel_type(st: String) -> String:
	match st:
		"Grass": return "grass"
		"Curb": return "rumble"
		"Gravel", "Dirt", "Sand": return "sand"
		_: return "asphalt"

## Detect the current surface preferring GEVP wheel.surface_type (autoridad física),
## con fallback a grupos de colisión del RayCast.
func _detect_surface() -> String:
	if vehicle == null:
		return "asphalt"
	var counts := {"asphalt": 0, "grass": 0, "sand": 0, "rumble": 0}
	# Fuente primaria: wheel.surface_type de GEVP (PhysicsBody wheels via axles).
	if vehicle.get("axles") != null:
		for axle in vehicle.axles:
			for wheel in axle.wheels:
				var st: String = wheel.get("surface_type")
				if st == null or st == "":
					continue
				var token := _surface_from_wheel_type(st)
				counts[token] = counts.get(token, 0) + 1
		if counts["rumble"] + counts["grass"] + counts["sand"] > 0:
			if counts["rumble"] > 0:
				return "rumble"
			if counts["grass"] > 0:
				return "grass"
			if counts["sand"] > 0:
				return "sand"
	# Fallback: muestreo del collider del RayCast.
	var wheels := ["WheelFrontLeft", "WheelFrontRight", "WheelRearLeft", "WheelRearRight"]
	for name in wheels:
		var ray: RayCast3D = vehicle.get_node_or_null(name)
		if ray == null or not ray.is_colliding():
			continue
		var token := _surface_from_collider(ray.get_collider())
		counts[token] = counts.get(token, 0) + 1
	if counts["rumble"] > 0:
		return "rumble"
	if counts["grass"] > 0:
		return "grass"
	if counts["sand"] > 0:
		return "sand"
	return "asphalt"

## Called each physics frame by the scene. `slip` in [0,1] (max abs wheel slip).
func update(_delta: float) -> void:
	surface = _detect_surface()
	if vehicle == null:
		return
	var rpm: float = vehicle.get("motor_rpm")
	var throttle: float = vehicle.get("throttle_amount")
	var gear: int = vehicle.get("current_gear")
	var speed_kph: float = vehicle.speed * 3.6
	# RPM range from the vehicle when exposed (F1-94: idle 4500 / max 17000),
	# falling back to the exported defaults.
	var vehicle_idle: float = idle_rpm
	var vehicle_max: float = max_rpm
	if vehicle.get("idle_rpm") != null:
		vehicle_idle = vehicle.get("idle_rpm")
	if vehicle.get("max_rpm") != null:
		vehicle_max = vehicle.get("max_rpm")

	# Engine band crossfade + pitch tracking (mirrors Rust engine_weights + pitch_scale).
	var norm := clampf((rpm - vehicle_idle) / (vehicle_max - vehicle_idle), 0.0, 1.0)
	var weights := _band_weights(norm)
	var engine_gain := 0.45 + 0.55 * throttle
	var pitches: Array = []
	for i in ENGINE_BANDS.size():
		pitches.append(clampf(rpm / ENGINE_BAND_NATIVE_RPM[i], PITCH_MIN, PITCH_MAX))
	# Snapshot for audio telemetry (before mutating players)
	last_norm = norm
	last_weights = weights.duplicate()
	last_pitches = pitches.duplicate()
	last_rpm = rpm
	last_throttle = throttle
	last_speed_kph = speed_kph
	last_engine_gain = engine_gain
	for i in ENGINE_BANDS.size():
		var key: String = ENGINE_BANDS[i]
		var player: AudioStreamPlayer = _players.get(key)
		if player != null:
			player.pitch_scale = pitches[i]
		_play_loop(key, weights[i], engine_gain)

	# Surface bed (asphalt none); silence any previous bed when it changes.
	var bed := SURFACE_KEYS.get(surface)
	var slip := _aggregate_slip()
	last_slip = slip
	if bed != null:
		var bed_gain := 0.25 + 0.55 * slip + 0.12 * minf(speed_kph / 120.0, 1.0)
		_play_loop(bed, 1.0, bed_gain)
	if bed != _active_bed:
		for key in SURFACE_KEYS.values():
			if key != null and key != bed:
				var stale: AudioStreamPlayer = _players.get(key)
				if stale != null and stale.playing:
					stale.volume_db = -80.0
		_active_bed = bed if bed != null else ""

	# Gear-change one-shots.
	var fired: String = ""
	if gear != _last_gear:
		if _last_gear != 0:
			fired = "shift_up" if gear > _last_gear else "shift_down"
			trigger(fired)
		_last_gear = gear
	last_trigger = fired

	# One-shots fired this frame.
	for t: String in _pending_triggers:
		_play_oneshot(t)
	_pending_triggers.clear()


## Fire a one-shot (bank key), e.g. "impact_barrier", "shift_up", "engine_backfire".
func trigger(key: String) -> void:
	if _players.has(key):
		_pending_triggers.append(key)


## Aggregate max abs longitudinal wheel slip across drive wheels (0..1).
func _aggregate_slip() -> float:
	if vehicle == null:
		return 0.0
	var slip := 0.0
	if vehicle.get("axles") != null:
		for axle in vehicle.axles:
			for wheel in axle.wheels:
				var sv: Vector2 = wheel.get("slip_vector")
				slip = maxf(slip, absf(sv.y))
	return clampf(slip, 0.0, 1.0)


## Triangular crossfade weights over the 5 engine bands (mirrors Rust core).
func _band_weights(norm: float) -> Array:
	var weights: Array = []
	var sum := 0.0
	for center in BAND_CENTERS:
		var w := maxf(0.0, 1.0 - absf(norm - center) / BAND_WIDTH)
		weights.append(w)
		sum += w
	if sum <= 0.0:
		return [1.0, 0.0, 0.0, 0.0, 0.0]
	for i in weights.size():
		weights[i] = weights[i] / sum
	return weights


## Play a looping role with weight (0..1) and an additional gain.
func _play_loop(key: String, weight: float, gain: float) -> void:
	var player: AudioStreamPlayer = _players.get(key)
	if player == null:
		return
	if weight <= 0.0:
		if player.playing:
			player.volume_db = -80.0
		return
	player.volume_db = linear_to_db(weight * gain)
	if not player.playing:
		player.play()


## Play a one-shot from the start.
func _play_oneshot(key: String) -> void:
	var player: AudioStreamPlayer = _players.get(key)
	if player == null:
		return
	player.volume_db = 0.0
	player.play()
