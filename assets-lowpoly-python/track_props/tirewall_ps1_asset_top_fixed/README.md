# PS1 Low-Poly Tire Wall Asset

Source: user-provided stacked tire image.

## Included

- `textures/tirewall_front_albedo_128x256.png`
  - Opaque, horizontally seamless.
  - 5 tire rows.
  - Baked shading; no normal map required.
- `textures/tirewall_top_albedo_128x64.png`
  - Opaque, horizontally seamless.
  - Darkened top surface.
- `textures/tirewall_end_albedo_64x256.png`
  - Opaque end-cap texture with stronger edge shading.
- `textures/tirewall_atlas_256x256.png`
  - Optional compact atlas.
- `meshes/tirewall_segment_8tris.glb`
  - 2.0 m wide × 1.25 m high × 0.45 m deep.
  - Front + top + two end faces.
  - Open back.
  - 8 triangles total.
  - Textures embedded in GLB.
- `meshes/tirewall_segment_8tris.obj/.mtl`
  - Editable external-texture version.
- `preview/tirewall_final_preview_1280x720.png`
- `preview/tirewall_textures_contact_sheet.png`

## Godot / PS1-style import

Recommended:
- Texture filter: Nearest, or Nearest + Mipmaps if distant shimmer is excessive.
- Repeat: Enabled for front/top textures.
- Normal map: none.
- Metallic: 0.
- Roughness: 1.
- Disable unnecessary transparency.
- Use the GLB as a repeated/instanced segment.
- If only the track-facing side is visible, the open back is intentional.
- For OBJ, disable back-face culling if you need both sides visible.

## Scale

Default segment:
- Width: 2.00 m
- Height: 1.25 m
- Depth: 0.45 m

The mesh can be scaled non-uniformly in X for longer straight sections, but repeated instances preserve the texture density better.

## Texture goal

The source sprite was converted into flat-wall albedo by:
1. removing transparent lateral waste,
2. stretching every opaque scanline edge-to-edge,
3. forcing left/right seam compatibility,
4. baking the tire curvature into the albedo,
5. reducing the palette/detail to suit a PS1-style track asset.


## Updated top source

- `textures/tirewall_top_albedo_128x64.png` now derives from the user-provided circular tire image (`source_top_reference.png`) using the same flattening, seam-fixing, and PS1-style palette reduction workflow.


## Top fix

- The previous top albedo used a flattened circular cap and produced visible stretching.
- The new top albedo now uses an **unwrapped outer tread band** from the circular tire source, which is much more compatible with a flat rectangular top face.
