<script setup lang="ts">
import { reactive } from 'vue'
import type { BuildIRResponse, MaterializationResponse, Parameter } from './types'

const props = defineProps<{
  parameters: Parameter[]
  buildPlan: BuildIRResponse | null
  busy: boolean
  materialization?: MaterializationResponse | null
}>()
const emit = defineEmits<{ compile: [values: Record<string, number>, policies: Record<string, string>]; materialize: [] }>()
const policies = reactive({ front_tire_width: 'centered', rear_tire_width: 'centered' } as Record<string, string>)
const values = reactive(Object.fromEntries(
  props.parameters.map((parameter) => [parameter.parameter_id, parameter.absolute_value]),
) as Record<string, number>)

function percentage(parameter: Parameter) {
  return (values[parameter.parameter_id] / parameter.baseline_value) * 100
}

function setPercentage(parameter: Parameter, event: Event) {
  const percent = Number((event.target as HTMLInputElement).value)
  values[parameter.parameter_id] = parameter.baseline_value * percent / 100
}

function label(role?: string) {
  return (role ?? 'parameter').replaceAll('_', ' ')
}
</script>

<template>
  <section aria-label="Parámetros globales" class="border-t border-white/10 bg-panel p-5">
    <div class="flex items-center justify-between">
      <div>
        <p class="text-xs uppercase tracking-[0.2em] text-signal">BuildIR</p>
        <h2 class="mt-1 font-semibold">Dimensiones globales</h2>
      </div>
      <span class="rounded bg-amber-400/10 px-2 py-1 text-[10px] text-amber-300">STAGING ONLY</span>
    </div>
    <div class="mt-4 grid gap-5">
      <label v-for="parameter in parameters" :key="parameter.parameter_id" class="grid gap-2">
        <span class="flex items-center justify-between text-xs">
          <strong class="capitalize">{{ label(parameter.semantic_role) }}</strong>
          <span class="font-mono text-slate-400">{{ percentage(parameter).toFixed(1) }}%</span>
        </span>
        <input
          v-model.number="values[parameter.parameter_id]"
          class="rounded border border-white/15 bg-workspace px-2 py-1 font-mono text-xs"
          type="number"
          :min="parameter.minimum"
          :max="parameter.maximum"
          :step="(parameter.maximum - parameter.minimum) / 200"
        />
        <input
          :aria-label="`${label(parameter.semantic_role)} porcentaje`"
          type="range"
          :min="parameter.minimum / parameter.baseline_value * 100"
          :max="parameter.maximum / parameter.baseline_value * 100"
          step="0.1"
          :value="percentage(parameter)"
          @input="setPercentage(parameter, $event)"
        />
      </label>
    </div>
    <button
      class="mt-5 w-full rounded bg-signal px-3 py-2 text-sm font-semibold text-workspace disabled:opacity-30"
      :disabled="busy"
      @click="emit('compile', { ...values }, { ...policies })"
    >
      {{ busy ? 'Compilando…' : 'Compilar plan de operaciones' }}
    </button>
    <div v-if="buildPlan" class="mt-4 rounded bg-workspace p-3">
      <p class="text-xs text-signal">{{ buildPlan.build_ir.operations.length }} operaciones válidas</p>
      <p class="mt-1 break-all font-mono text-[9px] text-slate-500">{{ buildPlan.build_ir_sha256 }}</p>
      <p class="mt-2 text-[10px] text-amber-300">Plan compilado · no publicado</p>
      <button
        class="mt-3 w-full rounded border border-signal/60 px-3 py-2 text-xs font-semibold text-signal disabled:opacity-30"
        :disabled="busy"
        @click="emit('materialize')"
      >
        {{ busy ? 'Materializando…' : 'Materializar variante en Blender' }}
      </button>
    </div>
    <div v-if="materialization" class="mt-3 rounded border border-signal/30 bg-signal/5 p-3">
      <p class="text-xs font-semibold text-signal">Variante Blender creada</p>
      <p class="mt-2 break-all font-mono text-[9px] text-slate-400">{{ materialization.report.output_blend }}
      </p>
      <p class="mt-2 break-all font-mono text-[9px] text-slate-400">{{ materialization.report.output_glb }}</p>
      <p class="mt-2 text-[10px] text-slate-300">Fuente intacta · staging sin publicar</p>
    </div>
  </section>
</template>
