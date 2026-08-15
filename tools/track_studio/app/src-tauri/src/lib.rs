//! Tauri shell for Formula-90 Track Studio.
//!
//! These `#[tauri::command]` functions are thin wrappers over
//! `track_service::AppService`. All real logic lives in the service so it is
//! unit-tested without a webview. The service is kept behind a `Mutex` because
//! rusqlite connections are `Send` but not `Sync`.

use serde_json::Value;
use std::sync::Mutex;
use tauri::State;
use track_service::{AppService, ProjectSnapshot};

/// Application state shared with Tauri commands.
pub struct AppState {
    pub service: Mutex<AppService>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            service: Mutex::new(AppService::default()),
        }
    }
}

#[tauri::command]
fn project_open(state: State<AppState>, path: String) -> Result<String, String> {
    let mut service = state.service.lock().map_err(|e| e.to_string())?;
    *service = AppService::open_project(&path, None).map_err(|e| e.to_string())?;
    Ok(service.track_id().to_owned())
}

#[tauri::command]
fn project_save(state: State<AppState>) -> Result<(), String> {
    let service = state.service.lock().map_err(|e| e.to_string())?;
    service.save_project().map_err(|e| e.to_string())
}

#[tauri::command]
fn document_snapshot(state: State<AppState>) -> Result<ProjectSnapshot, String> {
    let service = state.service.lock().map_err(|e| e.to_string())?;
    service.snapshot().map_err(|e| e.to_string())
}

#[tauri::command]
fn document_validate(state: State<AppState>) -> Result<Vec<Value>, String> {
    let service = state.service.lock().map_err(|e| e.to_string())?;
    Ok(service
        .validate()
        .into_iter()
        .map(|diagnostic| serde_json::to_value(diagnostic).unwrap_or(Value::Null))
        .collect())
}

#[tauri::command]
fn document_undo(state: State<AppState>) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|e| e.to_string())?;
    service.undo().map_err(|e| e.to_string())
}

#[tauri::command]
fn document_redo(state: State<AppState>) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|e| e.to_string())?;
    service.redo().map_err(|e| e.to_string())
}

#[tauri::command]
fn compile_build_ir(state: State<AppState>) -> Result<Value, String> {
    let service = state.service.lock().map_err(|e| e.to_string())?;
    let build_ir = service.compile_build_ir().map_err(|e| e.to_string())?;
    serde_json::to_value(build_ir).map_err(|e| e.to_string())
}

#[tauri::command]
fn build_plan(state: State<AppState>) -> Result<Vec<String>, String> {
    let service = state.service.lock().map_err(|e| e.to_string())?;
    let subsystems = service.dry_run_build().map_err(|e| e.to_string())?;
    Ok(subsystems.iter().map(|s| s.label().to_owned()).collect())
}

#[tauri::command]
fn reference_add(
    state: State<AppState>,
    image_path: String,
    x: f64,
    z: f64,
    opacity: f64,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|e| e.to_string())?;
    let layer = track_domain::ReferenceLayer {
        id: format!("ref_{}", service.reference_layers().len()),
        image_path,
        position: track_domain::Point2 { x, z },
        rotation_rad: 0.0,
        scale: 1.0,
        opacity,
        locked: false,
        visible: true,
        calibration: None,
    };
    service.add_reference_layer(layer).map_err(|e| e.to_string())
}

#[tauri::command]
fn reference_calibrate(
    state: State<AppState>,
    layer_id: String,
    image_distance_px: f64,
    real_distance_m: f64,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|e| e.to_string())?;
    service
        .calibrate_reference(layer_id, image_distance_px, real_distance_m)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn measure_world(state: State<AppState>, a: [f64; 2], b: [f64; 2]) -> Result<f64, String> {
    let service = state.service.lock().map_err(|e| e.to_string())?;
    Ok(service.measure(a, b))
}

#[tauri::command]
fn snap_world(
    state: State<AppState>,
    point: [f64; 2],
    grid_m: f64,
) -> Result<[f64; 2], String> {
    let service = state.service.lock().map_err(|e| e.to_string())?;
    Ok(service.snap(point, grid_m))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            project_open,
            project_save,
            document_snapshot,
            document_validate,
            document_undo,
            document_redo,
            compile_build_ir,
            build_plan,
            reference_add,
            reference_calibrate,
            measure_world,
            snap_world,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
