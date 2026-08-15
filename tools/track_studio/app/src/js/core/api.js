// Typed Tauri IPC surface for Track Studio.
//
// This is the only place the frontend talks to Rust. It mirrors the
// `#[tauri::command]` functions in `src-tauri/src/lib.rs`. No SQL or domain
// rules live in the frontend.
import { invoke } from "@tauri-apps/api/core";
export const api = {
    projectOpen(path) {
        return invoke("project_open", { path });
    },
    projectSave() {
        return invoke("project_save");
    },
    snapshot() {
        return invoke("document_snapshot");
    },
    validate() {
        return invoke("document_validate");
    },
    undo() {
        return invoke("document_undo");
    },
    redo() {
        return invoke("document_redo");
    },
    compileBuildIr() {
        return invoke("compile_build_ir");
    },
    buildPlan() {
        return invoke("build_plan");
    },
    referenceAdd(imagePath, x, z, opacity) {
        return invoke("reference_add", { imagePath, x, z, opacity });
    },
    referenceCalibrate(layerId, imageDistancePx, realDistanceM) {
        return invoke("reference_calibrate", {
            layerId,
            imageDistancePx,
            realDistanceM,
        });
    },
    measureWorld(a, b) {
        return invoke("measure_world", { a, b });
    },
    snapWorld(point, gridM) {
        return invoke("snap_world", { point, gridM });
    },
};
