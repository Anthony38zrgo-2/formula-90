#pragma once

#include "formula90s/audio/engine_audio_config.hpp"
#include "formula90s/audio/engine_dsp.hpp"
#include "formula90s/vehicle/vehicle_adapter.hpp"
#include <godot_cpp/classes/audio_stream_generator.hpp>
#include <godot_cpp/classes/audio_stream_generator_playback.hpp>
#include <godot_cpp/classes/audio_stream_player3d.hpp>
#include <godot_cpp/classes/node.hpp>
#include <array>
#include <vector>

namespace godot {

class EngineAudioController : public Node {
	GDCLASS(EngineAudioController, Node)

	struct SampleLayer { std::vector<float> samples; double cursor = 0.0; };
	Ref<EngineAudioConfig> config;
	Ref<AudioStreamGenerator> generator;
	Ref<AudioStreamGeneratorPlayback> playback;
	AudioStreamPlayer3D *player = nullptr;
	VehicleAdapter car_adapter;
	std::array<SampleLayer, 5> engine_layers;
	SampleLayer gear_up, gear_down;
	formula90s::audio::EngineDspChain dsp;
	int previous_gear = 1;
	int active_shift = 0;
	std::size_t shift_cursor = 0;
	float smoothed_gain = 0.0F;
	float sample_rate = 44100.0F;
	NodePath car_path = "..";

	bool load_sample(const String &path, SampleLayer &target);
	float read_looped(SampleLayer &layer, double ratio);
	float read_shift();
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
