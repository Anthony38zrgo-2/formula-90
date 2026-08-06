#pragma once

#include <algorithm>
#include <cmath>

namespace formula90s::presentation {

inline double smoothing_alpha(double rate, double delta) {
    return 1.0 - std::exp(-std::max(rate, 0.01) * std::max(delta, 0.0));
}

inline double clamped_visual_response(double source, double gain, double maximum) {
    const double limit = std::max(maximum, 0.0);
    return std::clamp(source * gain, -limit, limit);
}

inline double wheel_rotation_delta(double signed_speed, double wheel_radius, double delta) {
    if (wheel_radius <= 0.001 || delta <= 0.0) {
        return 0.0;
    }
    return signed_speed * delta / wheel_radius;
}

} // namespace formula90s::presentation
