#pragma once

#include <algorithm>
#include <cmath>

namespace formula90s::camera {
inline double travel_corrected_turn(double steering, double signed_forward_speed, double activation_speed) {
    if (std::abs(signed_forward_speed) <= std::max(0.0, activation_speed)) return 0.0;
    const double clamped_steering = std::clamp(steering, -1.0, 1.0);
    return signed_forward_speed >= 0.0 ? clamped_steering : -clamped_steering;
}
}
