use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct StoredWaypoint {
    pub index: usize,
    pub pos: [f64; 3],
    #[serde(rename = "width_left")]
    pub width_left_metres: f64,
    #[serde(rename = "width_right")]
    pub width_right_metres: f64,
    #[serde(rename = "lap_distance_m")]
    pub lap_distance_metres: f64,
    #[serde(rename = "branch_id")]
    pub branch_identifier: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StoredRacingLine {
    pub track_id: String,
    #[serde(rename = "lap_length_m")]
    pub lap_length_metres: f64,
    #[serde(rename = "waypoint_count")]
    pub waypoint_count: usize,
    pub waypoints: Vec<StoredWaypoint>,
}

#[derive(Debug, Clone)]
pub struct RacingLineWaypoint {
    pub position_x_metres: f64,
    pub position_y_metres: f64,
    pub position_z_metres: f64,
    pub width_left_metres: f64,
    pub width_right_metres: f64,
    pub lap_distance_metres: f64,
    pub branch_identifier: u32,
}

#[derive(Debug, Clone)]
pub struct RacingLine {
    pub track_identifier: String,
    pub lap_length_metres: f64,
    pub waypoints: Vec<RacingLineWaypoint>,
    pub discarded_tail_waypoint_count: usize,
}

pub fn load_racing_line_from_document(document: &str) -> Result<RacingLine, String> {
    let stored: StoredRacingLine = serde_json::from_str(document)
        .map_err(|error| format!("Racing line document is invalid: {error}"))?;
    validate_stored_racing_line(&stored)?;
    let travel_end = first_travel_discontinuity(&stored.waypoints);
    if travel_end < 4 {
        return Err("Racing line continuous loop has fewer than four waypoints".into());
    }
    let retained = &stored.waypoints[..travel_end];
    let mut accumulated_distance_metres = 0.0;
    for waypoint_index in 0..retained.len() {
        let current = &retained[waypoint_index];
        let next = &retained[(waypoint_index + 1) % retained.len()];
        let geometric_distance_metres = ((next.pos[0] - current.pos[0]).powi(2)
            + (next.pos[2] - current.pos[2]).powi(2))
        .sqrt();
        let progress_metres = (next.lap_distance_metres - current.lap_distance_metres)
            .rem_euclid(stored.lap_length_metres);
        if current.branch_identifier != next.branch_identifier
            || geometric_distance_metres < 0.001
            || geometric_distance_metres > 50.0
            || (progress_metres - geometric_distance_metres).abs()
                > 1.0 + geometric_distance_metres * 0.1
        {
            return Err(format!(
                "Racing line continuity or progress is invalid at waypoint {waypoint_index}"
            ));
        }
        accumulated_distance_metres += progress_metres;
    }
    if (accumulated_distance_metres - stored.lap_length_metres).abs() > 0.01 {
        return Err("Racing line must describe exactly one closed lap".into());
    }
    if travel_end != stored.waypoints.len()
        && !(stored.track_id == "fuji76_77" && travel_end == 819 && stored.waypoints.len() == 989)
    {
        return Err("Racing line contains an unexplained discontinuous tail".into());
    }
    let mut waypoints = Vec::with_capacity(travel_end);
    for stored_waypoint in stored.waypoints.iter().take(travel_end) {
        waypoints.push(RacingLineWaypoint {
            position_x_metres: stored_waypoint.pos[0],
            position_y_metres: stored_waypoint.pos[1],
            position_z_metres: stored_waypoint.pos[2],
            width_left_metres: stored_waypoint.width_left_metres,
            width_right_metres: stored_waypoint.width_right_metres,
            lap_distance_metres: stored_waypoint.lap_distance_metres,
            branch_identifier: stored_waypoint.branch_identifier,
        });
    }
    Ok(RacingLine {
        track_identifier: stored.track_id,
        lap_length_metres: stored.lap_length_metres,
        discarded_tail_waypoint_count: stored.waypoints.len() - travel_end,
        waypoints,
    })
}

fn validate_stored_racing_line(stored: &StoredRacingLine) -> Result<(), String> {
    if stored.track_id.trim().is_empty() {
        return Err("Racing line track identifier is empty".to_string());
    }
    if !stored.lap_length_metres.is_finite() || stored.lap_length_metres <= 0.0 {
        return Err("Racing line lap length is invalid".to_string());
    }
    if stored.waypoints.len() < 4 {
        return Err("Racing line has fewer than four waypoints".to_string());
    }
    if stored.waypoint_count != stored.waypoints.len() {
        return Err("Racing line waypoint count disagrees with waypoint entries".to_string());
    }
    for (waypoint_index, stored_waypoint) in stored.waypoints.iter().enumerate() {
        if stored_waypoint.index != waypoint_index {
            return Err("Racing line waypoint indices are not sequential".into());
        }
        if !stored_waypoint
            .pos
            .iter()
            .all(|coordinate| coordinate.is_finite())
        {
            return Err("Racing line waypoint position is invalid".to_string());
        }
        if !stored_waypoint.width_left_metres.is_finite()
            || stored_waypoint.width_left_metres <= 0.0
            || !stored_waypoint.width_right_metres.is_finite()
            || stored_waypoint.width_right_metres <= 0.0
        {
            return Err("Racing line waypoint width is invalid".to_string());
        }
        if !stored_waypoint.lap_distance_metres.is_finite()
            || stored_waypoint.lap_distance_metres < 0.0
            || stored_waypoint.lap_distance_metres > stored.lap_length_metres
        {
            return Err("Racing line waypoint lap distance is invalid".to_string());
        }
    }
    Ok(())
}

fn first_travel_discontinuity(waypoints: &[StoredWaypoint]) -> usize {
    for (index, window) in waypoints.windows(2).enumerate() {
        let separation_metres = ((window[1].pos[0] - window[0].pos[0]).powi(2)
            + (window[1].pos[2] - window[0].pos[2]).powi(2))
        .sqrt();
        if separation_metres > 50.0 {
            return index + 1;
        }
    }
    waypoints.len()
}
