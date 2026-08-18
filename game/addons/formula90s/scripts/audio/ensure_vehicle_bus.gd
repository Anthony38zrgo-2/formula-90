extends Node

## Ensures a dedicated "Vehicle" audio bus with a limiter exists at startup.
## The native VehicleAudioControllerNode routes its AudioStreamPlayer to bus="Vehicle";
## without this the bus falls back to Master (gain 1.0, no limiter) and transients can
## exceed 0 dBFS and hard-clip at the DAC. This recreates the safety net the GDScript
## VehicleAudioController used to install via _ensure_vehicle_bus().

func _ready() -> void:
	_ensure_vehicle_bus()


func _ensure_vehicle_bus() -> void:
	var idx := -1
	for i in AudioServer.bus_count:
		if AudioServer.get_bus_name(i) == "Vehicle":
			idx = i
			break

	if idx == -1:
		idx = AudioServer.bus_count
		AudioServer.add_bus(idx)
		AudioServer.set_bus_name(idx, "Vehicle")
		AudioServer.set_bus_send(idx, "Master")

	# Don't add a second limiter if the bus already has one.
	for e in AudioServer.get_bus_effect_count(idx):
		if AudioServer.get_bus_effect(idx, e) is AudioEffectLimiter:
			return

	var lim := AudioEffectLimiter.new()
	lim.threshold_db = 0.0
	lim.ceiling_db = -0.3
	AudioServer.add_bus_effect(idx, lim)
