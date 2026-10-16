#pragma once

struct DriverTireStateSnapshot {
    double wheel_normal_load_newtons[4];
    double wheel_lateral_capacity_newtons[4];
    double wheel_longitudinal_capacity_newtons[4];
    double wheel_lateral_force_newtons[4];
    double wheel_longitudinal_force_newtons[4];
    double wheel_combined_utilization[4];
    double wheel_combined_demand[4];
    double wheel_slip_angle_radians[4];
    double wheel_peak_slip_angle_radians[4];
    double wheel_grip_scale[4];
    double wheel_brake_efficiency[4];
    double total_mass_kilograms;
    double front_weight_fraction;
    double front_steering_angle_radians;
};

using ReadDriverTireStateFunction = bool (*)(const void *, DriverTireStateSnapshot *);

using ReadCoreDriverTireStateFunction = bool (*)(void *, unsigned int, DriverTireStateSnapshot *);
