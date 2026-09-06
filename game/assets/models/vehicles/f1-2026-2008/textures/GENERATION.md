# Jordan 1997-inspired livery

Generated with the built-in `image_gen` tool; no CLI/API fallback was used.
The image is original artwork inspired by the supplied Jordan 197 references,
not an exact historical livery scan. It contains no sponsors, lettering or logos.

- `jordan_1997_unbranded_atlas.png`: generated source artwork, kept unchanged.
- `jordan_1997_body_albedo.png`: opaque 2048×2048 Blender material bake mapped to
  the restored original body. The GLB embeds this runtime texture.
- `tools/blender/bake_jordan_livery.py`: spatial paint masks, UV projection and
  emission bake. The yellow/red/black palette also applies to the retained wings,
  endplates, mirrors, airbox and exposed carbon surfaces through PBR materials.

## Exact generation prompt

Use case: stylized-concept. Asset type: flat 2D albedo livery texture atlas for a 3D racing car, not a car render. Create one square 2048px texture with TWO full-width horizontal panels, no border, no labels. Top half (y=0 to 50%): a flat side decal of the 1997 Jordan Formula One snake nose artwork on saturated golden yellow background (#ffd000). The snake faces RIGHT, snout at far right, one realistic illustrated green-gold eye near x=76%, a red mouth stripe along lower portion, irregular black-outlined golden yellow reptile scales, black patches around eye and mouth, scales gradually sparse toward left. This is painted motorsport graphics, NOT a photoreal animal head or separate animal object; flat surface artwork with no lighting, bevel, shadows or perspective. At very top and bottom of this top panel keep a small plain-yellow bleed margin. Bottom half (y=50% to 100%): continuous charcoal black snake skin scale field, irregular overlapping black/dark grey scales with fine silver-grey crescent edges, like the sidepod snakeskin on the 1997 Jordan, subtle high contrast edging but mostly black. Bottom panel fills entire width with no illustration or lettering. Uniform albedo artwork ready for UV mapping. Strictly NO text, lettering, numbers, sponsor names, logos, racing badges, watermark, car, wheels, mockup, background scene. Make the two rectangular panels crisp and usable as UV regions. Preserve palette golden yellow, black, red, subtle silver and small green-gold snake eye.
