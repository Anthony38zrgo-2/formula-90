"""Stages de E/S -- leer un sample existente como Source, escribir un buffer como sink."""

from __future__ import annotations

from pathlib import Path

import numpy as np
import soundfile as sf

from tools.audio.synthesis.protocols import Source, Stage


class WaveReader(Source):
    """Fuente que carga un WAV existente (mono, float64). Útil para *modificar* samples."""

    def __init__(self, path: str | Path) -> None:
        self.path = Path(path)
        self.sample_rate = 44100  # se actualiza en render() con el SR real del archivo
        self._data: np.ndarray | None = None

    def render(self) -> np.ndarray:
        data, sr = sf.read(str(self.path), always_2d=False)
        if data.ndim > 1:
            data = data.mean(axis=1)
        self.sample_rate = int(sr)
        self._data = np.asarray(data, dtype=np.float64)
        return self._data


class WaveWriter(Stage):
    """Sink: escribe el buffer a WAV y lo deja pasar (para encadenar)."""

    def __init__(self, path: str | Path, subtype: str = "PCM_16") -> None:
        self.path = Path(path)
        self.subtype = subtype

    def process(self, x: np.ndarray, sr: int) -> tuple[np.ndarray, int]:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        sf.write(str(self.path), np.asarray(x, dtype=np.float32), sr, subtype=self.subtype)
        return x, sr
