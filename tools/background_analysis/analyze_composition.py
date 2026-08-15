#!/usr/bin/env python3
"""BG3-007: Analiza la composición de las 3 capas contra la referencia."""
from PIL import Image
import numpy as np
import os

BASE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(BASE, "..", "..")

REF_PATH = os.path.join(ROOT, "assets-lowpoly-python", "background", "background_layering_codex", "reference", "00_original_reference.jpg")
SKY_PATH = os.path.join(ROOT, "game", "assets", "backgrounds", "la_chutana_snes_day", "sky.png")
FAR_PATH = os.path.join(ROOT, "game", "assets", "backgrounds", "la_chutana_snes_day", "far_mountains.png")
NEAR_PATH = os.path.join(ROOT, "game", "assets", "backgrounds", "la_chutana_snes_day", "near_mountains.png")

OUT_DIR = os.path.join(ROOT, "reports", "background")
os.makedirs(OUT_DIR, exist_ok=True)

TARGET_W, TARGET_H = 1280, 720

def load_and_resize(path, size=(TARGET_W, TARGET_H)):
    img = Image.open(path).convert("RGBA")
    return img.resize(size, Image.NEAREST)

def analyze_vertical_profile(arr, label):
    h, w = arr.shape[:2]
    alpha = arr[:, :, 3]
    non_transparent = np.any(alpha > 0, axis=1)
    rows = np.where(non_transparent)[0]
    if len(rows) == 0:
        print(f"  {label}: EMPTY")
        return {}
    first, last = rows[0], rows[-1]
    weights = np.sum(alpha > 0, axis=1).astype(float)
    com = np.average(np.arange(h), weights=weights)
    # Find the "horizon" - topmost row with significant opaque content
    opaque_threshold = 128
    opaque_rows = np.where(np.max(alpha, axis=1) > opaque_threshold)[0]
    horizon = opaque_rows[0] if len(opaque_rows) > 0 else first
    print(f"  {label}:")
    print(f"    Content: rows {first}-{last} ({100*first/h:.1f}%-{100*last/h:.1f}%)")
    print(f"    Center of mass: {com:.1f}px ({100*com/h:.1f}%)")
    print(f"    Top opaque row: {horizon}px ({100*horizon/h:.1f}%)")
    return {"first": first, "last": last, "com": com, "horizon": horizon}

def composite_layers(sky, far, near, far_offset_y=0, near_offset_y=0):
    """Composite the 3 layers with vertical offsets (in pixels, negative=up)."""
    result = sky.copy()
    # Apply offset to far mountains
    if far_offset_y != 0:
        far_shifted = Image.new("RGBA", far.size, (0, 0, 0, 0))
        far_shifted.paste(far, (0, far_offset_y))
        far = far_shifted
    if near_offset_y != 0:
        near_shifted = Image.new("RGBA", near.size, (0, 0, 0, 0))
        near_shifted.paste(near, (0, near_offset_y))
        near = near_shifted
    # Paste far over sky (respecting alpha)
    result = Image.alpha_composite(result, far)
    # Paste near over result
    result = Image.alpha_composite(result, near)
    return result

def compare_compositions(comp, ref, label):
    """Compare composite against reference at key vertical positions."""
    comp_arr = np.array(comp.convert("RGB"))
    ref_arr = np.array(ref.convert("RGB").resize((TARGET_W, TARGET_H), Image.NEAREST))
    h = comp_arr.shape[0]
    print(f"\n  {label} vs Reference (pixel colors at center column):")
    for y_pct in [20, 30, 40, 50, 60, 70, 80, 90]:
        y = int(h * y_pct / 100)
        c = comp_arr[y, TARGET_W // 2, :3]
        r = ref_arr[y, TARGET_W // 2, :3]
        dist = np.sqrt(np.sum((c.astype(float) - r.astype(float)) ** 2))
        match = "OK" if dist < 60 else "DIFF"
        print(f"    y={y_pct:3d}%: comp=RGB({c[0]:3d},{c[1]:3d},{c[2]:3d}) ref=RGB({r[0]:3d},{r[1]:3d},{r[2]:3d}) dist={dist:.0f} [{match}]")

def main():
    print("=== BG3-007: Analisis de composicion ===\n")

    # Load images
    ref = Image.open(REF_PATH)
    sky = load_and_resize(SKY_PATH)
    far = load_and_resize(FAR_PATH)
    near = load_and_resize(NEAR_PATH)

    print("--- Perfil vertical de cada capa ---")
    analyze_vertical_profile(np.array(sky), "sky")
    far_profile = analyze_vertical_profile(np.array(far), "far_mountains")
    near_profile = analyze_vertical_profile(np.array(near), "near_mountains")

    print("\n--- Composicion actual (offset_y = 0) ---")
    current = composite_layers(sky, far, near)
    current.save(os.path.join(OUT_DIR, "composite_current.png"))
    compare_compositions(current, ref, "Actual")

    # Try different far_mountains offsets
    print("\n--- Buscando offset_y optimo para far_mountains ---")
    best_offset = 0
    best_score = float("inf")

    for offset in range(-200, 1, 10):
        test = composite_layers(sky, far, near, far_offset_y=offset)
        test_arr = np.array(test.convert("RGB"))
        ref_arr = np.array(ref.convert("RGB").resize((TARGET_W, TARGET_H), Image.NEAREST))
        # Compare the mountain band (y 30%-80%)
        band = slice(int(TARGET_H * 0.3), int(TARGET_H * 0.8))
        diff = np.sqrt(np.mean((test_arr[band].astype(float) - ref_arr[band].astype(float)) ** 2))
        if diff < best_score:
            best_score = diff
            best_offset = offset

    print(f"  Mejor offset para far_mountains: {best_offset}px (RMSE={best_score:.1f})")
    optimized = composite_layers(sky, far, near, far_offset_y=best_offset)
    optimized.save(os.path.join(OUT_DIR, "composite_optimized_far.png"))
    compare_compositions(optimized, ref, "Optimizado (solo far)")

    # Try combined far + near offsets
    print("\n--- Buscando offset combinado far + near ---")
    best_combo = (0, 0)
    best_combo_score = float("inf")

    for far_off in range(-200, 1, 20):
        for near_off in range(-100, 50, 20):
            test = composite_layers(sky, far, near, far_offset_y=far_off, near_offset_y=near_off)
            test_arr = np.array(test.convert("RGB"))
            ref_arr = np.array(ref.convert("RGB").resize((TARGET_W, TARGET_H), Image.NEAREST))
            band = slice(int(TARGET_H * 0.25), int(TARGET_H * 0.85))
            diff = np.sqrt(np.mean((test_arr[band].astype(float) - ref_arr[band].astype(float)) ** 2))
            if diff < best_combo_score:
                best_combo_score = diff
                best_combo = (far_off, near_off)

    print(f"  Mejor combo: far={best_combo[0]}px, near={best_combo[1]}px (RMSE={best_combo_score:.1f})")
    combo = composite_layers(sky, far, near, far_offset_y=best_combo[0], near_offset_y=best_combo[1])
    combo.save(os.path.join(OUT_DIR, "composite_optimized_combo.png"))
    compare_compositions(combo, ref, "Optimizado (combo)")

    # Fine-tune around best combo
    print("\n--- Fine-tuning alrededor del mejor combo ---")
    best_fine = best_combo
    best_fine_score = best_combo_score

    for far_off in range(best_combo[0] - 30, best_combo[0] + 30, 5):
        for near_off in range(best_combo[1] - 20, best_combo[1] + 20, 5):
            test = composite_layers(sky, far, near, far_offset_y=far_off, near_offset_y=near_off)
            test_arr = np.array(test.convert("RGB"))
            ref_arr = np.array(ref.convert("RGB").resize((TARGET_W, TARGET_H), Image.NEAREST))
            band = slice(int(TARGET_H * 0.25), int(TARGET_H * 0.85))
            diff = np.sqrt(np.mean((test_arr[band].astype(float) - ref_arr[band].astype(float)) ** 2))
            if diff < best_fine_score:
                best_fine_score = diff
                best_fine = (far_off, near_off)

    print(f"  Fine-tune: far={best_fine[0]}px, near={best_fine[1]}px (RMSE={best_fine_score:.1f})")
    final = composite_layers(sky, far, near, far_offset_y=best_fine[0], near_offset_y=best_fine[1])
    final.save(os.path.join(OUT_DIR, "composite_final.png"))
    compare_compositions(final, ref, "Final")

    # Convert pixel offsets to world-space offset_y for background.json
    # Sprite3D: offset is in sprite-local units
    # pixel_offset / (sprite_height_pixels / sprite_world_height) = world_offset
    # sprite_world_height = texture_height * pixel_size * scale
    # For our case: pixel_size=0.5, scale=1.0
    # sprite_world_height = 720 * 0.5 * 1.0 = 360 units
    # world_offset_y = pixel_offset * (sprite_world_height / texture_height) = pixel_offset * 0.5
    print("\n=== AJUSTES RECOMENDADOS PARA background.json ===")
    print(f"  far_mountains.offset_y: {best_fine[0] * 0.5:.1f} (pixel offset: {best_fine[0]})")
    print(f"  near_mountains.offset_y: {best_fine[1] * 0.5:.1f} (pixel offset: {best_fine[1]})")
    print(f"  sky.offset_y: 0.0 (sin cambio)")

    # Save summary
    with open(os.path.join(OUT_DIR, "bg3_007_analysis.txt"), "w") as f:
        f.write("BG3-007 Composition Analysis\n")
        f.write("============================\n\n")
        f.write(f"Reference: {REF_PATH}\n")
        f.write(f"Far mountains best offset: {best_fine[0]}px -> {best_fine[0] * 0.5:.1f} world units\n")
        f.write(f"Near mountains best offset: {best_fine[1]}px -> {best_fine[1] * 0.5:.1f} world units\n")
        f.write(f"Final RMSE: {best_fine_score:.1f}\n")
        f.write(f"\nOutputs:\n  composite_current.png\n  composite_optimized_far.png\n  composite_optimized_combo.png\n  composite_final.png\n")

    print(f"\nReportes guardados en: {OUT_DIR}")

if __name__ == "__main__":
    main()
