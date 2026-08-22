"""Stage de efectos -- envuelve un Pedalboard (reverb, comp, pitch, distorsión).

Pedalboard es autocontenido (no necesita ffmpeg/sox) y corre offline/headless.
"""

from __future__ import annotations

import numpy as np

from tools.audio.synthesis.protocols import Stage


class EffectsChain(Stage):
    """Cadena de efectos sobre el buffer.

    Pasa ``board`` ya construido, o bien usa los kwargs para armar uno estándar:
    ``reverb`` (0..1, mezcla húmeda), ``compressor`` (bool), ``pitch_semitones``,
    ``distortion`` (dB de drive), ``delay`` (segundos).
    """

    def __init__(
        self,
        board: object | None = None,
        *,
        reverb: float = 0.0,
        compressor: bool = False,
        pitch_semitones: float = 0.0,
        distortion: float = 0.0,
        delay: float = 0.0,
    ) -> None:
        if board is not None:
            self.board = board
            return
        from pedalboard import (  # import perezoso: el paquete sigue importable sin pedalboard
            Compressor,
            Delay,
            Distortion,
            Pedalboard,
            PitchShift,
            Reverb,
        )

        fx = []
        if compressor:
            fx.append(Compressor(threshold_db=-18.0, ratio=4.0))
        if reverb > 0.0:
            fx.append(Reverb(room_size=float(reverb), wet_level=float(reverb)))
        if pitch_semitones != 0.0:
            fx.append(PitchShift(semitones=float(pitch_semitones)))
        if distortion > 0.0:
            fx.append(Distortion(drive_db=float(distortion)))
        if delay > 0.0:
            fx.append(Delay(delay_seconds=float(delay)))
        self.board = Pedalboard(fx) if fx else None

    def process(self, x: np.ndarray, sr: int) -> tuple[np.ndarray, int]:
        if self.board is None or len(self.board) == 0:
            return x, sr
        mono = np.asarray(x, dtype=np.float32)
        out = self.board(mono, sr)
        return np.asarray(out, dtype=np.float64), sr
