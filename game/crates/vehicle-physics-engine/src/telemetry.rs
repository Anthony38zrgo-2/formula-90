use serde::{Deserialize, Serialize};

/// Canonical Rust telemetry snapshot. The CSV form appends per-wheel brake
/// energy diagnostics after the existing vehicle/setup fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    pub tc_enabled: bool,
    pub tc_eligible: bool,
    pub tc_active: bool,
    pub tc_gear_authority: f64,
    pub tc_slip_target: f64,
    pub tc_raw_cut_ratio: f64,
    pub tc_cut_ratio: f64,
    pub tc_slip_ratio: [f64; 4],
    pub wheel_drive_torque_pre_tc_nm: [f64; 4],
    pub wheel_drive_torque_nm: [f64; 4],
    pub drive_torque: f64,
    pub pre_tc_drive_power_w: f64,
    pub net_drive_power_w: f64,
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
    pub brake_torque_nm: [f64; 4],
    pub brake_spin_pre_rad_s: [f64; 4],
    pub brake_spin_post_rad_s: [f64; 4],
    pub brake_power_w: [f64; 4],
    pub brake_energy_j: [f64; 4],
}

#[cfg(test)]
mod tests {
    use super::TelemetryFrame;

    #[test]
    fn csv_header_matches_serialized_field_count() {
        let line = TelemetryFrame::default().to_csv_line();
        assert_eq!(line.split(',').count(), TelemetryFrame::CSV_HEADER.len());
    }
}

impl TelemetryFrame {
    pub const CSV_HEADER: &'static [&'static str] = &[
        "Time_ms",
        "Speed_kmh",
        "RPM",
        "Gear",
        "Throttle",
        "Brake",
        "Steering",
        "Lat_G",
        "Long_G",
        "Vert_G",
        "FL_Comp",
        "FR_Comp",
        "RL_Comp",
        "RR_Comp",
        "Front_Slip",
        "Rear_Slip",
        "TC_Enabled",
        "TC_Eligible",
        "TC_Active",
        "TC_GearAuthority",
        "TC_SlipTarget",
        "TC_RawCutRatio",
        "TC_CutRatio",
        "TC_AppliedCutRatio",
        "RL_TCSlip",
        "RR_TCSlip",
        "FL_DriveTorquePreTC_Nm",
        "FR_DriveTorquePreTC_Nm",
        "RL_DriveTorquePreTC_Nm",
        "RR_DriveTorquePreTC_Nm",
        "FL_DriveTorque_Nm",
        "FR_DriveTorque_Nm",
        "RL_DriveTorque_Nm",
        "RR_DriveTorque_Nm",
        "DriveTorque",
        "PreTCDrivePower_W",
        "NetDrivePower_W",
        "Session_Id",
        "Session_Timestamp_UTC",
        "Physics_Hz",
        "Test_Id",
        "Track_Scene",
        "Vehicle_Node_Path",
        "Vehicle_Scene",
        "Vehicle_Script",
        "Setup_Schema_Version",
        "Setup_JSON",
        "FL_BrakeTorque_Nm",
        "FL_SpinPre_RadS",
        "FL_SpinPost_RadS",
        "FL_BrakePower_W",
        "FL_BrakeEnergy_J",
        "FR_BrakeTorque_Nm",
        "FR_SpinPre_RadS",
        "FR_SpinPost_RadS",
        "FR_BrakePower_W",
        "FR_BrakeEnergy_J",
        "RL_BrakeTorque_Nm",
        "RL_SpinPre_RadS",
        "RL_SpinPost_RadS",
        "RL_BrakePower_W",
        "RL_BrakeEnergy_J",
        "RR_BrakeTorque_Nm",
        "RR_SpinPre_RadS",
        "RR_SpinPost_RadS",
        "RR_BrakePower_W",
        "RR_BrakeEnergy_J",
    ];

    /// Formats the telemetry frame into a single comma-separated CSV line.
    pub fn to_csv_line(&self) -> String {
        fn quoted(value: &str) -> String {
            format!("\"{}\"", value.replace('"', "\"\""))
        }
        let mut fields = vec![
            self.time_ms.to_string(),
            format!("{:.2}", self.speed_kmh),
            format!("{:.1}", self.rpm),
            self.gear.to_string(),
            format!("{:.3}", self.throttle),
            format!("{:.3}", self.brake),
            format!("{:.3}", self.steering),
            format!("{:.3}", self.lat_g),
            format!("{:.3}", self.long_g),
            format!("{:.3}", self.vert_g),
            format!("{:.2}", self.fl_comp_mm),
            format!("{:.2}", self.fr_comp_mm),
            format!("{:.2}", self.rl_comp_mm),
            format!("{:.2}", self.rr_comp_mm),
            format!("{:.4}", self.front_slip),
            format!("{:.4}", self.rear_slip),
            self.tc_enabled.to_string(),
            self.tc_eligible.to_string(),
            self.tc_active.to_string(),
            format!("{:.4}", self.tc_gear_authority),
            format!("{:.4}", self.tc_slip_target),
            format!("{:.4}", self.tc_raw_cut_ratio),
            format!("{:.4}", self.tc_cut_ratio),
            format!("{:.4}", self.tc_cut_ratio),
            format!("{:.4}", self.tc_slip_ratio[2]),
            format!("{:.4}", self.tc_slip_ratio[3]),
        ];
        fields.extend(self.wheel_drive_torque_pre_tc_nm.iter().map(|v| format!("{v:.3}")));
        fields.extend(self.wheel_drive_torque_nm.iter().map(|v| format!("{v:.3}")));
        fields.extend([
            format!("{:.3}", self.drive_torque),
            format!("{:.3}", self.pre_tc_drive_power_w),
            format!("{:.3}", self.net_drive_power_w),
            quoted(&self.session_id),
            quoted(&self.session_timestamp_utc),
            self.physics_hz.to_string(),
            quoted(&self.test_id),
            quoted(&self.track_scene),
            quoted(&self.vehicle_node_path),
            quoted(&self.vehicle_scene),
            quoted(&self.vehicle_script),
            self.setup_schema_version.to_string(),
            quoted(&self.setup_json),
        ]);
        for i in 0..4 {
            fields.extend([
                format!("{:.3}", self.brake_torque_nm[i]),
                format!("{:.3}", self.brake_spin_pre_rad_s[i]),
                format!("{:.3}", self.brake_spin_post_rad_s[i]),
                format!("{:.3}", self.brake_power_w[i]),
                format!("{:.3}", self.brake_energy_j[i]),
            ]);
        }
        fields.join(",")
    }
}
