
<script setup lang="ts">
import { defineAsyncComponent, onMounted, ref } from 'vue'
import { compileBuildIR, materializeBuild, compileCadViews, compileInitialRevision, getDimensionProfiles, getPresets, scanPreset, sendOnboardingCommand } from './api'
import CadViewPanels from './CadViewPanels.vue'
import DiagnosticsPanel from './DiagnosticsPanel.vue'
import ProjectInspector from './ProjectInspector.vue'
import OnboardingPanel from './OnboardingPanel.vue'
import ParameterInspector from './ParameterInspector.vue'
import type { BuildIRResponse, MaterializationResponse, CadResponse, DimensionProfile, OnboardingResponse, Preset, ProjectSnapshot } from './types'

const Preview3D = defineAsyncComponent(() => import('./Preview3D.vue'))

const presets = ref<Preset[]>([])
const dimensionProfiles = ref<DimensionProfile[]>([])
const session = ref<OnboardingResponse | null>(null)
const snapshot = ref<ProjectSnapshot | null>(null)
const cad = ref<CadResponse | null>(null)
const buildPlan = ref<BuildIRResponse | null>(null)
const materialization = ref<MaterializationResponse | null>(null)
const workspaceMode = ref<'cad' | 'preview'>('cad')
const busy = ref(false)
const errorMessage = ref('')

onMounted(async () => {
  try {
    presets.value = await getPresets()
    dimensionProfiles.value = await getDimensionProfiles()
  } catch (error) {
    errorMessage.value = messageOf(error)
  }
})

async function startScan(presetId: string) {
  busy.value = true
  errorMessage.value = ''
  try {
    session.value = await scanPreset(presetId)
  } catch (error) {
    errorMessage.value = messageOf(error)
  } finally {
    busy.value = false
  }
}

async function performCommand(command: Record<string, unknown>) {
  if (!session.value) return
  session.value.onboarding = await sendOnboardingCommand(
    session.value.session_id,
    session.value.onboarding.revision,
    command,
  )
}

async function applyCommand(command: Record<string, unknown>) {
  busy.value = true
  errorMessage.value = ''
  try {
    await performCommand(command)
  } catch (error) {
    errorMessage.value = messageOf(error)
  } finally {
    busy.value = false
  }
}

async function confirmReferences() {
  busy.value = true
  errorMessage.value = ''
  try {
    await performCommand({ type: 'set_axes', right_axis: '+X', up_axis: '+Y', forward_axis: '-Z' })
    await performCommand({ type: 'set_ground', ground_y_m: session.value?.scan.source.baseline_dimensions.ground_y_m ?? 0 })
    await performCommand({ type: 'set_symmetry', symmetry_plane_x_m: 0 })
  } catch (error) {
    errorMessage.value = messageOf(error)
  } finally {
    busy.value = false
  }
}

async function compileRevision() {
  if (!session.value) return
  busy.value = true
  errorMessage.value = ''
  try {
    snapshot.value = await compileInitialRevision(session.value.session_id, 'williams94')
  } catch (error) {
    errorMessage.value = messageOf(error)
  } finally {
    busy.value = false
  }
}

async function generateCadViews() {
  if (!session.value) return
  busy.value = true
  errorMessage.value = ''
  try {
    cad.value = await compileCadViews(session.value.session_id)
  } catch (error) {
    errorMessage.value = messageOf(error)
  } finally {
    busy.value = false
  }
}

async function compilePlan(values: Record<string, number>, widthPolicies: Record<string, string>) {
  if (!session.value) return
  busy.value = true
  errorMessage.value = ''
  try {
    buildPlan.value = await compileBuildIR(session.value.session_id, values, widthPolicies)
    materialization.value = null
  } catch (error) {
    errorMessage.value = messageOf(error)
  } finally {
    busy.value = false
  }
}

async function materializePlan() {
  if (!session.value || !buildPlan.value) return
  busy.value = true
  errorMessage.value = ''
  try {
    materialization.value = await materializeBuild(session.value.session_id)
  } catch (error) {
    errorMessage.value = messageOf(error)
  } finally {
    busy.value = false
  }
}

function messageOf(error: unknown) {
  return error instanceof Error ? error.message : 'Ocurrió un error inesperado.'
}
</script>

<template>
  <main class="min-h-screen bg-workspace">
    <header class="flex h-16 items-center justify-between border-b border-white/10 px-5">
      <div>
        <p class="text-xs uppercase tracking-[0.22em] text-signal">Formula 90</p>
        <h1 class="font-semibold">Vehicle Studio</h1>
      </div>
      <span class="font-mono text-xs text-slate-500">
        {{ session ? `REV ${session.onboarding.revision}` : 'LOCAL' }}
      </span>
    </header>

    <p v-if="errorMessage" role="alert" class="m-5 rounded border border-red-400/40 bg-red-950/30 p-3 text-sm text-red-200">
      {{ errorMessage }}
    </p>

    <section v-if="snapshot" class="grid min-h-[calc(100vh-4rem)] grid-cols-[1fr_20rem] grid-rows-[1fr_auto]">
      <div v-if="!cad" class="grid place-items-center p-8">
        <div class="w-full max-w-4xl rounded border border-signal/30 bg-panel/60 p-12 text-center">
          <p class="text-xs uppercase tracking-[0.2em] text-signal">Revision zero</p>
          <h2 class="mt-3 text-2xl font-semibold">VehicleDocument canónico creado</h2>
          <p class="mt-2 font-mono text-xs text-slate-400">{{ snapshot.document.revision_id }}</p>
          <button
            class="mt-6 rounded bg-signal px-4 py-2 text-sm font-semibold text-workspace disabled:opacity-40"
            :disabled="busy"
            @click="generateCadViews"
          >
            {{ busy ? 'Proyectando…' : 'Generar vistas CAD' }}
          </button>
        </div>
      </div>
      <div v-else>
        <nav class="flex gap-2 border-b border-white/10 px-5 py-3" aria-label="Modo de visualización">
          <button
            class="rounded px-3 py-2 text-sm"
            :class="workspaceMode === 'cad' ? 'bg-signal text-workspace' : 'border border-white/15'"
            @click="workspaceMode = 'cad'"
          >
            Vistas CAD
          </button>
          <button
            class="rounded px-3 py-2 text-sm"
            :class="workspaceMode === 'preview' ? 'bg-signal text-workspace' : 'border border-white/15'"
            @click="workspaceMode = 'preview'"
          >
            Preview 3D before/after
          </button>
        </nav>
        <CadViewPanels v-if="workspaceMode === 'cad'" :cad="cad" />
        <Preview3D
          v-else
          :key="materialization?.build_ir_sha256 ?? 'source'"
          :variant-source-url="materialization && session ? '/api/assets/variant?session_id=' + session.session_id + '&build=' + materialization.build_ir_sha256 : null"
        />
      </div>
      <aside class="border-l border-white/10 bg-panel">
        <ProjectInspector :snapshot="snapshot" />
        <ParameterInspector
          :parameters="snapshot.document.parameters"
          :profiles="dimensionProfiles"
          :build-plan="buildPlan"
          :busy="busy"
          :materialization="materialization"
          @compile="compilePlan"
          @materialize="materializePlan"
        />
      </aside>
      <DiagnosticsPanel class="col-span-2" :diagnostics="snapshot.diagnostics" />
    </section>

    <OnboardingPanel
      v-else-if="session"
      :onboarding="session.onboarding"
      :suggestions="session.suggestions"
      :busy="busy"
      @command="applyCommand"
      @confirm-references="confirmReferences"
      @compile="compileRevision"
    />

    <section v-else class="grid min-h-[calc(100vh-4rem)] place-items-center p-8">
      <div class="w-full max-w-2xl">
        <p class="font-mono text-xs text-signal">NEW PROJECT</p>
        <h2 class="mt-3 text-3xl font-semibold">Elige el coche que quieres analizar</h2>
        <p class="mt-3 text-slate-400">
          El escaneo es de solo lectura. No guarda ni modifica los GLB originales.
        </p>
        <div class="mt-7 grid gap-3">
          <button
            v-for="preset in presets"
            :key="preset.preset_id"
            class="flex items-center justify-between rounded border border-white/10 bg-panel p-5 text-left hover:border-signal/50 disabled:cursor-not-allowed disabled:opacity-40"
            :disabled="busy || !preset.available"
            @click="startScan(preset.preset_id)"
          >
            <span>
              <strong class="block">{{ preset.label }}</strong>
              <span class="mt-1 block text-sm text-slate-400">{{ preset.source_count }} archivos geométricos</span>
            </span>
            <span class="text-sm text-signal">{{ busy ? 'Analizando…' : preset.available ? 'Analizar' : 'No disponible' }}</span>
          </button>
        </div>
        <p v-if="presets.length === 0 && !errorMessage" class="mt-6 text-sm text-slate-500">Consultando fuentes locales…</p>
      </div>
    </section>
  </main>
</template>
