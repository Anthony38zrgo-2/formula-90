import type { BuildIRResponse, MaterializationResponse, CadResponse, OnboardingResponse, OnboardingState, Preset, PreviewRecipe, ProjectSnapshot } from './types'

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    ...init,
    headers: { 'Content-Type': 'application/json', ...(init?.headers ?? {}) },
  })
  const payload = await response.json()
  if (!response.ok) {
    throw new Error(payload?.error?.message ?? `Error HTTP ${response.status}`)
  }
  return payload as T
}

export async function getPresets(): Promise<Preset[]> {
  return (await request<{ presets: Preset[] }>('/api/onboarding/presets')).presets
}

export function scanPreset(presetId: string): Promise<OnboardingResponse> {
  return request('/api/onboarding/scan', {
    method: 'POST',
    body: JSON.stringify({ preset_id: presetId }),
  })
}

export async function sendOnboardingCommand(
  sessionId: string,
  expectedRevision: number,
  command: Record<string, unknown>,
): Promise<OnboardingState> {
  return (
    await request<{ onboarding: OnboardingState }>('/api/onboarding/command', {
      method: 'POST',
      body: JSON.stringify({
        session_id: sessionId,
        expected_revision: expectedRevision,
        command,
      }),
    })
  ).onboarding
}

export function compileInitialRevision(
  sessionId: string,
  projectId: string,
): Promise<ProjectSnapshot> {
  return request('/api/onboarding/compile', {
    method: 'POST',
    body: JSON.stringify({ session_id: sessionId, project_id: projectId }),
  })
}


export function compileCadViews(sessionId: string): Promise<CadResponse> {
  return request('/api/cad/compile', {
    method: 'POST',
    body: JSON.stringify({ session_id: sessionId }),
  })
}



export function getPreviewRecipe(): Promise<PreviewRecipe> {
  return request('/api/preview/recipe')
}



export function compileBuildIR(
  sessionId: string,
  parameterValues: Record<string, number>,
  widthPolicies: Record<string, string>,
): Promise<BuildIRResponse> {
  return request('/api/build/compile', {
    method: 'POST',
    body: JSON.stringify({ session_id: sessionId, parameter_values: parameterValues, width_policies: widthPolicies }),
  })
}



export function materializeBuild(sessionId: string): Promise<MaterializationResponse> {
  return request('/api/build/materialize', {
    method: 'POST',
    body: JSON.stringify({ session_id: sessionId }),
  })
}
