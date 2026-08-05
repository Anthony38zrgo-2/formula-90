#include "formula90s/audio/engine_audio_config.hpp"
#include <godot_cpp/core/class_db.hpp>

using namespace godot;

void EngineAudioConfig::_bind_methods() {
#define BIND_AUDIO_PROPERTY(name) ClassDB::bind_method(D_METHOD("set_" #name, "value"), &EngineAudioConfig::set_##name); ClassDB::bind_method(D_METHOD("get_" #name), &EngineAudioConfig::get_##name)
    BIND_AUDIO_PROPERTY(bank_path); BIND_AUDIO_PROPERTY(idle_rpm); BIND_AUDIO_PROPERTY(maximum_rpm);
    BIND_AUDIO_PROPERTY(pitch_minimum); BIND_AUDIO_PROPERTY(pitch_maximum); BIND_AUDIO_PROPERTY(throttle_gain);
    BIND_AUDIO_PROPERTY(coast_gain); BIND_AUDIO_PROPERTY(saturation); BIND_AUDIO_PROPERTY(limiter_threshold);
    BIND_AUDIO_PROPERTY(shift_gain); BIND_AUDIO_PROPERTY(attack_seconds); BIND_AUDIO_PROPERTY(release_seconds);
#undef BIND_AUDIO_PROPERTY
#define ADD_AUDIO_PROPERTY(type, name) ADD_PROPERTY(PropertyInfo(type, #name), "set_" #name, "get_" #name)
    ADD_AUDIO_PROPERTY(Variant::STRING, bank_path); ADD_AUDIO_PROPERTY(Variant::FLOAT, idle_rpm); ADD_AUDIO_PROPERTY(Variant::FLOAT, maximum_rpm);
    ADD_AUDIO_PROPERTY(Variant::FLOAT, pitch_minimum); ADD_AUDIO_PROPERTY(Variant::FLOAT, pitch_maximum); ADD_AUDIO_PROPERTY(Variant::FLOAT, throttle_gain);
    ADD_AUDIO_PROPERTY(Variant::FLOAT, coast_gain); ADD_AUDIO_PROPERTY(Variant::FLOAT, saturation); ADD_AUDIO_PROPERTY(Variant::FLOAT, limiter_threshold);
    ADD_AUDIO_PROPERTY(Variant::FLOAT, shift_gain); ADD_AUDIO_PROPERTY(Variant::FLOAT, attack_seconds); ADD_AUDIO_PROPERTY(Variant::FLOAT, release_seconds);
#undef ADD_AUDIO_PROPERTY
}
