<script setup lang="ts">
import { computed } from 'vue'
import type { OnboardingState, SemanticSuggestion } from './types'

const props = defineProps<{
  onboarding: OnboardingState
  suggestions: SemanticSuggestion[]
  busy: boolean
}>()
const emit = defineEmits<{
  command: [command: Record<string, unknown>]
  confirmReferences: []
  compile: []
}>()

const mappedRoles = computed(() => new Set([
  ...props.onboarding.components.map((item) => item.semantic_role),
  ...props.onboarding.frames.map((item) => item.semantic_role),
]))

const bulkSuggestionIds = computed(() => {
  const roles = new Set(mappedRoles.value)
  const ids: string[] = []
  for (const suggestion of props.suggestions) {
    if (!roles.has(suggestion.proposed_role)) {
      roles.add(suggestion.proposed_role)
      ids.push(suggestion.suggestion_id)
    }
  }
  return ids
})
</script>

<template>
  <section class="grid gap-5 p-5">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div>
        <p class="text-xs uppercase tracking-[0.2em] text-signal">Onboarding semántico</p>
        <h2 class="mt-1 text-2xl font-semibold">Confirma lo que detectó el escáner</h2>
      </div>
      <span
        class="rounded px-3 py-2 text-sm"
        :class="onboarding.ready_to_compile ? 'bg-signal/15 text-signal' : 'bg-amber-400/10 text-amber-300'"
      >
        {{ onboarding.ready_to_compile ? 'Mapping completo' : `${onboarding.missing_requirements.length} pendientes` }}
      </span>
    </div>

    <div class="rounded border border-white/10 bg-panel p-4">
      <div class="flex items-center justify-between gap-4">
        <div>
          <h3 class="font-semibold">Sistema de referencia</h3>
          <p class="mt-1 text-sm text-slate-400">+X derecha, +Y arriba, −Z delante; suelo Y=0 y simetría X=0.</p>
        </div>
        <button
          class="rounded border border-signal/50 px-3 py-2 text-sm text-signal disabled:opacity-40"
          :disabled="busy || Boolean(onboarding.ground_y_m !== null && onboarding.symmetry_plane_x_m !== null && onboarding.axes.right_axis)"
          @click="emit('confirmReferences')"
        >
          Confirmar referencias
        </button>
      </div>
    </div>

    <div class="rounded border border-white/10 bg-panel p-4">
      <div class="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h3 class="font-semibold">Sugerencias con evidencia</h3>
          <p class="mt-1 text-sm text-slate-400">Se elegirá una propuesta determinista por rol; cada evidencia permanece visible.</p>
        </div>
        <button
          class="rounded bg-signal px-4 py-2 text-sm font-semibold text-workspace disabled:cursor-not-allowed disabled:opacity-30"
          :disabled="busy || bulkSuggestionIds.length === 0"
          @click="emit('command', { type: 'accept_suggestions', suggestion_ids: bulkSuggestionIds })"
        >
          Confirmar todas las piezas ({{ bulkSuggestionIds.length }})
        </button>
      </div>
      <ul class="mt-4 grid gap-2">
        <li
          v-for="item in suggestions"
          :key="item.suggestion_id"
          class="grid grid-cols-[1fr_auto] items-center gap-4 rounded bg-panel-soft px-3 py-3"
        >
          <div class="min-w-0">
            <p class="font-mono text-sm text-signal">{{ item.proposed_role }}</p>
            <p class="truncate text-xs text-slate-400">{{ item.source_name }} · {{ item.source_path }}</p>
          </div>
          <button
            class="rounded px-3 py-2 text-sm"
            :class="mappedRoles.has(item.proposed_role) ? 'bg-signal/15 text-signal' : 'border border-white/15 hover:border-signal/50'"
            :disabled="busy || mappedRoles.has(item.proposed_role)"
            @click="emit('command', { type: 'accept_suggestion', suggestion_id: item.suggestion_id })"
          >
            {{ mappedRoles.has(item.proposed_role) ? 'Confirmado' : 'Confirmar' }}
          </button>
        </li>
      </ul>
    </div>

    <div class="rounded border border-white/10 bg-panel p-4">
      <h3 class="font-semibold">Roles opcionales</h3>
      <div class="mt-3 flex flex-wrap gap-3">
        <button
          v-for="role in ['cockpit', 'driver']"
          :key="role"
          class="rounded border border-white/15 px-3 py-2 text-sm disabled:border-signal/40 disabled:text-signal"
          :disabled="busy || Boolean(onboarding.optional_roles[role])"
          @click="emit('command', { type: 'set_optional_role', semantic_role: role, status: 'absent' })"
        >
          {{ onboarding.optional_roles[role] ? `${role}: ${onboarding.optional_roles[role]}` : `Marcar ${role} como ausente` }}
        </button>
      </div>
    </div>

    <div class="flex items-center justify-between rounded border border-white/10 bg-panel p-4">
      <div>
        <h3 class="font-semibold">Revisión inicial</h3>
        <p class="mt-1 text-sm text-slate-400">Compila el mapping confirmado a VehicleDocument canónico.</p>
      </div>
      <button
        class="rounded bg-signal px-4 py-2 text-sm font-semibold text-workspace disabled:cursor-not-allowed disabled:opacity-30"
        :disabled="busy || !onboarding.ready_to_compile"
        @click="emit('compile')"
      >
        Crear revisión inicial
      </button>
    </div>

    <details class="rounded border border-white/10 bg-panel p-4">
      <summary class="cursor-pointer text-sm font-semibold">Pendientes exactos</summary>
      <ul class="mt-3 grid gap-1 font-mono text-xs text-amber-200">
        <li v-for="item in onboarding.missing_requirements" :key="item">{{ item }}</li>
      </ul>
    </details>
  </section>
</template>




