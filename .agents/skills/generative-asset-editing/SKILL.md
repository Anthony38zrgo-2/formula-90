---
name: generative-asset-editing
description: Create or revise Formula-90 raster, texture, sprite, skybox, HUD, card, background, and other visual assets with generative AI. Use whenever an AI-generated asset must start from an existing editable input, preserve provenance, be versioned, validated, and reviewed before Godot integration.
---

# Generative asset editing

Follow [the project manifest](../../../docs/GENERATIVE_ASSET_MANIFEST.md). The
default operation is **edit**, never ungrounded generation.

## Non-negotiable contract

1. Start from a user-provided, licensed, or project-owned initial asset.
2. Keep that input immutable under `reference/`; record its SHA-256.
3. Use the initial asset as an explicit image-edit input. A style description alone
   is not equivalent evidence.
4. Write intermediate candidates under `working/`; write only accepted assets
   under `layers/`, `output/`, or the asset family's declared final directory.
5. Preserve old accepted versions. Create `v2`, `v3`, etc.; do not overwrite an
   active asset without explicit authorization.
6. Record tool, prompt intent, input paths/hashes, transformations and output
   hashes in `generative_asset_manifest.json`.
7. Do not integrate with Godot until structural validation and image-capable or
   user visual review both pass.

## Workflow

Classify the request before creating anything:

- `EDIT`: preserve composition and modify defined regions.
- `STYLE_TRANSFER`: preserve readable content while changing the visual language.
- `COMPOSITE`: combine identified source assets.
- `CUTOUT`: create a transparent isolated asset.
- `PALETTE`, `UPSCALE`, or `CROP`: deterministic post-process after an edit.
- `GENERATE_BASELINE`: exception; allowed only when the user explicitly asks for
  a new baseline. Promote the accepted result to immutable `reference/` before
  it can be used downstream.

Then proceed in this order:

1. Inspect input dimensions, alpha, palette, ownership/license and composition.
2. Create the standard asset-family folders and initialize the manifest.
3. Hash every input; assign each a role such as `edit_target` or
   `canonical_style_reference`.
4. Invoke the image editor with the target image as an explicit input. State the
   protected elements, requested edits, dimensions and output constraints.
5. Save each candidate to `working/`; never silently replace the reference.
6. Apply deterministic transforms separately and record them.
7. Validate file existence, expected dimensions, alpha, palette/nearest-filter
   compatibility, path, hash and layer compositing order.
8. Visually compare the candidate against its target and style reference. Mark it
   `accepted`, `rejected`, or `candidate` in the manifest.
9. Promote only accepted assets and update the Godot-facing manifest/path.

## Pixel-art and layer rules

- Preserve the intended pixel grid; do not blur, anti-alias, or resample an
  accepted pixel-art result.
- Use nearest filtering for Formula-90 pixel-art assets.
- Keep parallax layers independently composable, with transparent empty regions
  where needed. A flattened preview is not a replacement for the layers.
- Do not invent text, logos, gauges, gameplay markers, or collision-relevant
  detail unless the request explicitly includes them.
- Keep source references out of runtime asset folders; retain them under
  `reference/` with provenance.

## Stop conditions

Stop and request direction when the source is missing, ownership is unclear, a
requested edit would destroy the only baseline, or the desired change cannot be
verified from the available image inputs. Do not substitute a fresh generation
for an unavailable edit input.
