from __future__ import annotations

import hashlib
import json
from pathlib import Path

import cv2
import numpy as np


STYLIZER_ID = "vegetation_ps1_palette_occlusion"
STYLIZER_VERSION = 1
BAYER_4X4 = np.asarray(
    [
        [0, 8, 2, 10],
        [12, 4, 14, 6],
        [3, 11, 1, 9],
        [15, 7, 13, 5],
    ],
    dtype=np.float32,
) / 16.0

CATEGORY_RECIPE = {
    "trees": {"occlusion_strength": 0.34, "dither_limit": 0.34, "shadow_threshold": 12.0},
    "bushes": {"occlusion_strength": 0.38, "dither_limit": 0.36, "shadow_threshold": 10.0},
    "grass": {"occlusion_strength": 0.18, "dither_limit": 0.22, "shadow_threshold": 16.0},
}


def _hex_to_rgb(value: str) -> np.ndarray:
    text = str(value).strip().lstrip("#")
    if len(text) != 6:
        raise ValueError(f"Invalid palette color: {value!r}")
    return np.asarray([int(text[i : i + 2], 16) for i in (0, 2, 4)], dtype=np.float32)


def palette_from_catalog(catalog_path: Path, biome_id: str, category: str) -> tuple[np.ndarray, list[str]]:
    catalog = json.loads(catalog_path.read_text(encoding="utf-8"))
    key = {"trees": "tree", "bushes": "bush", "grass": "grass"}.get(category)
    if key is None:
        raise ValueError(f"Unsupported vegetation palette category: {category}")
    try:
        values = catalog["palette"][biome_id][key]
    except KeyError as exc:
        raise ValueError(f"Missing palette for biome={biome_id} category={category}") from exc
    if not isinstance(values, list) or len(values) < 2:
        raise ValueError(f"Palette must contain at least two colors: {biome_id}/{category}")
    colors = [_hex_to_rgb(value) for value in values]
    if any(np.array_equal(color, [0, 255, 255]) or np.array_equal(color, [255, 0, 255]) for color in colors):
        raise ValueError("Vegetation palette cannot contain chroma-key colors")
    return np.asarray(colors, dtype=np.float32), [str(value).upper() for value in values]


def _luminance(rgb: np.ndarray) -> np.ndarray:
    return rgb[..., 0] * 0.299 + rgb[..., 1] * 0.587 + rgb[..., 2] * 0.114


def _mask_digest(mask: np.ndarray) -> str:
    return hashlib.sha256(np.ascontiguousarray(mask.astype(np.uint8)).tobytes()).hexdigest()


def analyze_shadow_regions(rgba: np.ndarray, threshold: float) -> dict:
    """Build deterministic shadow/occlusion masks without changing the alpha silhouette."""
    if rgba.ndim != 3 or rgba.shape[2] != 4:
        raise ValueError("Expected an RGBA image")
    visible = rgba[..., 3] > 0
    rgb = rgba[..., :3].astype(np.float32)
    luma = _luminance(rgb)
    local_mean = cv2.GaussianBlur(luma, (5, 5), 0)
    visible_luma = luma[visible]
    median_luma = float(np.median(visible_luma)) if visible_luma.size else 0.0
    local_shadow = np.clip((local_mean - luma - threshold) / 36.0, 0.0, 1.0)
    dark_shadow = np.clip((median_luma - luma) / 82.0, 0.0, 1.0)

    height = rgba.shape[0]
    y = np.arange(height, dtype=np.float32)[:, None] / max(height - 1, 1)
    base_contact = np.clip((y - 0.72) / 0.28, 0.0, 1.0)
    base_contact = np.broadcast_to(base_contact, visible.shape)

    visible_u8 = visible.astype(np.uint8)
    distance = cv2.distanceTransform(visible_u8, cv2.DIST_L2, 3)
    max_distance = float(np.percentile(distance[visible], 80)) if visible.any() else 1.0
    interior = np.clip(distance / max(max_distance, 1.0), 0.0, 1.0)

    shadow = np.clip(0.62 * local_shadow + 0.38 * dark_shadow, 0.0, 1.0)
    occlusion = np.clip(0.68 * shadow + 0.22 * base_contact + 0.10 * interior, 0.0, 1.0)
    shadow[~visible] = 0.0
    occlusion[~visible] = 0.0
    shadow_coords = np.column_stack(np.where(shadow >= 0.20)).astype(np.int32)
    occlusion_coords = np.column_stack(np.where(occlusion >= 0.20)).astype(np.int32)
    return {
        "visible": visible,
        "shadow": shadow,
        "occlusion": occlusion,
        "shadow_coords": shadow_coords,
        "occlusion_coords": occlusion_coords,
        "shadow_mask_sha256": _mask_digest(shadow >= 0.20),
        "occlusion_mask_sha256": _mask_digest(occlusion >= 0.20),
    }


def _palette_quantize(rgb: np.ndarray, visible: np.ndarray, palette: np.ndarray, dither_limit: float) -> np.ndarray:
    distances = ((rgb[..., None, :] - palette[None, None, :, :]) ** 2).sum(axis=-1)
    order = np.argsort(distances, axis=-1, kind="stable")
    nearest = order[..., 0]
    second = order[..., 1]
    d1 = np.take_along_axis(distances, nearest[..., None], axis=-1)[..., 0]
    d2 = np.take_along_axis(distances, second[..., None], axis=-1)[..., 0]
    closeness = np.clip(1.0 - (np.sqrt(np.maximum(d2, 0.0)) - np.sqrt(np.maximum(d1, 0.0))) / 48.0, 0.0, dither_limit)
    rows, cols = np.indices(visible.shape)
    choose_second = (BAYER_4X4[rows % 4, cols % 4] < closeness) & visible
    indices = np.where(choose_second, second, nearest)
    output = palette[indices]
    output[~visible] = rgb[~visible]
    return np.clip(np.rint(output), 0, 255).astype(np.uint8)


def stylize_card_rgba(
    rgba: np.ndarray,
    palette: np.ndarray,
    category: str,
    *,
    occlusion_strength: float | None = None,
    dither_limit: float | None = None,
    shadow_threshold: float | None = None,
) -> tuple[np.ndarray, dict]:
    """Apply a deterministic palette, ordered dithering and fake occlusion to visible RGB only."""
    if rgba.dtype != np.uint8 or rgba.ndim != 3 or rgba.shape[2] != 4:
        raise ValueError("Expected uint8 RGBA image")
    if palette.ndim != 2 or palette.shape[1] != 3:
        raise ValueError("Expected an Nx3 palette")
    defaults = CATEGORY_RECIPE.get(category, CATEGORY_RECIPE["trees"])
    occlusion_strength = defaults["occlusion_strength"] if occlusion_strength is None else float(occlusion_strength)
    dither_limit = defaults["dither_limit"] if dither_limit is None else float(dither_limit)
    shadow_threshold = defaults["shadow_threshold"] if shadow_threshold is None else float(shadow_threshold)

    masks = analyze_shadow_regions(rgba, shadow_threshold)
    rgb = rgba[..., :3].astype(np.float32)
    visible = masks["visible"]
    occlusion = masks["occlusion"]
    darkened = rgb * (1.0 - np.clip(occlusion * occlusion_strength, 0.0, 0.85)[..., None])
    quantized = _palette_quantize(darkened, visible, palette.astype(np.float32), dither_limit)

    out = rgba.copy()
    out[..., :3] = quantized
    out[..., 3] = rgba[..., 3]
    return out, {
        "id": STYLIZER_ID,
        "version": STYLIZER_VERSION,
        "category": category,
        "palette_rgb": [[int(round(value)) for value in color] for color in palette],
        "occlusion_strength": occlusion_strength,
        "dither_limit": dither_limit,
        "shadow_threshold": shadow_threshold,
        "visible_pixels": int(visible.sum()),
        "shadow_pixels": int(masks["shadow_coords"].shape[0]),
        "occlusion_pixels": int(masks["occlusion_coords"].shape[0]),
        "shadow_mask_sha256": masks["shadow_mask_sha256"],
        "occlusion_mask_sha256": masks["occlusion_mask_sha256"],
        "alpha_preserved": bool(np.array_equal(out[..., 3], rgba[..., 3])),
        "coordinate_cache": "in_memory_per_asset",
        "dither": "ordered_bayer_4x4",
    }


def stylizer_recipe(category: str, palette_hex: list[str]) -> dict:
    defaults = CATEGORY_RECIPE.get(category, CATEGORY_RECIPE["trees"])
    return {
        "id": STYLIZER_ID,
        "version": STYLIZER_VERSION,
        "category": category,
        "palette_hex": list(palette_hex),
        "occlusion_strength": defaults["occlusion_strength"],
        "dither_limit": defaults["dither_limit"],
        "shadow_threshold": defaults["shadow_threshold"],
        "dither": "ordered_bayer_4x4",
        "coordinates": "cached_in_memory_from_source_card",
        "alpha": "preserved",
    }
