"""Format & quality tests — every bank WAV must be mono PCM16 44.1 kHz and pass quality gates."""

from __future__ import annotations

import json
import math
import struct
import wave
from pathlib import Path

BANK = Path("game/sounds/banks/v10_vehicle")


def _read_mono16(path: Path):
    with wave.open(str(path), "rb") as r:
        params = r.getparams()
        raw = r.readframes(r.getnframes())
    vals = [v[0] / 32767.0 for v in struct.iter_unpack("<h", raw[: len(raw) // 2 * 2])] if raw else []
    return params, vals


def test_every_wav_is_mono_pcm16_44100():
    for wav in sorted(BANK.glob("*.wav")):
        params, _ = _read_mono16(wav)
        assert params.nchannels == 1, f"{wav.name} channels"
        assert params.sampwidth == 2, f"{wav.name} sampwidth"
        assert params.framerate == 44100, f"{wav.name} rate {params.framerate}"


def test_no_clipping_and_dc_within_gate():
    for wav in sorted(BANK.glob("*.wav")):
        _, vals = _read_mono16(wav)
        assert vals, f"{wav.name} empty"
        peak = max(abs(v) for v in vals)
        assert peak < 0.99, f"{wav.name} clipping peak {peak:.4f}"
        assert all(math.isfinite(v) for v in vals), f"{wav.name} non-finite"
        dc = sum(vals) / len(vals)
        assert abs(dc) < 0.005, f"{wav.name} DC {dc:.5f}"


def test_loop_seam_continuity():
    manifest = json.loads((BANK / "bank_manifest.json").read_text(encoding="utf-8"))
    by_file = {e["file"]: e for e in manifest["files"]}
    for wav in sorted(BANK.glob("*.wav")):
        entry = by_file.get(wav.name)
        if entry is None:
            # WAV on disk not listed in the manifest (e.g. *_backup originals
            # kept alongside remasters). Loop-seam only applies to listed entries.
            continue
        _, vals = _read_mono16(wav)
        if entry["loop"]:
            thresh = 0.05
            disc = abs(vals[-1] - vals[0])
            assert disc <= thresh, f"{wav.name} loop seam step {disc:.3f} > {thresh}"


def test_bank_validator_passes():
    from tools.audio.bank_validator import validate_bank

    findings = validate_bank(BANK)
    errors = [f for f in findings if f.level == "error"]
    assert not errors, f"validator errors: {errors}"
