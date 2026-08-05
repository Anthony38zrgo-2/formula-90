#include "formula90s/presentation/engine_audio_controller.hpp"
#include "formula90s/vehicle/arcade_car_controller.hpp"
#include <godot_cpp/classes/audio_stream_playback.hpp>
#include <godot_cpp/classes/audio_stream_wav.hpp>
#include <godot_cpp/classes/audio_server.hpp>
#include <godot_cpp/classes/resource_loader.hpp>
#include <godot_cpp/classes/os.hpp>
#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/variant/utility_functions.hpp>
#include <algorithm>
#include <cmath>

using namespace godot;
using namespace formula90s::audio;

void EngineAudioController::_bind_methods() {
    ClassDB::bind_method(D_METHOD("set_engine_state", "rpm", "load"), &EngineAudioController::set_engine_state);
    ClassDB::bind_method(D_METHOD("notify_gear_shift", "gear"), &EngineAudioController::notify_gear_shift);
    ClassDB::bind_method(D_METHOD("set_config", "config"), &EngineAudioController::set_config);
    ClassDB::bind_method(D_METHOD("get_config"), &EngineAudioController::get_config);
    ADD_PROPERTY(PropertyInfo(Variant::OBJECT, "config", PROPERTY_HINT_RESOURCE_TYPE, "EngineAudioConfig"), "set_config", "get_config");
    ADD_SIGNAL(MethodInfo("gear_shift", PropertyInfo(Variant::INT, "gear")));
}

bool EngineAudioController::load_sample(const String &path, SampleLayer &target) {
    Ref<AudioStreamWAV> wav = ResourceLoader::get_singleton()->load(path);
    if (wav.is_null() || wav->get_format() != AudioStreamWAV::FORMAT_16_BITS || wav->is_stereo() || wav->get_mix_rate() != 44100) {
        UtilityFunctions::push_error("V10 audio must be mono 44.1 kHz PCM16: ", path);
        return false;
    }
    const PackedByteArray data = wav->get_data();
    target.samples.resize(static_cast<std::size_t>(data.size() / 2));
    for (int64_t i = 0; i + 1 < data.size(); i += 2) {
        const uint16_t bits = static_cast<uint16_t>(data[i]) | (static_cast<uint16_t>(data[i + 1]) << 8U);
        target.samples[static_cast<std::size_t>(i / 2)] = static_cast<float>(static_cast<int16_t>(bits)) / 32768.0F;
    }
    return !target.samples.empty();
}

void EngineAudioController::_ready() {
    set_process(true);
    car = Object::cast_to<ArcadeCarController>(get_parent());
    if (config.is_null()) {
        UtilityFunctions::push_error("EngineAudioController requires EngineAudioConfig");
        return;
    }
    const String root = config->get_bank_path().trim_suffix("/") + "/";
    constexpr std::array<const char *, 5> names{"engine_idle.wav", "engine_low.wav", "engine_mid.wav", "engine_high.wav", "engine_redline.wav"};
    bool valid = true;
    for (std::size_t i = 0; i < names.size(); ++i) valid = load_sample(root + String(names[i]), engine_layers[i]) && valid;
    valid = load_sample(root + String("gear_up.wav"), gear_up) && valid;
    valid = load_sample(root + String("gear_down.wav"), gear_down) && valid;
    if (!valid || !car) return;
    if (OS::get_singleton()->has_feature("headless") || AudioServer::get_singleton()->get_driver_name() == "Dummy") return;
    generator.instantiate();
    generator->set_mix_rate_mode(AudioStreamGenerator::MIX_RATE_CUSTOM);
    generator->set_mix_rate(44100.0F);
    generator->set_buffer_length(0.12F);
    player = memnew(AudioStreamPlayer3D);
    add_child(player);
    player->set_stream(generator);
    player->set_unit_size(8.0F);
    player->play();
    playback = player->get_stream_playback();
    dsp.set_sample_rate(44100.0F);
    previous_gear = car->get_gear();
    fill_audio_buffer();
}

float EngineAudioController::read_looped(SampleLayer &layer, double ratio) {
    if (layer.samples.empty()) return 0.0F;
    const std::size_t a = static_cast<std::size_t>(layer.cursor) % layer.samples.size();
    const std::size_t b = (a + 1) % layer.samples.size();
    const float fraction = static_cast<float>(layer.cursor - std::floor(layer.cursor));
    const float sample = layer.samples[a] + (layer.samples[b] - layer.samples[a]) * fraction;
    layer.cursor += ratio;
    while (layer.cursor >= static_cast<double>(layer.samples.size())) layer.cursor -= static_cast<double>(layer.samples.size());
    return sample;
}

float EngineAudioController::read_shift() {
    SampleLayer *layer = active_shift > 0 ? &gear_up : active_shift < 0 ? &gear_down : nullptr;
    if (!layer || shift_cursor >= layer->samples.size()) { active_shift = 0; return 0.0F; }
    return layer->samples[shift_cursor++] * static_cast<float>(config->get_shift_gain());
}

void EngineAudioController::fill_audio_buffer() {
    if (playback.is_null() || !car || config.is_null()) return;
    EngineDspState state;
    state.rpm = car->get_rpm();
    state.normalized_rpm = std::clamp((state.rpm - config->get_idle_rpm()) / (config->get_maximum_rpm() - config->get_idle_rpm()), 0.0, 1.0);
    state.throttle = car->get_throttle();
    state.speed_kph = car->get_speed_kph();
    state.gear = car->get_gear();
    state.reverse = state.gear < 0;
    state.rev_cut = state.rpm >= config->get_maximum_rpm() * 0.995;
    const auto weights = EngineLayerMixer::weights(state.normalized_rpm);
    const float ratio = EnginePitchProcessor::ratio(state, static_cast<float>(config->get_pitch_minimum()), static_cast<float>(config->get_pitch_maximum()));
    const float target_gain = static_cast<float>(config->get_coast_gain() + state.throttle * config->get_throttle_gain());
    const double smoothing_seconds = target_gain > smoothed_gain ? config->get_attack_seconds() : config->get_release_seconds();
    const float smoothing = static_cast<float>(1.0 - std::exp(-1.0 / (44100.0 * std::max(smoothing_seconds, 0.001))));
    int frames = playback->get_frames_available();
    for (int i = 0; i < frames; ++i) {
        smoothed_gain += (target_gain - smoothed_gain) * smoothing;
        float mixed = 0.0F;
        for (std::size_t layer = 0; layer < engine_layers.size(); ++layer) mixed += read_looped(engine_layers[layer], ratio) * weights[layer];
        if (state.rev_cut && ((i / 96) & 1) != 0) mixed *= 0.3F;
        mixed = mixed * smoothed_gain + read_shift();
        const float output = dsp.process(mixed, state, static_cast<float>(config->get_saturation()), static_cast<float>(config->get_limiter_threshold()));
        playback->push_frame(Vector2(output, output));
    }
}

void EngineAudioController::_process(double) {
    if (car) {
        const int gear = car->get_gear();
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
    active_shift = gear > previous_gear ? 1 : -1;
    shift_cursor = 0;
    emit_signal("gear_shift", gear);
}
