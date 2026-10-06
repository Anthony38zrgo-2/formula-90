use crate::coupled_vehicle_world::{CoupledVehicleWorld, CoupledVehicleWorldConfiguration, TimestampedVehicleInput};
use crate::physical_collision_world::{PhysicalCollisionWorld, PhysicalWorldPackage};
use crate::types::Vec3;
use crate::vehicle_config::VehicleConfig;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::{c_char, CString};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::thread::ThreadId;

pub const COUPLED_VEHICLE_WORLD_INTERFACE_VERSION: u32 = 1;

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum CoupledVehicleWorldRequest {
    CreateWorld { interface_version: u32, repository_root: PathBuf, physical_package_path: PathBuf, #[serde(default)] configuration: Option<CoupledVehicleWorldConfiguration>, #[serde(default)] vehicle_profile: Option<String> },
    DestroyWorld { world_identifier: u64 },
    RegisterVehicle { world_identifier: u64, vehicle_profile: String, position_world_metres: Vec3, yaw_radians: f64 },
    RemoveVehicle { world_identifier: u64, entity_identifier: u64 },
    SubmitInput { world_identifier: u64, entity_identifier: u64, sample: TimestampedVehicleInput },
    Advance { world_identifier: u64, duration_seconds: f64 },
    Snapshots { world_identifier: u64 },
    ResetVehicle { world_identifier: u64, entity_identifier: u64, #[serde(default)] position_world_metres: Option<Vec3>, #[serde(default)] yaw_radians: Option<f64> },
    SetPaused { world_identifier: u64, paused: bool },
    Refuel { world_identifier: u64, entity_identifier: u64, kilograms: f64 },
    ReplaceTires { world_identifier: u64, entity_identifier: u64 },
}

struct ThreadOwnedVehicleWorld {
    owner_thread: ThreadId,
    world: CoupledVehicleWorld,
}

#[derive(Default)]
struct CoupledVehicleWorldRegistry {
    worlds: BTreeMap<u64, ThreadOwnedVehicleWorld>,
    next_identifier: u64,
}

impl CoupledVehicleWorldRegistry {
    fn world(&mut self, identifier: u64) -> Result<&mut CoupledVehicleWorld, String> {
        let entry = self.worlds.get_mut(&identifier).ok_or("Invalid or destroyed physical-world identifier")?;
        if entry.owner_thread != std::thread::current().id() { return Err("Physical world must be accessed from its owning thread".into()); }
        Ok(&mut entry.world)
    }

    fn execute(&mut self, request: CoupledVehicleWorldRequest) -> Result<serde_json::Value, String> {
        match request {
            CoupledVehicleWorldRequest::CreateWorld { interface_version, repository_root, physical_package_path, configuration, vehicle_profile } => {
                if interface_version != COUPLED_VEHICLE_WORLD_INTERFACE_VERSION { return Err("Physical-world native interface version mismatch".into()); }
                let document = std::fs::read_to_string(physical_package_path).map_err(|error| format!("Cannot read physical-world package: {error}"))?;
                let package: PhysicalWorldPackage = serde_json::from_str(&document).map_err(|error| format!("Invalid physical-world package: {error}"))?;
                let collision = PhysicalCollisionWorld::from_package(package, &repository_root)?;
                let configuration = match (configuration, vehicle_profile) {
                    (Some(configuration), None) => configuration,
                    (None, Some(document)) => VehicleConfig::from_json_str(&document)?.coupled_world.ok_or("Profile does not select a coupled world")?,
                    _ => return Err("World creation requires exactly one configuration source".into()),
                };
                let world = CoupledVehicleWorld::new(collision, configuration)?;
                self.next_identifier = self.next_identifier.checked_add(1).ok_or("Physical-world identity space exhausted")?;
                let identifier = self.next_identifier;
                self.worlds.insert(identifier, ThreadOwnedVehicleWorld { owner_thread: std::thread::current().id(), world });
                Ok(serde_json::json!({"world_identifier": identifier, "interface_version": COUPLED_VEHICLE_WORLD_INTERFACE_VERSION}))
            }
            CoupledVehicleWorldRequest::DestroyWorld { world_identifier } => {
                self.world(world_identifier)?;
                self.worlds.remove(&world_identifier);
                Ok(serde_json::json!({"destroyed": world_identifier}))
            }
            CoupledVehicleWorldRequest::RegisterVehicle { world_identifier, vehicle_profile, position_world_metres, yaw_radians } => {
                let configuration = VehicleConfig::from_json_str(&vehicle_profile)?;
                let identifier = self.world(world_identifier)?.register_vehicle(configuration, position_world_metres, yaw_radians)?;
                Ok(serde_json::json!({"entity_identifier": identifier}))
            }
            CoupledVehicleWorldRequest::RemoveVehicle { world_identifier, entity_identifier } => {
                self.world(world_identifier)?.remove_vehicle(entity_identifier)?;
                Ok(serde_json::json!({"removed": entity_identifier}))
            }
            CoupledVehicleWorldRequest::SubmitInput { world_identifier, entity_identifier, sample } => {
                self.world(world_identifier)?.enqueue_input(entity_identifier, sample)?;
                Ok(serde_json::json!({"queued": entity_identifier}))
            }
            CoupledVehicleWorldRequest::Advance { world_identifier, duration_seconds } => {
                let world = self.world(world_identifier)?;
                world.advance_host_interval(duration_seconds)?;
                Ok(serde_json::json!({"time_seconds": world.time_seconds, "snapshots": world.snapshots()?, "events": world.collision_events,
                    "unconsumed_host_time_seconds": world.accumulated_host_time_seconds}))
            }
            CoupledVehicleWorldRequest::Snapshots { world_identifier } => {
                let world = self.world(world_identifier)?;
                Ok(serde_json::json!({"time_seconds": world.time_seconds, "snapshots": world.snapshots()?}))
            }
            CoupledVehicleWorldRequest::ResetVehicle { world_identifier, entity_identifier, position_world_metres, yaw_radians } => {
                let world = self.world(world_identifier)?;
                if position_world_metres.is_some() != yaw_radians.is_some() { return Err("Reset pose requires both position and yaw".into()); }
                if let (Some(position), Some(yaw)) = (position_world_metres, yaw_radians) {
                    if !crate::suspension_mass_properties::vector_is_finite(position) || !yaw.is_finite() { return Err("Invalid reset pose".into()); }
                    let mut candidate = world.clone();
                    let vehicle = candidate.vehicles.get_mut(&entity_identifier).ok_or("Unknown physical vehicle")?;
                    vehicle.spawn_position_world_metres = position;
                    vehicle.spawn_yaw_radians = yaw;
                    candidate.reset_vehicle(entity_identifier)?;
                    *world = candidate;
                } else { world.reset_vehicle(entity_identifier)?; }
                Ok(serde_json::json!({"reset": entity_identifier}))
            }
            CoupledVehicleWorldRequest::SetPaused { world_identifier, paused } => {
                self.world(world_identifier)?.paused = paused;
                Ok(serde_json::json!({"paused": paused}))
            }
            CoupledVehicleWorldRequest::Refuel { world_identifier, entity_identifier, kilograms } => {
                let world = self.world(world_identifier)?;
                let vehicle = world.vehicles.get_mut(&entity_identifier).ok_or("Unknown physical vehicle")?;
                if !kilograms.is_finite() || kilograms < 0.0 || kilograms > vehicle.systems.config.fuel.capacity_kg { return Err("Refuel request exceeds physical tank capacity".into()); }
                let mut candidate = vehicle.clone();
                candidate.model.update_fuel_mass(&mut candidate.state, &candidate.last_force_input, kilograms)?;
                candidate.systems.set_fuel_kg(kilograms);
                *vehicle = candidate;
                Ok(serde_json::json!({"fuel_mass_kilograms": kilograms}))
            }
            CoupledVehicleWorldRequest::ReplaceTires { world_identifier, entity_identifier } => {
                let vehicle = self.world(world_identifier)?.vehicles.get_mut(&entity_identifier).ok_or("Unknown physical vehicle")?;
                vehicle.systems.replace_tire_set();
                Ok(serde_json::json!({"tires_replaced": entity_identifier}))
            }
        }
    }
}

#[derive(Serialize)]
struct CoupledVehicleWorldResponse {
    interface_version: u32,
    success: bool,
    result: Option<serde_json::Value>,
    error: Option<String>,
}

static PHYSICAL_WORLD_REGISTRY: OnceLock<Mutex<CoupledVehicleWorldRegistry>> = OnceLock::new();

pub fn execute_coupled_vehicle_world_request(document: &str) -> String {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let request = serde_json::from_str(document).map_err(|error| format!("Invalid world request: {error}"))?;
        let registry = PHYSICAL_WORLD_REGISTRY.get_or_init(|| Mutex::new(CoupledVehicleWorldRegistry::default()));
        registry.lock().map_err(|_| "Physical-world registry was poisoned".to_string())?.execute(request)
    })).unwrap_or_else(|_| Err("Physical-world request failed inside the Rust boundary".into()));
    let response = match result {
        Ok(result) => CoupledVehicleWorldResponse { interface_version: COUPLED_VEHICLE_WORLD_INTERFACE_VERSION, success: true, result: Some(result), error: None },
        Err(error) => CoupledVehicleWorldResponse { interface_version: COUPLED_VEHICLE_WORLD_INTERFACE_VERSION, success: false, result: None, error: Some(error) },
    };
    serde_json::to_string(&response).unwrap_or_else(|_| "{\"interface_version\":1,\"success\":false,\"error\":\"World response serialization failed\"}".into())
}

#[no_mangle]
pub extern "C" fn coupled_vehicle_world_build_source() -> *const c_char {
    concat!(env!("COUPLED_WORLD_SOURCE_IDENTIFIER"), "\0").as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn coupled_vehicle_world_interface_version() -> u32 { COUPLED_VEHICLE_WORLD_INTERFACE_VERSION }

#[no_mangle]
pub unsafe extern "C" fn coupled_vehicle_world_execute_request(document: *const u8, document_length: usize) -> *mut c_char {
    let response = if document.is_null() || document_length == 0 || document_length > 67108864 {
        "{\"interface_version\":1,\"success\":false,\"error\":\"Invalid request buffer\"}".to_string()
    } else {
        let bytes = unsafe { std::slice::from_raw_parts(document, document_length) };
        match std::str::from_utf8(bytes) {
            Ok(document) => execute_coupled_vehicle_world_request(document),
            Err(_) => "{\"interface_version\":1,\"success\":false,\"error\":\"Request is not UTF-8\"}".to_string(),
        }
    };
    CString::new(response).map_or(std::ptr::null_mut(), CString::into_raw)
}

#[no_mangle]
pub unsafe extern "C" fn coupled_vehicle_world_free_response(response: *mut c_char) {
    if !response.is_null() { drop(unsafe { CString::from_raw(response) }); }
}
