#pragma once

#include "formula90s/audio/engine_audio_config.hpp"
#include "formula90s/audio/engine_audio_mixer.hpp"
#include "formula90s/vehicle/vehicle_state_reader.hpp"
#include <godot_cpp/classes/audio_stream_generator.hpp>
#include <godot_cpp/classes/audio_stream_generator_playback.hpp>
#include <godot_cpp/classes/audio_stream_player3d.hpp>
#include <godot_cpp/classes/node.hpp>
#include <vector>

namespace godot {

class EngineAudioController : public Node {
	GDCLASS(EngineAudioController, Node)

	Ref<EngineAudioConfig> config;
	Ref<AudioStreamGenerator> generator;
	Ref<AudioStreamGeneratorPlayback> playback;
	AudioStreamPlayer3D *player = nullptr;
	VehicleStateReader car_state;
	formula90s::audio::EngineAudioMixer mixer;
	int previous_gear = 1;
	NodePath car_path = "..";

	bool load_sample(const String &path, std::vector<float> &target);
	void fill_audio_buffer();
	bool load_all_samples();
	void create_audio_nodes();

protected:
	static void _bind_methods();

public:
	void _ready() override;
	void _exit_tree() override;
	void _process(double delta) override;
	void set_engine_state(double p_rpm, double p_load);
	void notify_gear_shift(int gear);
	void set_config(const Ref<EngineAudioConfig> &value) { config = value; }
	Ref<EngineAudioConfig> get_config() const { return config; }
	void set_car_path(const NodePath &p) { car_path = p; }
	NodePath get_car_path() const { return car_path; }
};

} // namespace godot
