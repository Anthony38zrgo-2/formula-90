use super::path_reader::RacingLineWaypoint;

#[derive(Debug, Clone, Copy)]
pub struct SpeedProfileSample {
    pub arc_length_metres: f64,
    pub target_speed_metres_per_second: f64,
    pub curvature_per_metre: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct SpeedProfileLimits {
    pub maximum_speed_metres_per_second: f64,
    pub lateral_acceleration_limit_metres_per_second_squared: f64,
    pub longitudinal_deceleration_limit_metres_per_second_squared: f64,
    pub longitudinal_acceleration_limit_metres_per_second_squared: f64,
    pub safety_margin_fraction: f64,
}

pub fn build_speed_profile(
    waypoints: &[RacingLineWaypoint],
    lap_length_metres: f64,
    limits: SpeedProfileLimits,
) -> Result<Vec<SpeedProfileSample>, String> {
    validate_speed_profile_limits(&limits)?;
    validate_profile_geometry(waypoints, lap_length_metres)?;
    let waypoint_count = waypoints.len();
    let mut samples = Vec::with_capacity(waypoint_count);
    for waypoint_index in 0..waypoint_count {
        let previous_waypoint = &waypoints[(waypoint_index + waypoint_count - 1) % waypoint_count];
        let current_waypoint = &waypoints[waypoint_index];
        let next_waypoint = &waypoints[(waypoint_index + 1) % waypoint_count];
        let curvature = curvature_from_triplet(previous_waypoint, current_waypoint, next_waypoint);
        let corner_speed = if curvature.abs() < 1e-9 {
            limits.maximum_speed_metres_per_second
        } else {
            let raw_corner_speed = (limits.lateral_acceleration_limit_metres_per_second_squared
                / curvature.abs())
            .sqrt();
            raw_corner_speed.min(limits.maximum_speed_metres_per_second)
        };
        let target_speed = corner_speed * (1.0 - limits.safety_margin_fraction);
        samples.push(SpeedProfileSample {
            arc_length_metres: current_waypoint.lap_distance_metres,
            target_speed_metres_per_second: target_speed,
            curvature_per_metre: curvature,
        });
    }
    for _ in 0..=waypoint_count {
        let previous_speeds: Vec<f64> = samples
            .iter()
            .map(|sample| sample.target_speed_metres_per_second)
            .collect();
        apply_braking_pass(&mut samples, lap_length_metres, &limits);
        if limits.maximum_speed_metres_per_second.is_finite() {
            apply_acceleration_pass(&mut samples, lap_length_metres, &limits);
        }
        if samples
            .iter()
            .zip(previous_speeds)
            .all(|(sample, previous_speed)| {
                (sample.target_speed_metres_per_second - previous_speed).abs() < 1e-10
            })
        {
            return Ok(samples);
        }
    }
    Err("Closed-loop speed profile did not converge".into())
}

fn validate_speed_profile_limits(limits: &SpeedProfileLimits) -> Result<(), String> {
    if limits.maximum_speed_metres_per_second.is_nan()
        || limits.maximum_speed_metres_per_second <= 0.0
    {
        return Err("Speed profile maximum speed is invalid".to_string());
    }
    if !limits
        .lateral_acceleration_limit_metres_per_second_squared
        .is_finite()
        || limits.lateral_acceleration_limit_metres_per_second_squared <= 0.0
    {
        return Err("Speed profile lateral acceleration limit is invalid".to_string());
    }
    if !limits
        .longitudinal_deceleration_limit_metres_per_second_squared
        .is_finite()
        || limits.longitudinal_deceleration_limit_metres_per_second_squared <= 0.0
    {
        return Err("Speed profile braking limit is invalid".to_string());
    }
    if !limits
        .longitudinal_acceleration_limit_metres_per_second_squared
        .is_finite()
        || limits.longitudinal_acceleration_limit_metres_per_second_squared <= 0.0
    {
        return Err("Speed profile acceleration limit is invalid".to_string());
    }
    if !limits.safety_margin_fraction.is_finite()
        || limits.safety_margin_fraction < 0.0
        || limits.safety_margin_fraction >= 0.5
    {
        return Err("Speed profile safety margin is invalid".to_string());
    }
    Ok(())
}

pub(crate) fn curvature_from_triplet(
    previous_waypoint: &RacingLineWaypoint,
    current_waypoint: &RacingLineWaypoint,
    next_waypoint: &RacingLineWaypoint,
) -> f64 {
    let entry_x_metres = current_waypoint.position_x_metres - previous_waypoint.position_x_metres;
    let entry_z_metres = current_waypoint.position_z_metres - previous_waypoint.position_z_metres;
    let exit_x_metres = next_waypoint.position_x_metres - current_waypoint.position_x_metres;
    let exit_z_metres = next_waypoint.position_z_metres - current_waypoint.position_z_metres;
    let entry_length = (entry_x_metres * entry_x_metres + entry_z_metres * entry_z_metres).sqrt();
    let exit_length = (exit_x_metres * exit_x_metres + exit_z_metres * exit_z_metres).sqrt();
    if entry_length < 1e-6 || exit_length < 1e-6 {
        return 0.0;
    }
    let cross_product = entry_x_metres * exit_z_metres - entry_z_metres * exit_x_metres;
    let dot_product = entry_x_metres * exit_x_metres + entry_z_metres * exit_z_metres;
    let turning_angle_radians = cross_product.atan2(dot_product);
    let average_length_metres = (entry_length + exit_length) * 0.5;
    turning_angle_radians / average_length_metres
}

fn apply_braking_pass(
    samples: &mut [SpeedProfileSample],
    lap_length_metres: f64,
    limits: &SpeedProfileLimits,
) {
    let sample_count = samples.len();
    for reverse_index in 0..sample_count {
        let current_index = (sample_count - 1 - reverse_index) % sample_count;
        let next_index = (current_index + 1) % sample_count;
        let mut gap_metres =
            samples[next_index].arc_length_metres - samples[current_index].arc_length_metres;
        if gap_metres <= 0.0 {
            gap_metres += lap_length_metres;
        }
        let reachable_speed = (samples[next_index].target_speed_metres_per_second.powi(2)
            + 2.0 * limits.longitudinal_deceleration_limit_metres_per_second_squared * gap_metres)
            .sqrt();
        if reachable_speed < samples[current_index].target_speed_metres_per_second {
            samples[current_index].target_speed_metres_per_second = reachable_speed;
        }
    }
    for _ in 0..2 {
        for reverse_index in 0..sample_count {
            let current_index = (sample_count - 1 - reverse_index) % sample_count;
            let next_index = (current_index + 1) % sample_count;
            let mut gap_metres =
                samples[next_index].arc_length_metres - samples[current_index].arc_length_metres;
            if gap_metres <= 0.0 {
                gap_metres += lap_length_metres;
            }
            let reachable_speed = (samples[next_index].target_speed_metres_per_second.powi(2)
                + 2.0
                    * limits.longitudinal_deceleration_limit_metres_per_second_squared
                    * gap_metres)
                .sqrt();
            if reachable_speed < samples[current_index].target_speed_metres_per_second {
                samples[current_index].target_speed_metres_per_second = reachable_speed;
            }
        }
    }
}

fn apply_acceleration_pass(
    samples: &mut [SpeedProfileSample],
    lap_length_metres: f64,
    limits: &SpeedProfileLimits,
) {
    let sample_count = samples.len();
    for _ in 0..3 {
        for current_index in 0..sample_count {
            let previous_index = (current_index + sample_count - 1) % sample_count;
            let mut gap_metres = samples[current_index].arc_length_metres
                - samples[previous_index].arc_length_metres;
            if gap_metres <= 0.0 {
                gap_metres += lap_length_metres;
            }
            let reachable_speed = (samples[previous_index]
                .target_speed_metres_per_second
                .powi(2)
                + 2.0
                    * limits.longitudinal_acceleration_limit_metres_per_second_squared
                    * gap_metres)
                .sqrt();
            if reachable_speed < samples[current_index].target_speed_metres_per_second {
                samples[current_index].target_speed_metres_per_second = reachable_speed;
            }
        }
    }
}

pub(crate) fn validate_profile_geometry(waypoints: &[RacingLineWaypoint], lap_length_metres: f64) -> Result<(), String> {
    if waypoints.len() < 4 {
        return Err("Speed profile needs at least four waypoints".to_string());
    }
    if !lap_length_metres.is_finite() || lap_length_metres <= 0.0 {
        return Err("Speed profile lap length is invalid".to_string());
    }
    let waypoint_count = waypoints.len();
    let mut total_progress_metres = 0.0;
    for waypoint_index in 0..waypoint_count {
        let waypoint = &waypoints[waypoint_index];
        let next_waypoint = &waypoints[(waypoint_index + 1) % waypoint_count];
        let geometric_distance_metres = (next_waypoint.position_x_metres
            - waypoint.position_x_metres)
            .hypot(next_waypoint.position_z_metres - waypoint.position_z_metres);
        if !geometric_distance_metres.is_finite()
            || geometric_distance_metres < 0.001
            || !waypoint.width_left_metres.is_finite()
            || waypoint.width_left_metres <= 0.0
            || !waypoint.width_right_metres.is_finite()
            || waypoint.width_right_metres <= 0.0
        {
            return Err(
                "Speed profile contains degenerate geometry or invalid track widths".into(),
            );
        }
        if ![
            waypoint.position_x_metres,
            waypoint.position_y_metres,
            waypoint.position_z_metres,
            waypoint.lap_distance_metres,
        ]
        .iter()
        .all(|value| value.is_finite())
            || waypoint.lap_distance_metres < 0.0
            || waypoint.lap_distance_metres >= lap_length_metres
        {
            return Err("Speed profile waypoint geometry or distance is invalid".into());
        }
        let progress_metres = (next_waypoint.lap_distance_metres - waypoint.lap_distance_metres)
            .rem_euclid(lap_length_metres);
        if progress_metres < 0.001 {
            return Err("Speed profile contains repeated waypoint distances".into());
        }
        total_progress_metres += progress_metres;
    }
    if (total_progress_metres - lap_length_metres).abs() > 0.01 {
        return Err("Speed profile waypoints do not describe one ordered lap".into());
    }
    Ok(())
}
