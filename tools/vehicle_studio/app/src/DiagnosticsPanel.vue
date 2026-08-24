<script setup lang="ts">
import type { Diagnostic } from './types'
defineProps<{ diagnostics: Diagnostic[] }>()
</script>

<template>
  <section aria-label="Diagnósticos" class="border-t border-white/10 bg-panel px-5 py-3">
    <div class="flex items-center justify-between">
      <h2 class="text-sm font-semibold">Diagnósticos</h2>
      <span class="rounded bg-panel-soft px-2 py-1 font-mono text-xs">{{ diagnostics.length }}</span>
    </div>
    <p v-if="diagnostics.length === 0" class="mt-2 text-sm text-slate-400">
      Sin errores de contrato.
    </p>
    <ul v-else class="mt-2 grid gap-2">
      <li v-for="item in diagnostics" :key="`${item.code}:${item.path ?? ''}`" class="text-sm">
        <strong>{{ item.severity }}</strong> · {{ item.code }} — {{ item.message }}
      </li>
    </ul>
  </section>
</template>
