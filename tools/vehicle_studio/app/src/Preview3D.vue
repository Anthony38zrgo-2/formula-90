<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import {
  AmbientLight, Box3, Color, DirectionalLight, PerspectiveCamera,
  Scene, Vector3, WebGLRenderer,
} from 'three'
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js'
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js'
import { getPreviewRecipe } from './api'
import type { PreviewRecipe } from './types'

const props = defineProps<{ variantSourceUrl?: string | null }>()

const beforeCanvas = ref<HTMLCanvasElement | null>(null)
const afterCanvas = ref<HTMLCanvasElement | null>(null)
const errorMessage = ref('')
const loading = ref(true)
const disposers: Array<() => void> = []

onMounted(async () => {
  try {
    const recipe = await getPreviewRecipe()
    if (!beforeCanvas.value || !afterCanvas.value) return
    const before = await createViewport(beforeCanvas.value, recipe)
    const afterRecipe = { ...recipe, source_url: props.variantSourceUrl ?? recipe.source_url }
    const after = await createViewport(afterCanvas.value, afterRecipe)
    const synchronize = (source: typeof before, target: typeof after) => {
      target.camera.position.copy(source.camera.position)
      target.camera.quaternion.copy(source.camera.quaternion)
      target.controls.target.copy(source.controls.target)
      target.controls.update()
    }
    before.controls.addEventListener('change', () => synchronize(before, after))
    after.controls.addEventListener('change', () => synchronize(after, before))
    disposers.push(before.dispose, after.dispose)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : 'No se pudo cargar el preview 3D.'
  } finally {
    loading.value = false
  }
})

onBeforeUnmount(() => {
  for (const dispose of disposers) dispose()
})

async function createViewport(canvas: HTMLCanvasElement, recipe: PreviewRecipe) {
  const renderer = new WebGLRenderer({ canvas, antialias: true, alpha: false })
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
  renderer.setClearColor(new Color('#0f1720'))
  const scene = new Scene()
  const camera = new PerspectiveCamera(recipe.camera.field_of_view_deg, 1, 0.01, 100)
  camera.position.fromArray(recipe.camera.position)
  const target = new Vector3().fromArray(recipe.camera.target)
  camera.lookAt(target)
  scene.add(new AmbientLight(0xffffff, 1.8))
  const key = new DirectionalLight(0xffffff, 3.2)
  key.position.set(4, 7, 3)
  scene.add(key)
  const fill = new DirectionalLight(0x8fb8ff, 1.4)
  fill.position.set(-4, 2, -5)
  scene.add(fill)

  const gltf = await new GLTFLoader().loadAsync(recipe.source_url)
  scene.add(gltf.scene)
  const bounds = new Box3().setFromObject(gltf.scene)
  if (bounds.isEmpty()) throw new Error('El GLB de preview no contiene geometría visible.')

  const controls = new OrbitControls(camera, canvas)
  controls.target.copy(target)
  controls.enableDamping = true
  controls.update()
  const resize = () => {
    const width = Math.max(canvas.clientWidth, 1)
    const height = Math.max(canvas.clientHeight, 1)
    renderer.setSize(width, height, false)
    camera.aspect = width / height
    camera.updateProjectionMatrix()
  }
  const observer = new ResizeObserver(resize)
  observer.observe(canvas)
  resize()
  renderer.setAnimationLoop(() => {
    controls.update()
    renderer.render(scene, camera)
  })
  return {
    camera,
    controls,
    dispose: () => {
      observer.disconnect()
      renderer.setAnimationLoop(null)
      controls.dispose()
      renderer.dispose()
      gltf.scene.traverse((object) => {
        const mesh = object as { geometry?: { dispose(): void }; material?: { dispose(): void } | Array<{ dispose(): void }> }
        mesh.geometry?.dispose()
        if (Array.isArray(mesh.material)) mesh.material.forEach((material) => material.dispose())
        else mesh.material?.dispose()
      })
    },
  }
}
</script>

<template>
  <section aria-label="Preview 3D comparativo" class="grid grid-cols-2 gap-4 p-5">
    <p v-if="errorMessage" role="alert" class="col-span-2 rounded border border-red-400/40 bg-red-950/30 p-3 text-sm text-red-200">{{ errorMessage }}</p>
    <p v-if="loading" class="col-span-2 text-sm text-slate-400">Cargando preview 3D…</p>
    <figure class="overflow-hidden rounded border border-white/10 bg-panel">
      <figcaption class="border-b border-white/10 p-3">
        <strong>Antes · fuente</strong>
        <span class="mt-1 block text-xs text-amber-300">Preview navegador, no render autoritativo de Blender</span>
      </figcaption>
      <canvas ref="beforeCanvas" class="block h-[32rem] w-full" />
    </figure>
    <figure class="overflow-hidden rounded border border-white/10 bg-panel">
      <figcaption class="border-b border-white/10 p-3">
        <strong>Después · revisión actual</strong>
        <span class="mt-1 block text-xs text-amber-300">{{ variantSourceUrl ? 'GLB staged exportado por Blender' : 'Sin variante materializada; cámara sincronizada' }}</span>
      </figcaption>
      <canvas ref="afterCanvas" class="block h-[32rem] w-full" />
    </figure>
  </section>
</template>
