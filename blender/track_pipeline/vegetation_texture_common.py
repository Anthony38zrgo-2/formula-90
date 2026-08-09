from __future__ import annotations

import cv2
import numpy as np


MAGENTA_RGB = np.array([255, 0, 255], dtype=np.uint8)
POSTPROCESS_ID = "vegetation_source_key_premultiplied_resize"
POSTPROCESS_VERSION = 2


def _as_key_rgb(value) -> np.ndarray:
    key = np.asarray(value, dtype=np.int32)
    if key.size != 3:
        raise ValueError(f"Expected three key channels, got shape={key.shape}")
    key = key.reshape(3)
    if np.any((key < 0) | (key > 255)):
        raise ValueError(f"Key RGB channels must be in 0..255, got {key.tolist()}")
    return key.astype(np.uint8)


def magenta_key_mask(rgb: np.ndarray, pass_index: int) -> np.ndarray:
    """Legacy-compatible near-magenta detector used only by fallback recut tooling."""
    rgb_i = rgb.astype(np.int32)
    distance = np.sqrt(((rgb_i - MAGENTA_RGB.astype(np.int32)) ** 2).sum(axis=-1))
    r = rgb_i[..., 0]
    g = rgb_i[..., 1]
    b = rgb_i[..., 2]
    min_rb = np.minimum(r, b)
    tolerance = 80.0 if pass_index == 1 else 125.0
    min_rb_floor = 135 if pass_index == 1 else 100
    green_gap = 45 if pass_index == 1 else 30
    channel_spread = 75 if pass_index == 1 else 100
    return (
        (distance <= tolerance)
        & (min_rb >= min_rb_floor)
        & (((r + b) // 2 - g) >= green_gap)
        & (np.abs(r - b) <= channel_spread)
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


def estimate_key_color(rgb: np.ndarray, nominal_rgb=MAGENTA_RGB) -> np.ndarray:
    """Estimate the actual key from border pixels while staying anchored to nominal magenta."""
    nominal = _as_key_rgb(nominal_rgb).astype(np.int32)
    h, w = rgb.shape[:2]
    band = max(2, int(round(min(h, w) * 0.04)))
    border = np.concatenate([
        rgb[:band].reshape(-1, 3),
        rgb[-band:].reshape(-1, 3),
        rgb[:, :band].reshape(-1, 3),
        rgb[:, -band:].reshape(-1, 3),
    ], axis=0).astype(np.int32)
    distance = np.sqrt(((border - nominal) ** 2).sum(axis=1))
    r, g, b = border[:, 0], border[:, 1], border[:, 2]
    plausible = (
        (distance <= 120.0)
        & (np.minimum(r, b) >= 120)
        & ((((r + b) // 2) - g) >= 40)
        & (np.abs(r - b) <= 100)
    )
    selected = border[plausible]
    minimum_samples = max(16, int(round(border.shape[0] * 0.02)))
    if selected.shape[0] < minimum_samples:
        return nominal.astype(np.uint8)
    estimate = np.median(selected, axis=0)
    # Do not let a noisy border redefine the semantic key.
    if float(np.linalg.norm(estimate - nominal)) > 72.0:
        return nominal.astype(np.uint8)
    return np.clip(np.rint(estimate), 0, 255).astype(np.uint8)


def _key_regions(rgb: np.ndarray, key_rgb: np.ndarray, pass_index: int) -> tuple[np.ndarray, np.ndarray, np.ndarray, float, float]:
    rgb_i = rgb.astype(np.int32)
    key_i = key_rgb.astype(np.int32)
    distance = np.sqrt(((rgb_i - key_i) ** 2).sum(axis=-1)).astype(np.float32)
    r, g, b = rgb_i[..., 0], rgb_i[..., 1], rgb_i[..., 2]
    min_rb = np.minimum(r, b)
    magenta_gap = ((r + b) // 2) - g
    spread = np.abs(r - b)

    if pass_index == 1:
        core_distance, outer_distance = 52.0, 220.0
        core_floor, possible_floor = 170, 92
        core_gap, possible_gap = 78, 28
        core_spread, possible_spread = 64, 118
    else:
        core_distance, outer_distance = 68.0, 245.0
        core_floor, possible_floor = 145, 72
        core_gap, possible_gap = 62, 20
        core_spread, possible_spread = 82, 138

    core = (
        (distance <= core_distance)
        & (min_rb >= core_floor)
        & (magenta_gap >= core_gap)
        & (spread <= core_spread)
    )
    possible = (
        (distance <= outer_distance)
        & (min_rb >= possible_floor)
        & (magenta_gap >= possible_gap)
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


def key_background_rgba(
    rgba: np.ndarray,
    background_rgb=MAGENTA_RGB,
    pass_index: int = 1,
) -> tuple[np.ndarray, dict]:
    """Build an alpha matte at source resolution and despill magenta before resizing."""
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
    # Reverse the foreground-over-key mixture for pixels whose original alpha is effectively opaque.
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
        "core_pixels": int(core.sum()),
        "affected_pixels": int(affected.sum()),
        "transparent_pixels": int((out[..., 3] == 0).sum()),
    }


def premultiplied_resize(rgba: np.ndarray, size: tuple[int, int]) -> np.ndarray:
    """Resize RGBA without allowing transparent background RGB to bleed into the silhouette."""
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
    """Fill only transparent RGB near the silhouette with nearby foreground colors for mip safety."""
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
    # Alpha is untouched; only hidden RGB is padded.
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


def postprocess_recipe(pass_index: int = 1, key_rgb=MAGENTA_RGB) -> dict:
    return {
        "id": POSTPROCESS_ID,
        "version": POSTPROCESS_VERSION,
        "pass": int(pass_index),
        "key_rgb": _as_key_rgb(key_rgb).tolist(),
        "key_stage": "source_resolution_before_resize",
        "resize": "premultiplied_lanczos4",
        "transparent_rgb": "foreground_edge_padding_4px",
        "bottom_anchor": "last_visible_alpha_row",
    }
