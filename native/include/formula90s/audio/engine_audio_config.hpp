#pragma once

#include <godot_cpp/classes/resource.hpp>

namespace godot {
class EngineAudioConfig : public Resource {
    GDCLASS(EngineAudioConfig, Resource)
    String bank_path = "res://assets/audio/engines/v10_prototype";
    double idle_rpm = 1100.0, maximum_rpm = 9000.0;
    double pitch_minimum = 0.72, pitch_maximum = 1.42;
    double throttle_gain = 0.45, coast_gain = 0.22, saturation = 0.12;
    double limiter_threshold = 0.92, shift_gain = 0.45, attack_seconds = 0.035, release_seconds = 0.12;
protected:
    static void _bind_methods();
public:
#define AUDIO_PROPERTY(type, name) void set_##name(type value) { name = value; } type get_##name() const { return name; }
    AUDIO_PROPERTY(String, bank_path)
    AUDIO_PROPERTY(double, idle_rpm)
    AUDIO_PROPERTY(double, maximum_rpm)
    AUDIO_PROPERTY(double, pitch_minimum)
    AUDIO_PROPERTY(double, pitch_maximum)
    AUDIO_PROPERTY(double, throttle_gain)
    AUDIO_PROPERTY(double, coast_gain)
    AUDIO_PROPERTY(double, saturation)
    AUDIO_PROPERTY(double, limiter_threshold)
    AUDIO_PROPERTY(double, shift_gain)
    AUDIO_PROPERTY(double, attack_seconds)
    AUDIO_PROPERTY(double, release_seconds)
#undef AUDIO_PROPERTY
};
}
