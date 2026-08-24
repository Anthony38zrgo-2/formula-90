const ALLOWED_ELEMENTS = new Set(['svg', 'title', 'g', 'path'])
const ALLOWED_ATTRIBUTES = new Set([
  'xmlns', 'viewBox', 'id', 'd', 'fill', 'stroke', 'stroke-width',
  'vector-effect', 'data-schema', 'data-view-id', 'data-document-sha256',
  'data-source-sha256', 'data-component-id',
])

export function sanitizeCadSvg(source: string): string {
  const document = new DOMParser().parseFromString(source, 'image/svg+xml')
  if (document.querySelector('parsererror') || document.documentElement.localName !== 'svg') {
    throw new Error('El servidor devolvió un SVG CAD inválido.')
  }
  sanitizeElement(document.documentElement)
  return new XMLSerializer().serializeToString(document.documentElement)
}

function sanitizeElement(element: Element): void {
  for (const child of [...element.children]) {
    if (!ALLOWED_ELEMENTS.has(child.localName)) {
      child.remove()
      continue
    }
    sanitizeElement(child)
  }
  for (const attribute of [...element.attributes]) {
    if (!ALLOWED_ATTRIBUTES.has(attribute.name)) {
      element.removeAttribute(attribute.name)
    }
  }
}
