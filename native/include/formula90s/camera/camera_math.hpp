#pragma once

#include <algorithm>
#include <cmath>

namespace formula90s::camera {
inline double travel_corrected_turn(double steering, double signed_forward_speed, double activation_speed) {
    if (std::abs(signed_forward_speed) <= std::max(0.0, activation_speed)) return 0.0;
    const double clamped_steering = std::clamp(steering, -1.0, 1.0);
    return signed_forward_speed >= 0.0 ? clamped_steering : -clamped_steering;
}

inline double lateral_camera_offset(double turn, double lateral_swing) {
    return std::clamp(turn, -1.0, 1.0) * std::max(lateral_swing, 0.0);
}

inline double lateral_look_offset(double turn, double lateral_swing, double turn_look_offset) {
    const double clamped_turn = std::clamp(turn, -1.0, 1.0);
    return lateral_camera_offset(clamped_turn, lateral_swing) + clamped_turn * std::max(turn_look_offset, 0.0);
}

inline double framed_car_angle(double turn, double lateral_swing, double turn_look_offset, double distance, double look_ahead) {
    const double camera_x = lateral_camera_offset(turn, lateral_swing);
    const double look_x = lateral_look_offset(turn, lateral_swing, turn_look_offset);
    const double camera_distance = std::max(distance, 0.001);
    const double target_distance = std::max(distance + look_ahead, 0.001);
    return std::atan2(-camera_x, camera_distance) - std::atan2(look_x - camera_x, target_distance);
}
}
