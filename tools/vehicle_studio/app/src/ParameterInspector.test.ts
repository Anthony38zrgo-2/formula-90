import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import ParameterInspector from './ParameterInspector.vue'

describe('parameter inspector', () => {
  it('supports percentage sliders and emits absolute values', async () => {
    const wrapper = mount(ParameterInspector, {
      props: {
        busy: false,
        buildPlan: null,
        parameters: [{
          parameter_id: 'parameter-wheelbase', semantic_role: 'wheelbase',
          absolute_value: 2.9, baseline_value: 2.9, minimum: 2.465,
          maximum: 3.335, unit: 'meter',
        }],
      },
    })
    await wrapper.get('input[type="range"]').setValue('105')
    expect(wrapper.text()).toContain('105.0%')
    await wrapper.get('button').trigger('click')
    const command = wrapper.emitted('compile')?.[0]?.[0] as Record<string, number>
    expect(command['parameter-wheelbase']).toBeCloseTo(3.045)
  })

  it('applies the 2009 FW31 profile without compiling implicitly', async () => {
    const wrapper = mount(ParameterInspector, {
      props: {
        busy: false,
        buildPlan: null,
        parameters: [{
          parameter_id: 'parameter-wheelbase', semantic_role: 'wheelbase',
          absolute_value: 2.9, baseline_value: 2.9, minimum: 2.465,
          maximum: 3.77, unit: 'meter',
        }],
        profiles: [{
          profile_id: 'f1-2009-fw31', label: 'F1 2009 · Williams FW31 (3.100 m)',
          targets: { 'parameter-wheelbase': 3.1 }, width_policies: {},
          constraints: {}, notes: [],
        }],
      },
    })
    await wrapper.get('button').trigger('click')
    expect((wrapper.get('input[type="number"]').element as HTMLInputElement).value).toBe('3.1')
    expect(wrapper.emitted('compile')).toBeUndefined()
  })
})
