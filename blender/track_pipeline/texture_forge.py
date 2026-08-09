from __future__ import annotations

from dataclasses import dataclass, asdict
from pathlib import Path
import hashlib
import json

import cv2
import numpy as np
from PIL import Image

FORGE_VERSION = 1
STYLE_ID = "ps1_rally_clean"

BAYER_4 = np.array(
    [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]],
    dtype=np.float32,
) / 16.0


@dataclass(frozen=True)
class CardRecipe:
    posterize_levels: int = 18
    dither_strength: float = 0.018
    alpha_threshold: int = 42
    silhouette_resolution: int = 64
    fake_light_strength: float = 0.16
    fake_ao_strength: float = 0.20
    edge_darkening: float = 0.07
    lower_shadow: float = 0.09


@dataclass(frozen=True)
class SurfaceRecipe:
    posterize_levels: int = 28
    dither_strength: float = 0.010
    contrast: float = 1.03


def stable_seed(base_seed: int, label: str) -> int:
    digest = hashlib.sha256(f"{int(base_seed)}:{label}".encode("utf-8")).digest()
    return int.from_bytes(digest[:8], "little", signed=False)


def rng_for(base_seed: int, label: str) -> np.random.Generator:
    return np.random.default_rng(stable_seed(base_seed, label))


def low_frequency_field(rng: np.random.Generator, size: int, coarse: int = 9, blur: float | None = None) -> np.ndarray:
    coarse = max(3, int(coarse))
    small = rng.random((coarse, coarse)).astype(np.float32)
    image = Image.fromarray((small * 255.0).astype(np.uint8), "L").resize((size, size), Image.Resampling.BICUBIC)
    if blur is not None and blur > 0:
        arr = cv2.GaussianBlur(np.asarray(image, dtype=np.float32) / 255.0, (0, 0), blur)
    else:
        arr = np.asarray(image, dtype=np.float32) / 255.0
    return np.clip(arr, 0.0, 1.0)


def _ordered_dither(rgb: np.ndarray, strength: float) -> np.ndarray:
    h, w = rgb.shape[:2]
    tiled = np.tile(BAYER_4, (int(np.ceil(h / 4)), int(np.ceil(w / 4))))[:h, :w]
    offset = (tiled - 0.5) * float(strength)
    return np.clip(rgb + offset[..., None], 0.0, 1.0)


def _posterize(rgb: np.ndarray, levels: int) -> np.ndarray:
    levels = max(2, int(levels))
    return np.round(np.clip(rgb, 0.0, 1.0) * (levels - 1)) / float(levels - 1)


def _clean_alpha(alpha: np.ndarray, target_size: tuple[int, int], threshold: int, silhouette_resolution: int) -> np.ndarray:
    h, w = target_size
    small_w = max(16, min(w, int(silhouette_resolution)))
    small_h = max(16, min(h, int(round(small_w * h / max(w, 1)))))
    small = cv2.resize(alpha, (small_w, small_h), interpolation=cv2.INTER_AREA)
    mask = (small >= int(threshold)).astype(np.uint8) * 255
    kernel = np.ones((3, 3), dtype=np.uint8)
    mask = cv2.morphologyEx(mask, cv2.MORPH_CLOSE, kernel, iterations=1)
    mask = cv2.morphologyEx(mask, cv2.MORPH_OPEN, kernel, iterations=1)
    return cv2.resize(mask, (w, h), interpolation=cv2.INTER_NEAREST)


def _fake_card_lighting(rgb: np.ndarray, alpha: np.ndarray, recipe: CardRecipe) -> np.ndarray:
    mask = (alpha > 0).astype(np.uint8)
    if not mask.any():
        return rgb
    dist = cv2.distanceTransform(mask, cv2.DIST_L2, 3).astype(np.float32)
    if float(dist.max()) > 1e-6:
        dist /= float(dist.max())

    h, w = alpha.shape
    yy, xx = np.mgrid[0:h, 0:w].astype(np.float32)
    xn = xx / max(1.0, float(w - 1))
    yn = yy / max(1.0, float(h - 1))
    directional = ((1.0 - xn) * 0.65 + (1.0 - yn) * 0.35) - 0.5
    lower = np.clip((yn - 0.55) / 0.45, 0.0, 1.0)

    eroded = cv2.erode(mask, np.ones((3, 3), np.uint8), iterations=1)
    edge = np.clip(mask - eroded, 0, 1).astype(np.float32)

    shade = (
        1.0
        + directional * float(recipe.fake_light_strength)
        - dist * float(recipe.fake_ao_strength) * 0.42
        - lower * float(recipe.lower_shadow)
        - edge * float(recipe.edge_darkening)
    )
    shaded = np.clip(rgb * shade[..., None], 0.0, 1.0)
    return np.where(mask[..., None] > 0, shaded, rgb)


def forge_card(image: Image.Image, output_size: int, recipe: CardRecipe = CardRecipe()) -> Image.Image:
    rgba = image.convert("RGBA").resize((output_size, output_size), Image.Resampling.LANCZOS)
    arr = np.asarray(rgba, dtype=np.uint8)
    alpha = _clean_alpha(arr[..., 3], (output_size, output_size), recipe.alpha_threshold, recipe.silhouette_resolution)
    rgb = arr[..., :3].astype(np.float32) / 255.0
    rgb = _fake_card_lighting(rgb, alpha, recipe)
    rgb = _ordered_dither(rgb, recipe.dither_strength)
    rgb = _posterize(rgb, recipe.posterize_levels)
    out = np.zeros((output_size, output_size, 4), dtype=np.uint8)
    out[..., :3] = np.clip(rgb * 255.0, 0, 255).astype(np.uint8)
    out[..., 3] = alpha
    return Image.fromarray(out, "RGBA")


def forge_surface(image: Image.Image, recipe: SurfaceRecipe = SurfaceRecipe()) -> Image.Image:
    rgb = np.asarray(image.convert("RGB"), dtype=np.float32) / 255.0
    rgb = np.clip((rgb - 0.5) * float(recipe.contrast) + 0.5, 0.0, 1.0)
    rgb = _ordered_dither(rgb, recipe.dither_strength)
    rgb = _posterize(rgb, recipe.posterize_levels)
    return Image.fromarray(np.clip(rgb * 255.0, 0, 255).astype(np.uint8), "RGB")


def write_recipe_manifest(path: Path, *, seed: int, entries: dict[str, dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "forge_version": FORGE_VERSION,
        "style": STYLE_ID,
        "seed": int(seed),
        "entries": entries,
    }
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")


def recipe_dict(recipe) -> dict:
    return asdict(recipe)
