use super::path_reader::RacingLineWaypoint;

#[derive(Debug, Clone, Copy)]
pub struct PathProjection {
    pub segment_start_index: usize,
    pub arc_length_metres: f64,
    pub lateral_offset_metres: f64,
    pub heading_error_radians: f64,
}

const LOCAL_SEARCH_HALF_WINDOW: usize = 40;

pub fn project_onto_path(
    waypoints: &[RacingLineWaypoint],
    lap_length_metres: f64,
    position_x_metres: f64,
    position_z_metres: f64,
    heading_radians: f64,
    previous_segment_start_index: usize,
) -> Result<PathProjection, String> {
    if waypoints.len() < 2 {
        return Err("Path projection needs at least two waypoints".to_string());
    }
    if !lap_length_metres.is_finite() || lap_length_metres <= 0.0 {
        return Err("Path projection lap length is invalid".to_string());
    }
    if !position_x_metres.is_finite()
        || !position_z_metres.is_finite()
        || !heading_radians.is_finite()
    {
        return Err("Path projection pose is invalid".to_string());
    }
    let local_candidate = search_window_projection(
        waypoints,
        position_x_metres,
        position_z_metres,
        previous_segment_start_index,
    );
    let global_candidate =
        if local_candidate.is_none_or(|candidate| candidate.distance_squared_metres > 400.0) {
            global_projection(waypoints, position_x_metres, position_z_metres)
        } else {
            None
        };
    let selected = match (local_candidate, global_candidate) {
        (Some(local), Some(global)) => {
            if global.distance_squared_metres + 25.0 < local.distance_squared_metres {
                global
            } else {
                local
            }
        }
        (Some(local), None) => local,
        (None, Some(global)) => global,
        (None, None) => return Err("Path projection found no valid segment".to_string()),
    };
    Ok(finish_projection(
        waypoints,
        lap_length_metres,
        &selected,
        position_x_metres,
        position_z_metres,
        heading_radians,
    ))
}

#[derive(Clone, Copy)]
struct SegmentProjection {
    segment_start_index: usize,
    along_segment_fraction: f64,
    distance_squared_metres: f64,
}

fn search_window_projection(
    waypoints: &[RacingLineWaypoint],
    position_x_metres: f64,
    position_z_metres: f64,
    previous_segment_start_index: usize,
) -> Option<SegmentProjection> {
    let waypoint_count = waypoints.len();
    if previous_segment_start_index >= waypoint_count {
        return None;
    }
    let mut best_projection: Option<SegmentProjection> = None;
    for offset in 0..LOCAL_SEARCH_HALF_WINDOW {
        for direction in [1_isize, -1_isize] {
            if offset == 0 && direction == -1 {
                continue;
            }
            let candidate_index = (previous_segment_start_index as isize
                + direction * offset as isize)
                .rem_euclid(waypoint_count as isize) as usize;
            let candidate = project_onto_segment(
                waypoints,
                candidate_index,
                position_x_metres,
                position_z_metres,
            );
            best_projection = Some(select_nearest_projection(best_projection, candidate));
        }
    }
    best_projection
}

fn global_projection(
    waypoints: &[RacingLineWaypoint],
    position_x_metres: f64,
    position_z_metres: f64,
) -> Option<SegmentProjection> {
    let mut best_projection: Option<SegmentProjection> = None;
    for segment_start_index in 0..waypoints.len() {
        let candidate = project_onto_segment(
            waypoints,
            segment_start_index,
            position_x_metres,
            position_z_metres,
        );
        best_projection = Some(select_nearest_projection(best_projection, candidate));
    }
    best_projection
}

fn select_nearest_projection(
    best_projection: Option<SegmentProjection>,
    candidate: SegmentProjection,
) -> SegmentProjection {
    match best_projection {
        Some(best) => {
            if candidate.distance_squared_metres < best.distance_squared_metres {
                candidate
            } else {
                best
            }
        }
        None => candidate,
    }
}

fn project_onto_segment(
    waypoints: &[RacingLineWaypoint],
    segment_start_index: usize,
    position_x_metres: f64,
    position_z_metres: f64,
) -> SegmentProjection {
    let waypoint_count = waypoints.len();
    let start_waypoint = &waypoints[segment_start_index];
    let end_waypoint = &waypoints[(segment_start_index + 1) % waypoint_count];
    let segment_x_metres = end_waypoint.position_x_metres - start_waypoint.position_x_metres;
    let segment_z_metres = end_waypoint.position_z_metres - start_waypoint.position_z_metres;
    let segment_length_squared =
        segment_x_metres * segment_x_metres + segment_z_metres * segment_z_metres;
    if segment_length_squared < 1e-12 {
        let offset_x_metres = position_x_metres - start_waypoint.position_x_metres;
        let offset_z_metres = position_z_metres - start_waypoint.position_z_metres;
        return SegmentProjection {
            segment_start_index,
            along_segment_fraction: 0.0,
            distance_squared_metres: offset_x_metres * offset_x_metres
                + offset_z_metres * offset_z_metres,
        };
    }
    let along_segment_fraction = ((position_x_metres - start_waypoint.position_x_metres)
        * segment_x_metres
        + (position_z_metres - start_waypoint.position_z_metres) * segment_z_metres)
        / segment_length_squared;
    let clamped_fraction = along_segment_fraction.clamp(0.0, 1.0);
    let nearest_x_metres = start_waypoint.position_x_metres + clamped_fraction * segment_x_metres;
    let nearest_z_metres = start_waypoint.position_z_metres + clamped_fraction * segment_z_metres;
    let offset_x_metres = position_x_metres - nearest_x_metres;
    let offset_z_metres = position_z_metres - nearest_z_metres;
    SegmentProjection {
        segment_start_index,
        along_segment_fraction: clamped_fraction,
        distance_squared_metres: offset_x_metres * offset_x_metres
            + offset_z_metres * offset_z_metres,
    }
}

fn finish_projection(
    waypoints: &[RacingLineWaypoint],
    lap_length_metres: f64,
    selected: &SegmentProjection,
    position_x_metres: f64,
    position_z_metres: f64,
    heading_radians: f64,
) -> PathProjection {
    let waypoint_count = waypoints.len();
    let start_waypoint = &waypoints[selected.segment_start_index];
    let end_waypoint = &waypoints[(selected.segment_start_index + 1) % waypoint_count];
    let segment_x_metres = end_waypoint.position_x_metres - start_waypoint.position_x_metres;
    let segment_z_metres = end_waypoint.position_z_metres - start_waypoint.position_z_metres;
    let segment_length_metres =
        (segment_x_metres * segment_x_metres + segment_z_metres * segment_z_metres).sqrt();
    let start_distance = start_waypoint.lap_distance_metres;
    let mut end_distance = end_waypoint.lap_distance_metres;
    if end_distance < start_distance {
        end_distance += lap_length_metres;
    }
    let mut arc_length_metres =
        start_distance + selected.along_segment_fraction * (end_distance - start_distance);
    if arc_length_metres >= lap_length_metres {
        arc_length_metres -= lap_length_metres;
    }
    let lateral_offset_metres = if segment_length_metres > 1e-9 {
        let relative_x_metres = position_x_metres - start_waypoint.position_x_metres;
        let relative_z_metres = position_z_metres - start_waypoint.position_z_metres;
        (segment_x_metres * relative_z_metres - segment_z_metres * relative_x_metres)
            / segment_length_metres
    } else {
        0.0
    };
    let segment_heading_radians = (-segment_x_metres).atan2(-segment_z_metres);
    PathProjection {
        segment_start_index: selected.segment_start_index,
        arc_length_metres,
        lateral_offset_metres,
        heading_error_radians: normalize_angle(heading_radians - segment_heading_radians),
    }
}

fn normalize_angle(angle_radians: f64) -> f64 {
    let two_pi = std::f64::consts::PI * 2.0;
    let mut normalized = angle_radians % two_pi;
    if normalized > std::f64::consts::PI {
        normalized -= two_pi;
    }
    if normalized < -std::f64::consts::PI {
        normalized += two_pi;
    }
    normalized
}
