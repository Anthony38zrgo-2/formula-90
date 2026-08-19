use serde::{Deserialize, Serialize};

/// Canonical 26-column telemetry snapshot matching TelemetryManager.gd.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryFrame {
    pub time_ms: i64,
    pub speed_kmh: f64,
    pub rpm: f64,
    pub gear: i8,
    pub throttle: f64,
    pub brake: f64,
    pub steering: f64,
    pub lat_g: f64,
    pub long_g: f64,
    pub vert_g: f64,
    pub fl_comp_mm: f64,
    pub fr_comp_mm: f64,
    pub rl_comp_mm: f64,
    pub rr_comp_mm: f64,
    pub front_slip: f64,
    pub rear_slip: f64,
    pub tc_active: bool,
    pub drive_torque: f64,
    pub session_id: String,
    pub session_timestamp_utc: String,
    pub physics_hz: i32,
    pub test_id: String,
    pub track_scene: String,
    pub vehicle_node_path: String,
    pub vehicle_scene: String,
    pub vehicle_script: String,
    pub setup_schema_version: i32,
    pub setup_json: String,
}

impl TelemetryFrame {
    pub const CSV_HEADER: &'static [&'static str] = &[
        "Time_ms", "Speed_kmh", "RPM", "Gear",
        "Throttle", "Brake", "Steering",
        "Lat_G", "Long_G", "Vert_G",
        "FL_Comp", "FR_Comp", "RL_Comp", "RR_Comp",
        "Front_Slip", "Rear_Slip", "TC_Active", "DriveTorque",
        "Session_Id", "Session_Timestamp_UTC", "Physics_Hz",
        "Test_Id", "Track_Scene", "Vehicle_Node_Path", "Vehicle_Scene",
        "Vehicle_Script", "Setup_Schema_Version", "Setup_JSON",
    ];

    /// Formats the telemetry frame into a single comma-separated CSV line.
    pub fn to_csv_line(&self) -> String {
        format!(
            "{},{:.2},{:.1},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.2},{:.2},{:.2},{:.2},{:.4},{:.4},{},{:.1},\"{}\",\"{}\",{},\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",{},\"{}\"",
            self.time_ms,
            self.speed_kmh,
            self.rpm,
            self.gear,
            self.throttle,
            self.brake,
            self.steering,
            self.lat_g,
            self.long_g,
            self.vert_g,
            self.fl_comp_mm,
            self.fr_comp_mm,
            self.rl_comp_mm,
            self.rr_comp_mm,
            self.front_slip,
            self.rear_slip,
            self.tc_active,
            self.drive_torque,
            self.session_id,
            self.session_timestamp_utc,
            self.physics_hz,
            self.test_id,
            self.track_scene,
            self.vehicle_node_path,
            self.vehicle_scene,
            self.vehicle_script,
            self.setup_schema_version,
            self.setup_json.replace('"', "\"\"") // Escape CSV quotes in JSON
        )
    }
}
