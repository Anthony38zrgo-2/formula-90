import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import CadViewPanels from './CadViewPanels.vue'
import { sanitizeCadSvg } from './svgSanitizer'
import type { CadResponse } from './types'

const svg = (view: string) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1" data-view-id="view-${view}"><g><path id="component-nose" data-component-id="component-nose" d="M 0 0 L 1 1"/></g></svg>`
const cad: CadResponse = {
  manifest: {
    document_sha256: 'A'.repeat(64),
    views: ['top', 'side', 'front', 'rear'].map((kind) => ({
      view_id: `view-${kind}`, sha256: 'B'.repeat(64), edge_count: 1,
    })),
  },
  views: { top: svg('top'), side: svg('side'), front: svg('front'), rear: svg('rear') },
}

describe('linked CAD panels', () => {
  it('synchronizes selection across all four views', async () => {
    const wrapper = mount(CadViewPanels, { props: { cad } })
    await wrapper.find('[data-component-id="component-nose"]').trigger('click')
    await wrapper.vm.$nextTick()
    expect(wrapper.text()).toContain('component-nose')
    expect(wrapper.findAll('.cad-selected')).toHaveLength(4)
    expect(wrapper.emitted('command')?.[0]).toEqual([{ type: 'select_component', component_id: 'component-nose' }])
  })

  it('sanitizes executable SVG content', () => {
    const result = sanitizeCadSvg('<svg xmlns="http://www.w3.org/2000/svg" onload="alert(1)"><script>alert(1)</script><path data-component-id="safe" onclick="bad()" d="M 0 0"/></svg>')
    expect(result).not.toContain('script')
    expect(result).not.toContain('onload')
    expect(result).not.toContain('onclick')
    expect(result).toContain('data-component-id="safe"')
  })
})


