#pragma once

#include <algorithm>
#include <array>
#include <cmath>
#include <cstddef>
#include <numbers>

namespace formula90s::audio {

constexpr float FILTER_CUTOFF_BASE = 900.0F;
constexpr float FILTER_CUTOFF_RANGE = 9500.0F;
constexpr float TWO_PI_F = 2.0F * std::numbers::pi_v<float>;
constexpr float SATURATION_GAIN_FACTOR = 3.0F;
constexpr float LIMITER_MIN = 0.1F;
constexpr float LIMITER_MAX = 0.999F;

struct EngineDspState {
    double rpm = 1100.0;
    double normalized_rpm = 0.0;
    double throttle = 0.0;
    double speed_kph = 0.0;
    int gear = 1;
    bool reverse = false;
    bool rev_cut = false;
};

class EngineLayerMixer {
public:
    static std::array<float, 5> weights(double normalized_rpm) {
        const float x = static_cast<float>(std::clamp(normalized_rpm, 0.0, 1.0));
        constexpr std::array<float, 5> centers{0.0F, 0.25F, 0.5F, 0.75F, 1.0F};
        std::array<float, 5> result{};
        float sum = 0.0F;
        for (std::size_t i = 0; i < result.size(); ++i) {
            result[i] = std::max(0.0F, 1.0F - std::abs(x - centers[i]) / 0.25F);
            sum += result[i];
        }
        if (sum <= 0.0F) result[0] = 1.0F;
        else for (float &weight : result) weight /= sum;
        return result;
    }
};

class EnginePitchProcessor {
public:
    static float ratio(const EngineDspState &state, float minimum, float maximum) {
        const float base = minimum + static_cast<float>(state.normalized_rpm) * (maximum - minimum);
        const float reverse_factor = state.reverse ? 0.88F : 1.0F;
        return std::clamp(base * reverse_factor, minimum, maximum);
    }
};

class EngineFilterProcessor {
    float sample_rate_ = 44100.0F;
    float state_ = 0.0F;
public:
    void set_sample_rate(float sample_rate) { sample_rate_ = std::max(sample_rate, 8000.0F); }
    void reset() { state_ = 0.0F; }
    float process(float input, float load) {
        const float cutoff = FILTER_CUTOFF_BASE + std::clamp(load, 0.0F, 1.0F) * FILTER_CUTOFF_RANGE;
        const float alpha = 1.0F - std::exp(-TWO_PI_F * cutoff / sample_rate_);
        state_ += alpha * (input - state_);
        return state_;
    }
    float state() const { return state_; }
};

class EngineSaturator {
public:
    static float process(float input, float drive) {
        const float gain = 1.0F + std::max(drive, 0.0F) * SATURATION_GAIN_FACTOR;
        return std::tanh(input * gain);
    }
};

class EngineLimiter {
public:
    static float process(float input, float threshold) {
        if (!std::isfinite(input)) return 0.0F;
        const float limit = std::clamp(threshold, LIMITER_MIN, LIMITER_MAX);
        return std::clamp(input, -limit, limit);
    }
};

class EngineDspChain {
    EngineFilterProcessor filter_;
public:
    void set_sample_rate(float sample_rate) { filter_.set_sample_rate(sample_rate); }
    void reset() { filter_.reset(); }
    float process(float sample, const EngineDspState &state, float saturation, float limiter_threshold) {
        const float load = static_cast<float>(std::clamp(state.throttle, 0.0, 1.0));
        float output = filter_.process(sample, load);
        output = EngineSaturator::process(output, saturation);
        return EngineLimiter::process(output, limiter_threshold);
    }
};

} // namespace formula90s::audio
