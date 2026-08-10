#pragma once

#include "formula90s/audio/engine_dsp.hpp"
#include <array>
#include <cstddef>
#include <vector>

namespace formula90s::audio {

struct EngineAudioMixConfig {
	double pitch_minimum = 0.7;
	double pitch_maximum = 1.4;
	double coast_gain = 0.2;
	double throttle_gain = 0.8;
	double attack_seconds = 0.02;
	double release_seconds = 0.08;
	double saturation = 0.12;
	double limiter_threshold = 0.92;
	double shift_gain = 0.7;
};

// Stateful PCM mixing only: this class has no Godot scene-node ownership.
class EngineAudioMixer {
	struct SampleLayer {
		std::vector<float> samples;
		double cursor = 0.0;
	};

	std::array<SampleLayer, 5> engine_layers;
	SampleLayer gear_up;
	SampleLayer gear_down;
	EngineDspChain dsp;
	int active_shift = 0;
	std::size_t shift_cursor = 0;
	float smoothed_gain = 0.0F;
	float sample_rate = 44100.0F;

	float read_looped(SampleLayer &layer, double ratio);
	float read_shift(float shift_gain);

public:
	void set_sample_rate(float value);
	void set_engine_layer(std::size_t index, std::vector<float> samples);
	void set_gear_up(std::vector<float> samples);
	void set_gear_down(std::vector<float> samples);
	void start_shift(bool upshift);
	float get_sample_rate() const { return sample_rate; }
	float mix_next(const EngineDspState &state, const EngineAudioMixConfig &config, int frame_index);
};

} // namespace formula90s::audio
