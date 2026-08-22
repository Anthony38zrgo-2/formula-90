"""Stage de análisis -- graba espectrograma o waveform a PNG (validación headless)."""

from __future__ import annotations

from pathlib import Path

import numpy as np

from tools.audio.synthesis.protocols import Stage


class Analyzer(Stage):
    def __init__(self, out_path: str | Path | None = None, kind: str = "spectrogram") -> None:
        self.out_path = Path(out_path) if out_path is not None else None
        self.kind = kind

    def process(self, x: np.ndarray, sr: int) -> tuple[np.ndarray, int]:
        if self.out_path is None:
            return x, sr
        import matplotlib

        matplotlib.use("Agg")
        import librosa.display
        import matplotlib.pyplot as plt

        fig, ax = plt.subplots(figsize=(8, 3))
        if self.kind == "waveform":
            ax.plot(np.arange(len(x)) / sr, x)
            ax.set_title("waveform")
        else:
            S = librosa.amplitude_to_db(np.abs(librosa.stft(x)), ref=np.max)
            librosa.display.specshow(S, sr=sr, y_axis="log", x_axis="time", ax=ax)
            ax.set_title("spectrogram")
        self.out_path.parent.mkdir(parents=True, exist_ok=True)
        fig.savefig(str(self.out_path), dpi=120)
        plt.close(fig)
        return x, sr
