import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import OnboardingPanel from './OnboardingPanel.vue'

describe('onboarding bulk confirmation', () => {
  it('emits one deterministic suggestion per unmapped role', async () => {
    const wrapper = mount(OnboardingPanel, {
      props: {
        busy: false,
        onboarding: {
          revision: 0, axes: {}, ground_y_m: null, symmetry_plane_x_m: null,
          components: [], frames: [], optional_roles: {},
          missing_requirements: ['component:nose'], ready_to_compile: false,
        },
        suggestions: [
          { suggestion_id: 's1', semantic_kind: 'component', proposed_role: 'nose', source_kind: 'mesh', source_name: 'NOSE_A', source_path: 'a.glb', confidence: 'high' },
          { suggestion_id: 's2', semantic_kind: 'component', proposed_role: 'nose', source_kind: 'mesh', source_name: 'NOSE_B', source_path: 'b.glb', confidence: 'high' },
          { suggestion_id: 's3', semantic_kind: 'component', proposed_role: 'chassis', source_kind: 'mesh', source_name: 'BODY', source_path: 'c.glb', confidence: 'high' },
        ],
      },
    })
    const button = wrapper.findAll('button').find((item) => item.text().includes('Confirmar todas'))
    expect(button).toBeDefined()
    await button!.trigger('click')
    expect(wrapper.emitted('command')?.[0]).toEqual([{
      type: 'accept_suggestions',
      suggestion_ids: ['s1', 's3'],
    }])
  })
})
