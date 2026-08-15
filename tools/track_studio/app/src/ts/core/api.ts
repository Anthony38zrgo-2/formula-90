// Typed Tauri IPC surface for Track Studio.
//
// This is the only place the frontend talks to Rust. It mirrors the
// `#[tauri::command]` functions in `src-tauri/src/lib.rs`. No SQL or domain
// rules live in the frontend.

import { invoke } from "@tauri-apps/api/core";

export type Severity = "Info" | "Warning" | "Error" | "Fatal";

export interface Diagnostic {
  code: string;
  severity: Severity;
  object_id?: string | null;
  station?: number | null;
  message: string;
}

export interface Snapshot {
  track_id: string;
  canonical_json: unknown;
  runtime_metadata: unknown;
  diagnostics: Diagnostic[];
}

export const api = {
  projectOpen(path: string): Promise<string> {
    return invoke<string>("project_open", { path });
  },
  projectSave(): Promise<void> {
    return invoke<void>("project_save");
  },
  snapshot(): Promise<Snapshot> {
    return invoke<Snapshot>("document_snapshot");
  },
  validate(): Promise<Diagnostic[]> {
    return invoke<Diagnostic[]>("document_validate");
  },
  undo(): Promise<void> {
    return invoke<void>("document_undo");
  },
  redo(): Promise<void> {
    return invoke<void>("document_redo");
  },
  compileBuildIr(): Promise<unknown> {
    return invoke<unknown>("compile_build_ir");
  },
  buildPlan(): Promise<string[]> {
    return invoke<string[]>("build_plan");
  },
  referenceAdd(imagePath: string, x: number, z: number, opacity: number): Promise<void> {
    return invoke<void>("reference_add", { imagePath, x, z, opacity });
  },
  referenceCalibrate(
    layerId: string,
    imageDistancePx: number,
    realDistanceM: number,
  ): Promise<void> {
    return invoke<void>("reference_calibrate", {
      layerId,
      imageDistancePx,
      realDistanceM,
    });
  },
  measureWorld(a: [number, number], b: [number, number]): Promise<number> {
    return invoke<number>("measure_world", { a, b });
  },
  snapWorld(point: [number, number], gridM: number): Promise<[number, number]> {
    return invoke<[number, number]>("snap_world", { point, gridM });
  },
};
