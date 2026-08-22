"""Contratos del pipeline de síntesis modular.

Un :class:`Source` produce el buffer de audio inicial. Un :class:`Stage` lo
transforma en otro buffer (y puede cambiar el sample rate). El orquestador
(:mod:`tools.audio.synthesis.orchestrator`) encadena un source con una lista
ordenada de stages, de modo que añadir comportamiento nuevo es escribir un
stage nuevo, no editar el orquestador.
"""

from __future__ import annotations

from typing import Protocol, runtime_checkable

import numpy as np


@runtime_checkable
class Source(Protocol):
    """Genera el buffer de audio inicial a ``sample_rate`` Hz."""

    sample_rate: int

    def render(self) -> np.ndarray:
        ...


@runtime_checkable
class Stage(Protocol):
    """Transforma ``(signal, sr)`` en ``(signal, sr)``.

    Los stages son puros: misma entrada => misma salida. Un stage que
    re-muestrea debe devolver el nuevo sample rate como segundo elemento.
    """

    def process(self, x: np.ndarray, sr: int) -> tuple[np.ndarray, int]:
        ...
