"""Authoring source for sample-specific playback metadata."""

from __future__ import annotations

from dataclasses import dataclass

from tools.audio.bank_manifest import BankManifest


@dataclass(frozen=True)
class EngineBandPlayback:
    key: str
    native_rpm: float
    index: int
    center: float
    width: float = 0.25


ENGINE_BANDS: tuple[EngineBandPlayback, ...] = (
    EngineBandPlayback("engine_idle", 3941.0, 0, 0.0),
    EngineBandPlayback("engine_low", 7429.0, 1, 0.25),
    EngineBandPlayback("engine_mid", 9800.0, 2, 0.5),
    EngineBandPlayback("engine_high", 16950.0, 3, 0.75),
    EngineBandPlayback("engine_redline", 7687.0, 4, 1.0),
)

_ENGINE_BAND_BY_KEY = {band.key: band for band in ENGINE_BANDS}
_PITCHED_NATIVE_RPM = {"exhaust-mic": 14400.0}


def playback_for_key(key: str) -> dict[str, object]:
    """Return manifest-ready metadata; empty means native-rate playback."""

    band = _ENGINE_BAND_BY_KEY.get(key)
    if band is not None:
        return {
            "native_rpm": band.native_rpm,
            "engine_band": {
                "index": band.index,
                "center": band.center,
                "width": band.width,
            },
        }
    native_rpm = _PITCHED_NATIVE_RPM.get(key)
    return {"native_rpm": native_rpm} if native_rpm is not None else {}


def attach_playback_metadata(manifest: BankManifest) -> BankManifest:
    """Attach canonical playback metadata without changing audio content."""

    for entry in manifest.files:
        key = entry.file.removesuffix(".wav")
        entry.playback = playback_for_key(key)
        entry.synthesis.pop("native_rpm", None)
    return manifest
