use serde::{Deserialize, Serialize};
use vehicle_physics_engine::TelemetryFrame;

/// Compact, serializable per-entity state carried in every `Snapshot`. This is the
/// contract between the authoritative Rust core and any mirror (headless replay,
/// Godot renderer, or a remote server). Keep it transport-agnostic: the same bytes
/// go over shared memory locally or over the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub id: u32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f64,
    pub vehicle_scene: String,
    pub track_scene: String,
    pub telemetry: EntityTelemetry,
}

/// The simulation-derived readouts a mirror (HUD, renderer) needs. A subset of
/// `TelemetryFrame` chosen to be mirror-relevant and cheap to ship every tick.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityTelemetry {
    pub speed_kmh: f64,
    pub rpm: f64,
    pub gear: i8,
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
}

impl From<&TelemetryFrame> for EntityTelemetry {
    fn from(t: &TelemetryFrame) -> Self {
        Self {
            speed_kmh: t.speed_kmh,
            rpm: t.rpm,
            gear: t.gear,
            steering: t.steering,
            lat_g: t.lat_g,
            long_g: t.long_g,
            vert_g: t.vert_g,
            fl_comp_mm: t.fl_comp_mm,
            fr_comp_mm: t.fr_comp_mm,
            rl_comp_mm: t.rl_comp_mm,
            rr_comp_mm: t.rr_comp_mm,
            front_slip: t.front_slip,
            rear_slip: t.rear_slip,
            tc_active: t.tc_active,
            drive_torque: t.drive_torque,
        }
    }
}

/// The authoritative world snapshot for a single tick. `time_ms` is the core clock
/// so mirrors never depend on their own wall clock (determinism).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub time_ms: i64,
    pub entities: Vec<EntitySnapshot>,
}

impl Snapshot {
    pub fn to_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        postcard::to_allocvec(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, postcard::Error> {
        postcard::from_bytes(bytes)
    }
}
