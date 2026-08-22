"""Stage de filtrado -- anti-alias / high-pass / band-pass / notch (IIR scipy)."""

from __future__ import annotations

import numpy as np
from scipy.signal import butter, iirnotch, sosfilt, tf2sos

from tools.audio.synthesis.protocols import Stage


class AntiAliasFilter(Stage):
    """Low-pass que elimina contenido sobre ``cutoff_hz`` (protege de aliasing)."""

    def __init__(self, cutoff_hz: float, order: int = 4) -> None:
        self.cutoff_hz = cutoff_hz
        self.order = order

    def process(self, x: np.ndarray, sr: int) -> tuple[np.ndarray, int]:
        nyq = 0.5 * sr
        wc = min(self.cutoff_hz, nyq - 1.0) / nyq
        sos = butter(self.order, wc, btype="low", output="sos")
        return sosfilt(sos, x), sr


class HighPassFilter(Stage):
    def __init__(self, cutoff_hz: float, order: int = 2) -> None:
        self.cutoff_hz = cutoff_hz
        self.order = order

    def process(self, x: np.ndarray, sr: int) -> tuple[np.ndarray, int]:
        nyq = 0.5 * sr
        wc = max(self.cutoff_hz, 1.0) / nyq
        sos = butter(self.order, wc, btype="high", output="sos")
        return sosfilt(sos, x), sr


class BandPassFilter(Stage):
    def __init__(self, low_hz: float, high_hz: float, order: int = 4) -> None:
        self.low_hz = low_hz
        self.high_hz = high_hz
        self.order = order

    def process(self, x: np.ndarray, sr: int) -> tuple[np.ndarray, int]:
        sos = butter(self.order, [self.low_hz, self.high_hz], btype="band", fs=sr, output="sos")
        return sosfilt(sos, x), sr


class NotchFilter(Stage):
    def __init__(self, freq_hz: float, quality: float = 2.5) -> None:
        self.freq_hz = freq_hz
        self.quality = quality

    def process(self, x: np.ndarray, sr: int) -> tuple[np.ndarray, int]:
        b, a = iirnotch(self.freq_hz, self.quality, fs=sr)
        sos = tf2sos(b, a)
        return sosfilt(sos, x), sr
