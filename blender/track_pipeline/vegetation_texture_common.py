from __future__ import annotations

import cv2
import numpy as np


MAGENTA_RGB = np.array([255, 0, 255], dtype=np.uint8)


def magenta_key_mask(rgb: np.ndarray, pass_index: int) -> np.ndarray:
    """Detect only near-magenta key/spill colors, preserving autumn pinks."""
    rgb_i = rgb.astype(np.int32)
    distance = np.sqrt(((rgb_i - MAGENTA_RGB.astype(np.int16)) ** 2).sum(axis=-1))
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
