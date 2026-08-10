#include "formula90s/audio/engine_audio_mixer.hpp"
#include <algorithm>
#include <cmath>

namespace formula90s::audio {

void EngineAudioMixer::set_sample_rate(float value) {
	sample_rate = value;
	dsp.set_sample_rate(sample_rate);
}

void EngineAudioMixer::set_engine_layer(std::size_t index, std::vector<float> samples) {
	if (index >= engine_layers.size()) return;
	engine_layers[index].samples = std::move(samples);
	engine_layers[index].cursor = 0.0;
}

void EngineAudioMixer::set_gear_up(std::vector<float> samples) {
	gear_up.samples = std::move(samples);
	gear_up.cursor = 0.0;
}

void EngineAudioMixer::set_gear_down(std::vector<float> samples) {
	gear_down.samples = std::move(samples);
	gear_down.cursor = 0.0;
}

void EngineAudioMixer::start_shift(bool upshift) {
	active_shift = upshift ? 1 : -1;
	shift_cursor = 0;
}

float EngineAudioMixer::read_looped(SampleLayer &layer, double ratio) {
	if (layer.samples.empty()) return 0.0F;
	const std::size_t a = static_cast<std::size_t>(layer.cursor) % layer.samples.size();
	const std::size_t b = (a + 1) % layer.samples.size();
	const float fraction = static_cast<float>(layer.cursor - std::floor(layer.cursor));
	const float sample = layer.samples[a] + (layer.samples[b] - layer.samples[a]) * fraction;
	layer.cursor += ratio;
	if (layer.cursor >= static_cast<double>(layer.samples.size()))
		layer.cursor = std::fmod(layer.cursor, static_cast<double>(layer.samples.size()));
	return sample;
}

float EngineAudioMixer::read_shift(float shift_gain) {
	SampleLayer *layer = active_shift > 0 ? &gear_up : active_shift < 0 ? &gear_down : nullptr;
	if (!layer || shift_cursor >= layer->samples.size()) {
		active_shift = 0;
		return 0.0F;
	}
	return layer->samples[shift_cursor++] * shift_gain;
}

float EngineAudioMixer::mix_next(const EngineDspState &state, const EngineAudioMixConfig &config, int frame_index) {
	const auto weights = EngineLayerMixer::weights(state.normalized_rpm);
	const float ratio = EnginePitchProcessor::ratio(
		state, static_cast<float>(config.pitch_minimum), static_cast<float>(config.pitch_maximum));
	const float target_gain = static_cast<float>(config.coast_gain + state.throttle * config.throttle_gain);
	const double smoothing_seconds = target_gain > smoothed_gain ? config.attack_seconds : config.release_seconds;
	const float smoothing = static_cast<float>(
		1.0 - std::exp(-1.0 / (static_cast<double>(sample_rate) * std::max(smoothing_seconds, 0.001))));
	smoothed_gain += (target_gain - smoothed_gain) * smoothing;
	float mixed = 0.0F;
	for (std::size_t layer = 0; layer < engine_layers.size(); ++layer)
		mixed += read_looped(engine_layers[layer], ratio) * weights[layer];
	if (state.rev_cut && ((frame_index / 96) & 1) != 0) mixed *= 0.3F;
	mixed = mixed * smoothed_gain + read_shift(static_cast<float>(config.shift_gain));
	return dsp.process(mixed, state, static_cast<float>(config.saturation), static_cast<float>(config.limiter_threshold));
}

} // namespace formula90s::audio
