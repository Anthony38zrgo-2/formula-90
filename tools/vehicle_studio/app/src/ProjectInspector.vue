<script setup lang="ts">
import type { ProjectSnapshot } from './types'
defineProps<{ snapshot: ProjectSnapshot }>()
</script>

<template>
  <aside aria-label="Inspector del proyecto" class="border-l border-white/10 bg-panel p-5">
    <p class="text-xs uppercase tracking-[0.2em] text-signal">Inspector</p>
    <h2 class="mt-2 text-lg font-semibold">{{ snapshot.project_id }}</h2>
    <dl class="mt-5 grid gap-4 text-sm">
      <div>
        <dt class="text-slate-500">Revisión</dt>
        <dd class="font-mono">{{ snapshot.revision }}</dd>
      </div>
      <div>
        <dt class="text-slate-500">Fuente</dt>
        <dd class="break-all font-mono text-xs">{{ snapshot.document.source.path }}</dd>
      </div>
      <div>
        <dt class="text-slate-500">Componentes</dt>
        <dd>{{ snapshot.document.component_map.length }}</dd>
      </div>
      <div>
        <dt class="text-slate-500">Documento</dt>
        <dd class="break-all font-mono text-xs">{{ snapshot.document_sha256 }}</dd>
      </div>
    </dl>
  </aside>
</template>
