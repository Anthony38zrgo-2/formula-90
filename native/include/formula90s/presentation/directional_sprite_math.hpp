#pragma once

#include <algorithm>
#include <cmath>

namespace formula90s::presentation {
inline double clockwise_view_angle(double rear_x, double rear_z, double camera_x, double camera_z) {
    const double cross_y = camera_z * rear_x - camera_x * rear_z;
    const double dot = rear_x * camera_x + rear_z * camera_z;
    double degrees = std::atan2(cross_y, dot) * 180.0 / 3.14159265358979323846;
    degrees = std::fmod(degrees + 360.0, 360.0);
    return degrees;
}
inline double circular_angle_distance(double a, double b) {
    return std::abs(std::fmod(a - b + 540.0, 360.0) - 180.0);
}
inline bool should_switch_direction(double current_distance, double candidate_distance, double hysteresis) {
    return candidate_distance + std::max(0.0, hysteresis) < current_distance;
}
}
