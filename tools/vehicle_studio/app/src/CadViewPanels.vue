<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { sanitizeCadSvg } from './svgSanitizer'
import type { CadResponse } from './types'

const props = defineProps<{ cad: CadResponse }>()
const emit = defineEmits<{ command: [command: { type: 'select_component'; component_id: string | null }] }>()
const root = ref<HTMLElement | null>(null)
const selectedComponentId = ref<string | null>(null)
const kinds = ['top', 'side', 'front', 'rear'] as const
const safeViews = computed(() => Object.fromEntries(
  kinds.map((kind) => [kind, sanitizeCadSvg(props.cad.views[kind])]),
) as Record<(typeof kinds)[number], string>)

function selectFromEvent(event: MouseEvent) {
  const target = event.target instanceof Element
    ? event.target.closest<SVGPathElement>('[data-component-id]')
    : null
  selectedComponentId.value = target?.dataset.componentId ?? null
  emit('command', { type: 'select_component', component_id: selectedComponentId.value })
}

function clearSelection() {
  selectedComponentId.value = null
  emit('command', { type: 'select_component', component_id: null })
}

async function applyHighlight() {
  await nextTick()
  const paths = root.value?.querySelectorAll<SVGPathElement>('[data-component-id]') ?? []
  for (const path of paths) {
    const selected = path.dataset.componentId === selectedComponentId.value
    path.classList.toggle('cad-selected', selected)
    path.classList.toggle('cad-muted', Boolean(selectedComponentId.value) && !selected)
  }
}

watch(selectedComponentId, applyHighlight)
watch(safeViews, applyHighlight, { flush: 'post' })
</script>

<template>
  <section ref="root" aria-label="Vistas CAD enlazadas" class="cad-grid grid grid-cols-2 gap-4 p-5">
    <header class="col-span-2 flex items-center justify-between">
      <div>
        <p class="text-xs uppercase tracking-[0.2em] text-signal">Selección enlazada</p>
        <p class="mt-1 font-mono text-xs text-slate-400">
          {{ selectedComponentId ?? 'Haz clic en una pieza para identificarla' }}
        </p>
      </div>
      <button
        v-if="selectedComponentId"
        class="rounded border border-white/15 px-3 py-2 text-sm"
        @click="clearSelection"
      >
        Limpiar selección
      </button>
    </header>
    <figure
      v-for="kind in kinds"
      :key="kind"
      class="rounded border border-white/10 bg-white p-3 text-workspace"
    >
      <figcaption class="mb-2 flex items-center justify-between">
        <strong class="uppercase">{{ kind }}</strong>
        <span class="font-mono text-[10px] text-slate-500">
          {{ cad.manifest.views.find((view) => view.view_id === `view-${kind}`)?.edge_count }} edges
        </span>
      </figcaption>
      <div
        class="cad-svg [&_svg]:h-64 [&_svg]:w-full"
        @click="selectFromEvent"
        v-html="safeViews[kind]"
      />
    </figure>
  </section>
</template>

<style>
.cad-svg path[data-component-id] { cursor: pointer; transition: opacity 120ms ease, stroke 120ms ease; }
.cad-svg path.cad-muted { opacity: 0.12; }
.cad-svg path.cad-selected { opacity: 1; stroke: #e11d48; stroke-width: 0.008; }
</style>


