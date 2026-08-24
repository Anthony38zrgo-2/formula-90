import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, describe, expect, it, vi } from 'vitest'
import App from './App.vue'

afterEach(() => vi.restoreAllMocks())

describe('Vehicle Studio onboarding shell', () => {
  it('loads allowlisted local vehicle presets', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ presets: [{
        preset_id: 'williams94',
        label: 'Williams 1994 — modular GLB',
        available: true,
        source_count: 5,
      }] }),
    }))
    const wrapper = mount(App)
    await flushPromises()
    expect(wrapper.text()).toContain('Williams 1994')
    expect(wrapper.text()).toContain('Analizar')
    expect(wrapper.text()).not.toContain('Abrir snapshot')
  })

  it('shows backend failures without creating a project', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: false,
      status: 503,
      json: async () => ({ error: { message: 'Servicio local no disponible' } }),
    }))
    const wrapper = mount(App)
    await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toContain('Servicio local')
  })
})
