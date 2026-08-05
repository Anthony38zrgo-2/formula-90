#pragma once

#include <cmath>

namespace formula90s::presentation {
inline double clockwise_view_angle(double rear_x, double rear_z, double camera_x, double camera_z) {
    const double cross_y = camera_z * rear_x - camera_x * rear_z;
    const double dot = rear_x * camera_x + rear_z * camera_z;
    double degrees = std::atan2(cross_y, dot) * 180.0 / 3.14159265358979323846;
    degrees = std::fmod(degrees + 360.0, 360.0);
    return degrees;
}
}
