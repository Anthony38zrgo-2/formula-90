use std::collections::HashMap;
use std::path::Path;

use vehicle_physics_engine::{
    Mat3, RaycastHit, SurfaceType, Transform3D, TriRaycastSample, Vec3, VehicleConfig, VehicleInput,
    VehicleSimulator, WheelIndex,
};

use crate::input::{AidsState, DriverInput};
use crate::snapshot::{EntitySnapshot, EntityTelemetry, Snapshot};

/// Stable identifier for a logical entity in the world. The mirror instantiates a
/// node per `(entity_id, vehicle_scene)` pair, so Rust dictates what exists.
pub type EntityId = u32;

/// A vehicle owned by the core: its authoritative simulator, its aids state, and
/// the packed-scene references the mirror uses to render it.
#[derive(Debug, Clone)]
pub struct VehicleEntity {
    pub id: EntityId,
    pub sim: VehicleSimulator,
    pub aids: AidsState,
    pub vehicle_scene: String,
    pub track_scene: String,
    pub last: Option<vehicle_physics_engine::TelemetryFrame>,
    pub pending_input: Option<DriverInput>,
}

/// Logical session descriptor (track, laps, ...). This is the "scene the core
/// dictates" — the mirror only learns about it through `VehicleEntity`/`Snapshot`.
#[derive(Debug, Clone, Default)]
pub struct SessionConfig {
    pub track_scene: String,
    pub laps: u32,
}

/// The authoritative simulation world. It owns every entity and steps them with a
/// fixed timestep. It never touches Godot; it emits `Snapshot`s for any mirror.
pub struct World {
    pub entities: Vec<VehicleEntity>,
    pub session: SessionConfig,
    pub time: f64,
    pub dt: f64,
    next_id: EntityId,
}

impl World {
    pub fn new(dt: f64) -> Self {
        Self {
            entities: Vec::new(),
            session: SessionConfig::default(),
            time: 0.0,
            dt,
            next_id: 1,
        }
    }

    /// Spawn a vehicle from an already-loaded config at a world transform. Returns
    /// the new entity id. The `vehicle_scene`/`track_scene` strings are opaque
    /// references the mirror resolves to packed scenes.
    pub fn spawn_vehicle(
        &mut self,
        config: VehicleConfig,
        spawn: Transform3D,
        vehicle_scene: String,
        track_scene: String,
    ) -> EntityId {
        let id = self.next_id;
        self.next_id += 1;
        let sim = VehicleSimulator::new(config, spawn.origin, yaw_from_transform(&spawn));
        let aids = AidsState::from_config_mask(&sim.aids);
        self.entities.push(VehicleEntity {
            id,
            sim,
            aids,
            vehicle_scene,
            track_scene,
            last: None,
            pending_input: None,
        });
        id
    }

    pub fn load_and_spawn(
        &mut self,
        json_path: &Path,
        spawn: Transform3D,
        vehicle_scene: String,
        track_scene: String,
    ) -> Result<EntityId, String> {
        let cfg = VehicleConfig::from_json_path(json_path)?;
        Ok(self.spawn_vehicle(cfg, spawn, vehicle_scene, track_scene))
    }

    /// Advance the whole world by one fixed step. `inputs` carries the per-entity
    /// mapped driver input; missing entries mean "no input" for that entity.
    pub fn step(&mut self, inputs: &HashMap<EntityId, DriverInput>) {
        let dt = self.dt;
        self.time += dt;
        for ent in &mut self.entities {
            let input = inputs.get(&ent.id);
            // Apply edge-triggered commands from the driver input.
            if let Some(inp) = input {
                if inp.toggle_traction_control {
                    ent.aids.traction_control = !ent.aids.traction_control;
                }
                // toggle_transmission is captured here for completeness; transmission
                // mode ownership moves into the core in a later phase.
            }
            // The AidsState is authoritative: push it into the physics solver mask.
            ent.sim.aids.traction_control = ent.aids.traction_control;
            ent.sim.aids.steering_slip_assist = ent.aids.steering_slip_assist;

            let samples = flat_ground_samples(&ent.sim);
            let vi: VehicleInput = input.map(|i| i.to_vehicle_input()).unwrap_or_default();
            let telem = ent.sim.step(&vi, &samples, dt);
            ent.last = Some(telem);
        }
    }

    /// Variant used by the thin C-ABI bridge: each entity carries a `pending_input`
    /// set by `sim_world_set_input`, removing the need to ship a hash map across
    /// the FFI boundary.
    pub fn step_pending(&mut self) {
        let dt = self.dt;
        self.time += dt;
        for ent in &mut self.entities {
            let input = ent.pending_input.take();
            if let Some(inp) = &input {
                if inp.toggle_traction_control {
                    ent.aids.traction_control = !ent.aids.traction_control;
                }
            }
            ent.sim.aids.traction_control = ent.aids.traction_control;
            ent.sim.aids.steering_slip_assist = ent.aids.steering_slip_assist;
            let samples = flat_ground_samples(&ent.sim);
            let vi: VehicleInput = input.map(|i| i.to_vehicle_input()).unwrap_or_default();
            let telem = ent.sim.step(&vi, &samples, dt);
            ent.last = Some(telem);
        }
    }

    /// Seed an entity's transform from an external (Godot) pose. Used by the bridge's
    /// velocity-drive loop so the core steps from the body's real, collision-resolved
    /// position each frame.
    pub fn set_pose(&mut self, id: EntityId, x: f64, y: f64, z: f64, yaw: f64) {
        if let Some(ent) = self.entities.iter_mut().find(|e| e.id == id) {
            ent.sim.state.transform = Transform3D {
                origin: Vec3::new(x, y, z),
                basis: Mat3::from_euler_yxz(yaw, 0.0, 0.0),
            };
        }
    }

    /// Seed an entity's transform AND velocity from an external (Godot) body. Used by
    /// the bridge's velocity-drive loop so the core integrates consistently with the
    /// body's real, collision-resolved state (position + velocity), avoiding the
    /// feedback instability of seeding position alone.
    pub fn set_pose_and_velocity(
        &mut self,
        id: EntityId,
        x: f64,
        y: f64,
        z: f64,
        yaw: f64,
        lin: Vec3,
        ang: Vec3,
    ) {
        if let Some(ent) = self.entities.iter_mut().find(|e| e.id == id) {
            ent.sim.state.transform = Transform3D {
                origin: Vec3::new(x, y, z),
                basis: Mat3::from_euler_yxz(yaw, 0.0, 0.0),
            };
            ent.sim.state.linear_velocity = lin;
            ent.sim.state.angular_velocity = ang;
        }
    }

    /// Step a single entity with caller-supplied tri-ray samples (the in-engine
    /// bridge feeds real Godot raycasts; the headless CLI uses `step` with
    /// flat-ground samples). Same solver + integration path either way -> no
    /// algorithmic divergence between headless and in-engine.
    pub fn step_with_samples(
        &mut self,
        id: EntityId,
        input: &DriverInput,
        samples: &[TriRaycastSample; 4],
        dt: f64,
    ) {
        let dt = dt.clamp(1.0 / 1000.0, 1.0 / 20.0);
        self.time += dt;
        if let Some(ent) = self.entities.iter_mut().find(|e| e.id == id) {
            if input.toggle_traction_control {
                ent.aids.traction_control = !ent.aids.traction_control;
            }
            ent.sim.aids.traction_control = ent.aids.traction_control;
            ent.sim.aids.steering_slip_assist = ent.aids.steering_slip_assist;
            let vi = input.to_vehicle_input();
            let telem = ent.sim.step(&vi, samples, dt);
            ent.last = Some(telem);
        }
    }

    pub fn entity_telemetry(&self, id: EntityId) -> Option<EntityTelemetry> {
        self.entities
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| e.last.as_ref().map(EntityTelemetry::from))
    }

    /// Build the mirror contract for this tick. Only entities that have produced at
    /// least one telemetry frame are included.
    pub fn snapshot(&self) -> Snapshot {
        let mut entities = Vec::new();
        for ent in &self.entities {
            if let Some(t) = &ent.last {
                let tf = &ent.sim.state.transform;
                entities.push(EntitySnapshot {
                    id: ent.id,
                    x: tf.origin.x,
                    y: tf.origin.y,
                    z: tf.origin.z,
                    yaw: yaw_from_transform(tf),
                    vehicle_scene: ent.vehicle_scene.clone(),
                    track_scene: ent.track_scene.clone(),
                    telemetry: EntityTelemetry::from(t),
                });
            }
        }
        Snapshot {
            time_ms: (self.time * 1000.0).round() as i64,
            entities,
        }
    }
}

/// Extract yaw (radians) from a transform's basis. Forward is local -Z.
pub fn yaw_from_transform(t: &Transform3D) -> f64 {
    let mut fwd = t.basis.transform_vector(Vec3::new(0.0, 0.0, -1.0));
    fwd.y = 0.0;
    if fwd.x == 0.0 && fwd.z == 0.0 {
        0.0
    } else {
        fwd.x.atan2(-fwd.z)
    }
}

/// Build tri-raycast samples from the simulator's own transform and tire widths.
/// This is the exact same construction the in-engine C++ bridge uses (it samples
/// the real scene); here we feed flat-ground samples so the headless core and the
/// in-engine sim share ONE solver path -> no algorithmic divergence.
pub fn flat_ground_samples(sim: &VehicleSimulator) -> [TriRaycastSample; 4] {
    let mut samples = [TriRaycastSample::default(); 4];
    for (sample, &wheel) in samples.iter_mut().zip(WheelIndex::ALL.iter()) {
        let hub_local = sim.config.wheel_anchor_local(wheel);
        let hub_world = sim.state.transform.transform_point(hub_local);
        let tire_w = if wheel.is_front() {
            sim.config.front_tire_width
        } else {
            sim.config.rear_tire_width
        };
        let span = tire_w * sim.config.tri_ray_spacing_ratio;
        let left = sim
            .state
            .transform
            .basis
            .transform_vector(Vec3::new(-span, 0.0, 0.0));
        let right = sim
            .state
            .transform
            .basis
            .transform_vector(Vec3::new(span, 0.0, 0.0));
        *sample = TriRaycastSample {
            inner: make_hit(hub_world + left),
            center: make_hit(hub_world),
            outer: make_hit(hub_world + right),
        };
    }
    samples
}

fn make_hit(p: Vec3) -> RaycastHit {
    RaycastHit {
        is_colliding: true,
        distance: p.y.max(0.0),
        point: Vec3::new(p.x, 0.0, p.z),
        normal: Vec3::UP,
        surface: SurfaceType::Road,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vehicle_physics_engine::{default_spawn_height, Mat3, Transform3D, Vec3, VehicleConfig};

    fn build_world() -> World {
        let mut w = World::new(1.0 / 120.0);
        let cfg = VehicleConfig::f1_94_canonical();
        let spawn = Transform3D {
            origin: Vec3::new(0.0, default_spawn_height(&cfg), 0.0),
            basis: Mat3::IDENTITY,
        };
        w.spawn_vehicle(cfg, spawn, "vs".to_string(), "ts".to_string());
        w
    }

    #[test]
    fn headless_core_is_deterministic() {
        let mut a = build_world();
        let mut b = build_world();
        let input = DriverInput {
            throttle: 1.0,
            ..Default::default()
        };
        let mut ia = HashMap::new();
        let mut ib = HashMap::new();
        ia.insert(1u32, input);
        ib.insert(1u32, input);
        for _ in 0..120 {
            a.step(&ia);
            b.step(&ib);
        }
        let ba = a.snapshot().to_bytes().unwrap();
        let bb = b.snapshot().to_bytes().unwrap();
        assert_eq!(ba, bb, "snapshots must be byte-identical across identical runs");
    }
}
