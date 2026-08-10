from __future__ import annotations

import cv2
import numpy as np


CYAN_ELECTRIC_RGB = np.array([0, 255, 255], dtype=np.uint8)
CYAN_ELECTRIC_HEX = "#00FFFF"
LEGACY_MAGENTA_RGB = np.array([255, 0, 255], dtype=np.uint8)
DEFAULT_BACKGROUND_RGB = CYAN_ELECTRIC_RGB
DEFAULT_BACKGROUND_HEX = CYAN_ELECTRIC_HEX
DEFAULT_BACKGROUND_NAME = "electric_cyan"
POSTPROCESS_ID = "vegetation_precut_rgba_premultiplied_resize"
POSTPROCESS_VERSION = 4


def _as_key_rgb(value) -> np.ndarray:
    key = np.asarray(value, dtype=np.int32)
    if key.size != 3:
        raise ValueError(f"Expected three key channels, got shape={key.shape}")
    key = key.reshape(3)
    if np.any((key < 0) | (key > 255)):
        raise ValueError(f"Key RGB channels must be in 0..255, got {key.tolist()}")
    return key.astype(np.uint8)


def key_name_for_rgb(key_rgb) -> str:
    key = _as_key_rgb(key_rgb)
    if np.array_equal(key, DEFAULT_BACKGROUND_RGB):
        return DEFAULT_BACKGROUND_NAME
    if np.array_equal(key, LEGACY_MAGENTA_RGB):
        return "legacy_magenta"
    return "custom"


def _key_channel_metrics(rgb_i: np.ndarray, key_rgb: np.ndarray) -> tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray]:
    key_i = key_rgb.astype(np.int32)
    distance = np.sqrt(((rgb_i - key_i) ** 2).sum(axis=-1)).astype(np.float32)
    dominant = np.max(key_i)
    weak = np.min(key_i)
    dominant_channels = key_i == dominant
    weak_channels = key_i == weak
    strong_signal = rgb_i[..., dominant_channels].mean(axis=-1)
    weak_signal = rgb_i[..., weak_channels].mean(axis=-1)
    spread = np.ptp(rgb_i[..., dominant_channels], axis=-1) if dominant_channels.sum() > 1 else np.zeros(distance.shape, dtype=np.int32)
    return distance, strong_signal, weak_signal, spread


def key_candidate_mask(rgb: np.ndarray, key_rgb=DEFAULT_BACKGROUND_RGB, tolerance: float = 170.0) -> np.ndarray:
    rgb_i = rgb.astype(np.int32)
    key_rgb = _as_key_rgb(key_rgb)
    distance, strong_signal, weak_signal, spread = _key_channel_metrics(rgb_i, key_rgb)
    return (
        (distance <= tolerance)
        & (strong_signal >= 140)
        & (weak_signal <= 105)
        & ((strong_signal - weak_signal) >= 55)
        & (spread <= 95)
    )


def transparent_edge(alpha: np.ndarray, radius: int) -> np.ndarray:
    transparent = (alpha == 0).astype(np.uint8)
    kernel = np.ones((radius * 2 + 1, radius * 2 + 1), dtype=np.uint8)
    return cv2.dilate(transparent, kernel).astype(bool)


def border_connected(mask: np.ndarray) -> np.ndarray:
    mask8 = mask.astype(np.uint8)
    labels_count, labels, _, _ = cv2.connectedComponentsWithStats(mask8, connectivity=8)
    if labels_count <= 1:
        return np.zeros_like(mask, dtype=bool)
    border_labels = set(np.unique(np.concatenate([
        labels[0, :], labels[-1, :], labels[:, 0], labels[:, -1],
    ])))
    border_labels.discard(0)
    if not border_labels:
        return np.zeros_like(mask, dtype=bool)
    return np.isin(labels, list(border_labels))


def estimate_key_color(rgb: np.ndarray, nominal_rgb=DEFAULT_BACKGROUND_RGB) -> np.ndarray:
    nominal = _as_key_rgb(nominal_rgb).astype(np.int32)
    h, w = rgb.shape[:2]
    band = max(2, int(round(min(h, w) * 0.04)))
    border = np.concatenate([
        rgb[:band].reshape(-1, 3),
        rgb[-band:].reshape(-1, 3),
        rgb[:, :band].reshape(-1, 3),
        rgb[:, -band:].reshape(-1, 3),
    ], axis=0).astype(np.int32)
    plausible = key_candidate_mask(border.reshape(-1, 1, 3).astype(np.uint8), nominal, tolerance=120.0).reshape(-1)
    selected = border[plausible]
    minimum_samples = max(16, int(round(border.shape[0] * 0.02)))
    if selected.shape[0] < minimum_samples:
        return nominal.astype(np.uint8)
    estimate = np.median(selected, axis=0)
    if float(np.linalg.norm(estimate - nominal)) > 72.0:
        return nominal.astype(np.uint8)
    return np.clip(np.rint(estimate), 0, 255).astype(np.uint8)


def _key_regions(rgb: np.ndarray, key_rgb: np.ndarray, pass_index: int) -> tuple[np.ndarray, np.ndarray, np.ndarray, float, float]:
    rgb_i = rgb.astype(np.int32)
    distance, strong_signal, weak_signal, spread = _key_channel_metrics(rgb_i, key_rgb)
    if pass_index == 1:
        core_distance, outer_distance = 52.0, 220.0
        core_floor, possible_floor = 170, 92
        signal_gap_core, signal_gap_possible = 78, 28
        core_spread, possible_spread = 64, 118
    else:
        core_distance, outer_distance = 68.0, 245.0
        core_floor, possible_floor = 145, 72
        signal_gap_core, signal_gap_possible = 62, 20
        core_spread, possible_spread = 82, 138
    signal_gap = strong_signal - weak_signal
    core = (
        (distance <= core_distance)
        & (strong_signal >= core_floor)
        & (signal_gap >= signal_gap_core)
        & (spread <= core_spread)
    )
    possible = (
        (distance <= outer_distance)
        & (strong_signal >= possible_floor)
        & (signal_gap >= signal_gap_possible)
        & (spread <= possible_spread)
    )
    return core, possible, distance, core_distance, outer_distance


def _components_seeded_by_core(possible: np.ndarray, core: np.ndarray) -> np.ndarray:
    labels_count, labels = cv2.connectedComponents(possible.astype(np.uint8), connectivity=8)
    if labels_count <= 1 or not core.any():
        return core.copy()
    seed_labels = np.unique(labels[core])
    seed_labels = seed_labels[seed_labels != 0]
    if seed_labels.size == 0:
        return core.copy()
    return np.isin(labels, seed_labels)


def remove_isolated_key_speckles_rgba(
    rgba: np.ndarray,
    key_rgb=DEFAULT_BACKGROUND_RGB,
    tolerance: float = 170.0,
    max_component_px: int = 24,
    max_bbox_span: int = 8,
) -> tuple[np.ndarray, dict]:
    """Remove tiny isolated visible key-colored speckles left inside already-downscaled legacy cards."""
    out = rgba.copy()
    alpha = out[..., 3]
    visible_candidates = (alpha >= 16) & key_candidate_mask(out[..., :3], key_rgb=key_rgb, tolerance=tolerance)
    labels_count, labels, stats, _ = cv2.connectedComponentsWithStats(visible_candidates.astype(np.uint8), connectivity=8)
    removed = np.zeros(visible_candidates.shape, dtype=bool)
    for label in range(1, labels_count):
        area = int(stats[label, cv2.CC_STAT_AREA])
        width = int(stats[label, cv2.CC_STAT_WIDTH])
        height = int(stats[label, cv2.CC_STAT_HEIGHT])
        if area <= max_component_px and max(width, height) <= max_bbox_span:
            removed |= labels == label
    out[..., 3][removed] = 0
    out = pad_transparent_rgb(out, radius=4)
    return out, {
        "candidate_pixels": int(visible_candidates.sum()),
        "removed_pixels": int(removed.sum()),
        "removed_components": int(np.unique(labels[removed]).size - (1 if removed.any() and 0 in np.unique(labels[removed]) else 0)),
        "max_component_px": int(max_component_px),
        "max_bbox_span": int(max_bbox_span),
    }


def key_background_rgba(
    rgba: np.ndarray,
    background_rgb=DEFAULT_BACKGROUND_RGB,
    pass_index: int = 1,
) -> tuple[np.ndarray, dict]:
    if rgba.ndim != 3 or rgba.shape[2] != 4:
        raise ValueError("Expected an RGBA uint8 image")
    src = rgba.astype(np.uint8, copy=True)
    rgb = src[..., :3]
    source_alpha = src[..., 3].astype(np.float32) / 255.0
    key_rgb = estimate_key_color(rgb, background_rgb)
    core, possible, distance, core_distance, outer_distance = _key_regions(rgb, key_rgb, pass_index)
    affected = _components_seeded_by_core(possible, core)

    strength = np.zeros(distance.shape, dtype=np.float32)
    denom = max(outer_distance - core_distance, 1e-6)
    strength[affected] = np.clip((outer_distance - distance[affected]) / denom, 0.0, 1.0)
    strength[core] = 1.0

    object_alpha = 1.0 - strength
    final_alpha = np.clip(source_alpha * object_alpha, 0.0, 1.0)

    rgb_f = rgb.astype(np.float32) / 255.0
    key_f = key_rgb.astype(np.float32) / 255.0
    mixed = affected & (object_alpha > (1.0 / 255.0)) & (object_alpha < 0.999) & (source_alpha > 0.98)
    if mixed.any():
        a = object_alpha[mixed, None]
        reconstructed = (rgb_f[mixed] - key_f[None, :] * (1.0 - a)) / np.maximum(a, 1.0 / 255.0)
        rgb_f[mixed] = np.clip(reconstructed, 0.0, 1.0)

    out = np.empty_like(src)
    out[..., :3] = np.clip(np.rint(rgb_f * 255.0), 0, 255).astype(np.uint8)
    out[..., 3] = np.clip(np.rint(final_alpha * 255.0), 0, 255).astype(np.uint8)
    out[..., 3][out[..., 3] < 2] = 0
    return out, {
        "key_rgb": key_rgb.tolist(),
        "key_hex": "#%02X%02X%02X" % tuple(int(v) for v in key_rgb),
        "core_pixels": int(core.sum()),
        "affected_pixels": int(affected.sum()),
        "transparent_pixels": int((out[..., 3] == 0).sum()),
    }


def premultiplied_resize(rgba: np.ndarray, size: tuple[int, int]) -> np.ndarray:
    target_w, target_h = int(size[0]), int(size[1])
    arr = rgba.astype(np.float32) / 255.0
    alpha = arr[..., 3:4]
    premult = arr[..., :3] * alpha
    packed = np.concatenate([premult, alpha], axis=-1)
    resized = cv2.resize(packed, (target_w, target_h), interpolation=cv2.INTER_LANCZOS4)
    resized = np.clip(resized, 0.0, 1.0)
    alpha_r = resized[..., 3:4]
    rgb_r = np.zeros_like(resized[..., :3])
    visible = alpha_r[..., 0] > (0.5 / 255.0)
    rgb_r[visible] = resized[..., :3][visible] / np.maximum(alpha_r[visible], 1.0 / 255.0)
    out = np.empty((target_h, target_w, 4), dtype=np.uint8)
    out[..., :3] = np.clip(np.rint(rgb_r * 255.0), 0, 255).astype(np.uint8)
    out[..., 3] = np.clip(np.rint(alpha_r[..., 0] * 255.0), 0, 255).astype(np.uint8)
    out[..., 3][out[..., 3] < 2] = 0
    return out


def pad_transparent_rgb(rgba: np.ndarray, radius: int = 4) -> np.ndarray:
    out = rgba.copy()
    known = out[..., 3] > 0
    if not known.any() or radius <= 0:
        out[~known, :3] = 0
        return out

    rgb_f = out[..., :3].astype(np.float32)
    kernel = np.ones((3, 3), dtype=np.float32)
    for _ in range(int(radius)):
        count = cv2.filter2D(known.astype(np.float32), -1, kernel, borderType=cv2.BORDER_CONSTANT)
        frontier = (~known) & (count > 0)
        if not frontier.any():
            break
        for channel in range(3):
            values = cv2.filter2D(
                rgb_f[..., channel] * known.astype(np.float32),
                -1,
                kernel,
                borderType=cv2.BORDER_CONSTANT,
            )
            rgb_f[..., channel][frontier] = values[frontier] / count[frontier]
        known[frontier] = True

    out[..., :3] = np.clip(np.rint(rgb_f), 0, 255).astype(np.uint8)
    out[(~known), :3] = 0
    out[..., 3] = rgba[..., 3]
    return out


def align_bottom(rgba: np.ndarray) -> tuple[np.ndarray, int]:
    visible = rgba[..., 3] > 0
    ys, _ = np.where(visible)
    if len(ys) == 0:
        raise RuntimeError("Vegetation card became empty")
    shift_down = rgba.shape[0] - 1 - int(ys.max())
    if shift_down <= 0:
        return rgba, 0
    shifted = np.zeros_like(rgba)
    shifted[shift_down:] = rgba[:-shift_down]
    return shifted, int(shift_down)


def prepare_vegetation_card_rgba(
    rgba: np.ndarray,
    output_size: tuple[int, int] = (128, 128),
    background_rgb=None,
    pass_index: int = 1,
) -> tuple[np.ndarray, dict]:
    working = rgba.astype(np.uint8, copy=True)
    metrics: dict = {"source_size": [int(working.shape[1]), int(working.shape[0])]}
    if background_rgb is not None:
        working, key_metrics = key_background_rgba(working, background_rgb, pass_index)
        metrics["key"] = key_metrics
    working = premultiplied_resize(working, output_size)
    working, shift_down = align_bottom(working)
    working = pad_transparent_rgb(working, radius=4)
    visible = working[..., 3] > 0
    ys, xs = np.where(visible)
    if len(xs) == 0:
        raise RuntimeError("Vegetation card became empty")
    metrics.update({
        "output_size": [int(output_size[0]), int(output_size[1])],
        "bbox": [int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1],
        "bottom_gap_px": int(working.shape[0] - (int(ys.max()) + 1)),
        "shift_down_px": shift_down,
        "alpha_nonzero": int(visible.sum()),
    })
    return working, metrics


def postprocess_recipe(pass_index: int = 1, key_rgb=None) -> dict:
    recipe = {
        "id": POSTPROCESS_ID,
        "version": POSTPROCESS_VERSION,
        "pass": int(pass_index),
        "legacy_previous_key_rgb": LEGACY_MAGENTA_RGB.tolist(),
        "resize": "premultiplied_lanczos4",
        "transparent_rgb": "foreground_edge_padding_4px",
        "bottom_anchor": "last_visible_alpha_row",
    }
    if key_rgb is None:
        recipe.update({
            "source_contract": "precut_rgba_transparent",
            "key_stage": "not_applied_to_precut_sources",
        })
    else:
        key_rgb = _as_key_rgb(key_rgb)
        recipe.update({
            "source_contract": "legacy_chroma_compatibility",
            "key_name": key_name_for_rgb(key_rgb),
            "key_rgb": key_rgb.tolist(),
            "key_hex": "#%02X%02X%02X" % tuple(int(v) for v in key_rgb),
            "key_stage": "source_resolution_before_resize",
        })
    return recipe
