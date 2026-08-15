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


## Metrica estructural: primera fila donde el composite difiere del cielo.
## El RMSE contra la foto de referencia es un oracle pobre (la foto contiene
## pista/coches/edificios ausentes en el arte pixel-art); la posicion de la
## primera montana visible es la metrica estructural que guia el ajuste.
def first_mountain_row_pct(comp, sky):
    comp_arr = np.array(comp.convert("RGBA"))
    sky_arr = np.array(sky.convert("RGBA"))
    h = comp_arr.shape[0]
    for y in range(h):
        comp_row = comp_arr[y, :, :3].astype(float)
        sky_row = sky_arr[y, :, :3].astype(float)
        diff = np.mean(np.abs(comp_row - sky_row))
        if diff > 8.0:
            return 100.0 * y / h
    return 100.0


def report_structure(comp, sky, label):
    pct = first_mountain_row_pct(comp, sky)
    # Banda objetivo derivada de la referencia: montana visible ~35-45%
    if 30.0 <= pct <= 50.0:
        verdict = "OK"
    elif pct < 30.0:
        verdict = "DEMASIADO ALTA"
    else:
        verdict = "DEMASIADO BAJA"
    print(f"  {label}: primera montana visible en {pct:.1f}% desde arriba [{verdict}]")
    return pct

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
    print("\n[Estructura]")
    report_structure(current, sky, "Actual")
    print("\n[RMSE (referencia de color, oracle secundario)]")
    compare_compositions(current, ref, "Actual")

    # Buscar offset_y optimo con la METRICA ESTRUCTURAL (primera montana ~40%).
    # El RMSE contra la foto es un oracle pobre: la foto incluye pista/coches
    # que el arte pixel-art no reproduce. La estructura es la autoridad.
    print("\n--- Buscando offset_y optimo (metrica estructural: primera montana ~40%) ---")
    best_offset = 0
    best_score = float("inf")

    for offset in range(-400, 1, 10):
        test = composite_layers(sky, far, near, far_offset_y=offset)
        pct = first_mountain_row_pct(test, sky)
        score = abs(pct - 40.0)
        if score < best_score:
            best_score = score
            best_offset = offset

    print(f"  Mejor offset para far_mountains: {best_offset}px (primera montana ~{40 + best_score:.0f}%)")
    optimized = composite_layers(sky, far, near, far_offset_y=best_offset)
    optimized.save(os.path.join(OUT_DIR, "composite_optimized_far.png"))
    print("\n[Estructura]")
    report_structure(optimized, sky, "Optimizado (solo far)")
    print("\n[RMSE]")
    compare_compositions(optimized, ref, "Optimizado (solo far)")

    # Buscar offset combinado far + near con la metrica estructural
    print("\n--- Buscando offset combinado far + near (metrica estructural) ---")
    best_combo = (0, 0)
    best_combo_score = float("inf")

    for far_off in range(-400, 1, 20):
        for near_off in range(-300, 100, 20):
            test = composite_layers(sky, far, near, far_offset_y=far_off, near_offset_y=near_off)
            pct = first_mountain_row_pct(test, sky)
            score = abs(pct - 40.0)
            if score < best_combo_score:
                best_combo_score = score
                best_combo = (far_off, near_off)

    print(f"  Mejor combo: far={best_combo[0]}px, near={best_combo[1]}px (primera montana ~{40 + best_combo_score:.0f}%)")
    combo = composite_layers(sky, far, near, far_offset_y=best_combo[0], near_offset_y=best_combo[1])
    combo.save(os.path.join(OUT_DIR, "composite_optimized_combo.png"))
    print("\n[Estructura]")
    report_structure(combo, sky, "Optimizado (combo)")
    print("\n[RMSE]")
    compare_compositions(combo, ref, "Optimizado (combo)")

    # Fine-tuning alrededor del mejor combo
    print("\n--- Fine-tuning alrededor del mejor combo ---")
    best_fine = best_combo
    best_fine_score = best_combo_score

    for far_off in range(best_combo[0] - 60, best_combo[0] + 60, 5):
        for near_off in range(best_combo[1] - 40, best_combo[1] + 40, 5):
            test = composite_layers(sky, far, near, far_offset_y=far_off, near_offset_y=near_off)
            pct = first_mountain_row_pct(test, sky)
            score = abs(pct - 40.0)
            if score < best_fine_score:
                best_fine_score = score
                best_fine = (far_off, near_off)

    print(f"  Fine-tune: far={best_fine[0]}px, near={best_fine[1]}px (primera montana ~{40 + best_fine_score:.0f}%)")
    final = composite_layers(sky, far, near, far_offset_y=best_fine[0], near_offset_y=best_fine[1])
    final.save(os.path.join(OUT_DIR, "composite_final.png"))
    print("\n[Estructura]")
    report_structure(final, sky, "Final")
    print("\n[RMSE]")
    compare_compositions(final, ref, "Final")

    # Convert pixel offsets to world-space offset_y for background.json
    # Sprite3D: offset is in sprite-local units
    # pixel_offset / (sprite_height_pixels / sprite_world_height) = world_offset
    # sprite_world_height = texture_height * pixel_size * scale
    # For our case: pixel_size=0.5, scale=1.0
    # sprite_world_height = 720 * 0.5 * 1.0 = 360 units
    # world_offset_y = pixel_offset * (sprite_world_height / texture_height) = pixel_offset * 0.5
    print("\n=== AJUSTES RECOMENDADOS PARA background.json (referencia estructural) ===")
    print(f"  far_mountains.offset_y: {best_fine[0] * 0.5:.1f} (pixel offset: {best_fine[0]})")
    print(f"  near_mountains.offset_y: {best_fine[1] * 0.5:.1f} (pixel offset: {best_fine[1]})")
    print(f"  sky.offset_y: 0.0 (sin cambio)")

    # Save summary
    with open(os.path.join(OUT_DIR, "bg3_007_analysis.txt"), "w") as f:
        f.write("BG3-007 Composition Analysis\n")
        f.write("============================\n\n")
        f.write("Oracle: metrica estructural (primera fila de montana visible, ~40% objetivo).\n")
        f.write("El RMSE de color contra la foto de referencia es oracle secundario.\n\n")
        f.write(f"Reference: {REF_PATH}\n")
        f.write(f"Far mountains best offset: {best_fine[0]}px -> {best_fine[0] * 0.5:.1f} world units\n")
        f.write(f"Near mountains best offset: {best_fine[1]}px -> {best_fine[1] * 0.5:.1f} world units\n")
        f.write(f"Final structural score (|first_mountain_pct - 40|): {best_fine_score:.1f}\n")
        f.write(f"\nOutputs:\n  composite_current.png\n  composite_optimized_far.png\n  composite_optimized_combo.png\n  composite_final.png\n")

    print(f"\nReportes guardados en: {OUT_DIR}")

if __name__ == "__main__":
    main()
