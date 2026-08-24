



export interface Diagnostic {
  code: string
  severity: 'INFO' | 'WARNING' | 'ERROR' | 'FATAL'
  message: string
  path?: string
}

export interface Parameter {
  parameter_id: string
  semantic_role?: string
  absolute_value: number
  baseline_value: number
  minimum: number
  maximum: number
  unit?: string
}

export interface VehicleDocument {
  schema_version: number
  revision_id: string
  source: { path: string; sha256: string }
  component_map: Array<{ component_id: string; semantic_role: string }>
  parameters: Parameter[]
}

export interface ProjectSnapshot {
  project_id: string
  revision: number
  document_sha256: string
  document: VehicleDocument
  diagnostics: Diagnostic[]
  read_only: boolean
}

export interface Preset {
  preset_id: string
  label: string
  available: boolean
  source_count: number
}

export interface DimensionProfile {
  profile_id: string
  label: string
  targets: Record<string, number>
  width_policies: Record<string, string>
  constraints: Record<string, number>
  notes: string[]
}

export interface SemanticSuggestion {
  suggestion_id: string
  semantic_kind: 'component' | 'frame'
  proposed_role: string
  source_kind: string
  source_name: string
  source_path: string
  confidence: string
}

export interface OnboardingState {
  revision: number
  axes: Record<string, string>
  ground_y_m: number | null
  symmetry_plane_x_m: number | null
  components: Array<{ semantic_role: string; source_name: string }>
  frames: Array<{ semantic_role: string; source_name: string }>
  optional_roles: Record<string, string>
  missing_requirements: string[]
  ready_to_compile: boolean
}

export interface OnboardingResponse {
  session_id: string
  suggestions: SemanticSuggestion[]
  onboarding: OnboardingState
  scan: {
    source: { path: string; sha256: string; files: unknown[]; baseline_dimensions: Record<string, number> }
    findings: Array<{ code: string; severity: string; message: string; source_path: string }>
  }
}




export interface CadResponse {
  manifest: {
    document_sha256: string
    views: Array<{ view_id: string; sha256: string; edge_count: number }>
  }
  views: Record<'top' | 'side' | 'front' | 'rear', string>
}



export interface PreviewRecipe {
  source_url: string
  label: string
  camera: {
    position: [number, number, number]
    target: [number, number, number]
    field_of_view_deg: number
  }
}



export interface BuildIRResponse {
  build_ir_sha256: string
  materialized: boolean
  published: boolean
  build_ir: {
    schema_version: string
    operations: Array<{ operation_id: string; kind: string; postconditions: string[] }>
  }
}



export interface MaterializationResponse {
  build_ir_sha256: string
  materialized: true
  published: false
  report: {
    output_blend: string
    output_glb: string
    source_unchanged: boolean
    artifacts: Record<string, { path: string; sha256: string }>
    ground_contact: { front_y_m: number; rear_y_m: number }
    chassis_height: { front_delta_m: number; rear_delta_m: number; rake_delta_rad: number }
    measured_dimensions_m: Record<string, number>
  }
}
