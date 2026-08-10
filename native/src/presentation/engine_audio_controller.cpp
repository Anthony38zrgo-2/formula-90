#include "formula90s/presentation/engine_audio_controller.hpp"
#include <godot_cpp/classes/audio_stream_playback.hpp>
#include <godot_cpp/classes/audio_stream_wav.hpp>
#include <godot_cpp/classes/audio_server.hpp>
#include <godot_cpp/classes/resource_loader.hpp>
#include <godot_cpp/classes/os.hpp>
#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/variant/utility_functions.hpp>
#include <algorithm>
#include <array>
#include <vector>

using namespace godot;
using namespace formula90s::audio;

void EngineAudioController::_bind_methods() {
	ClassDB::bind_method(D_METHOD("set_engine_state", "rpm", "load"), &EngineAudioController::set_engine_state);
	ClassDB::bind_method(D_METHOD("notify_gear_shift", "gear"), &EngineAudioController::notify_gear_shift);
	ClassDB::bind_method(D_METHOD("set_config", "config"), &EngineAudioController::set_config);
	ClassDB::bind_method(D_METHOD("get_config"), &EngineAudioController::get_config);
	ADD_PROPERTY(PropertyInfo(Variant::OBJECT, "config", PROPERTY_HINT_RESOURCE_TYPE, "EngineAudioConfig"), "set_config", "get_config");
	ClassDB::bind_method(D_METHOD("set_car_path", "path"), &EngineAudioController::set_car_path);
	ClassDB::bind_method(D_METHOD("get_car_path"), &EngineAudioController::get_car_path);
	ADD_PROPERTY(PropertyInfo(Variant::NODE_PATH, "car_path"), "set_car_path", "get_car_path");
	ADD_SIGNAL(MethodInfo("gear_shift", PropertyInfo(Variant::INT, "gear")));
}

bool EngineAudioController::load_sample(const String &path, std::vector<float> &target) {
	Ref<AudioStreamWAV> wav = ResourceLoader::get_singleton()->load(path);
	if (wav.is_null() || wav->get_format() != AudioStreamWAV::FORMAT_16_BITS || wav->is_stereo()) {
		UtilityFunctions::push_error("V10 audio must be mono PCM16: ", path);
		return false;
	}
	const float rate = static_cast<float>(wav->get_mix_rate());
	if (rate != 44100.0F && rate != 48000.0F) {
		UtilityFunctions::push_error("V10 audio must be 44.1 or 48 kHz: ", path);
		return false;
	}
	mixer.set_sample_rate(rate);
	const PackedByteArray data = wav->get_data();
	target.resize(static_cast<std::size_t>(data.size() / 2));
	for (int64_t i = 0; i + 1 < data.size(); i += 2) {
		const uint16_t bits = static_cast<uint16_t>(data[i]) | (static_cast<uint16_t>(data[i + 1]) << 8U);
		target[static_cast<std::size_t>(i / 2)] = static_cast<float>(static_cast<int16_t>(bits)) / 32768.0F;
	}
	return !target.empty();
}

bool EngineAudioController::load_all_samples() {
	if (config.is_null()) {
		UtilityFunctions::push_error("EngineAudioController requires EngineAudioConfig");
		return false;
	}
	const String root = config->get_bank_path().trim_suffix("/") + "/";
	constexpr std::array<const char *, 5> names{"engine_idle.wav", "engine_low.wav", "engine_mid.wav", "engine_high.wav", "engine_redline.wav"};
	bool valid = true;
	for (std::size_t i = 0; i < names.size(); ++i) {
		std::vector<float> samples;
		valid = load_sample(root + String(names[i]), samples) && valid;
		mixer.set_engine_layer(i, std::move(samples));
	}
	std::vector<float> up_samples;
	valid = load_sample(root + String("gear_up.wav"), up_samples) && valid;
	mixer.set_gear_up(std::move(up_samples));
	std::vector<float> down_samples;
	valid = load_sample(root + String("gear_down.wav"), down_samples) && valid;
	mixer.set_gear_down(std::move(down_samples));
	return valid;
}

void EngineAudioController::create_audio_nodes() {
	if (OS::get_singleton()->has_feature("headless") || AudioServer::get_singleton()->get_driver_name() == "Dummy") return;
	generator.instantiate();
	generator->set_mix_rate_mode(AudioStreamGenerator::MIX_RATE_CUSTOM);
	generator->set_mix_rate(mixer.get_sample_rate());
	generator->set_buffer_length(0.12F);
	player = memnew(AudioStreamPlayer3D);
	add_child(player);
	player->set_stream(generator);
	player->set_unit_size(8.0F);
	player->play();
	playback = player->get_stream_playback();
	fill_audio_buffer();
}

void EngineAudioController::_ready() {
	set_process(true);
	car_state = VehicleStateReader(get_node_or_null(car_path));
	if (!car_state.is_valid()) {
		UtilityFunctions::push_error("EngineAudioController: car not found at ", car_path);
		return;
	}
	if (!load_all_samples()) return;
	create_audio_nodes();
	previous_gear = car_state.get_current_gear();
}

void EngineAudioController::fill_audio_buffer() {
	if (playback.is_null() || !car_state.is_valid() || config.is_null()) return;
	EngineDspState state;
	state.rpm = car_state.get_motor_rpm();
	state.normalized_rpm = std::clamp(
		(state.rpm - config->get_idle_rpm()) / (config->get_maximum_rpm() - config->get_idle_rpm()), 0.0, 1.0);
	state.throttle = car_state.get_throttle_amount();
	state.speed_kph = car_state.get_speed() * 3.6;
	state.gear = car_state.get_current_gear();
	state.reverse = state.gear < 0;
	state.rev_cut = state.rpm >= config->get_maximum_rpm() * 0.995;
	const EngineAudioMixConfig mix_config{
		config->get_pitch_minimum(), config->get_pitch_maximum(), config->get_coast_gain(),
		config->get_throttle_gain(), config->get_attack_seconds(), config->get_release_seconds(),
		config->get_saturation(), config->get_limiter_threshold(), config->get_shift_gain()
	};
	int frames = playback->get_frames_available();
	for (int i = 0; i < frames; ++i) {
		const float output = mixer.mix_next(state, mix_config, i);
		playback->push_frame(Vector2(output, output));
	}
}

void EngineAudioController::_process(double) {
	if (car_state.is_valid()) {
		const int gear = car_state.get_current_gear();
		if (gear != previous_gear) { notify_gear_shift(gear); previous_gear = gear; }
	}
	fill_audio_buffer();
}

void EngineAudioController::_exit_tree() {
	if (player) {
		player->stop();
		playback.unref();
		player->set_stream(Ref<AudioStream>());
		if (player->get_parent() == this) remove_child(player);
		memdelete(player);
	}
	generator.unref();
	player = nullptr;
}

void EngineAudioController::set_engine_state(double, double) { fill_audio_buffer(); }

void EngineAudioController::notify_gear_shift(int gear) {
	mixer.start_shift(gear > previous_gear);
	emit_signal("gear_shift", gear);
}
