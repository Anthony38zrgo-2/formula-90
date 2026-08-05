from __future__ import annotations

import hashlib
import json
import math
import struct
import wave
from pathlib import Path

from tools.audio.generate_engine_samples import generate

ROOT = Path(__file__).resolve().parents[3]
CONFIG = ROOT / "config" / "audio_synthesis_v10.yaml"


def read_samples(path: Path) -> tuple[wave._wave_params, list[float]]:
    with wave.open(str(path), "rb") as source:
        params = source.getparams()
        payload = source.readframes(source.getnframes())
    values = [value[0] / 32767.0 for value in struct.iter_unpack("<h", payload)]
    return params, values


def test_generates_reproducible_valid_bank(tmp_path: Path) -> None:
    first = tmp_path / "first"
    second = tmp_path / "second"
    generate(CONFIG, first)
    generate(CONFIG, second)
    layers = ["idle", "low", "mid", "high", "redline"]
    hashes = []
    for name in layers:
        path = first / f"engine_{name}.wav"
        params, samples = read_samples(path)
        assert params.nchannels == 1
        assert params.framerate == 44100
        assert params.sampwidth == 2
        assert math.isclose(params.nframes / params.framerate, 2.0, abs_tol=1 / 44100)
        assert all(math.isfinite(value) for value in samples)
        assert max(abs(value) for value in samples) < 0.99
        assert abs(sum(samples) / len(samples)) < 0.005
        first_hash = hashlib.sha256(path.read_bytes()).hexdigest()
        assert first_hash == hashlib.sha256((second / path.name).read_bytes()).hexdigest()
        hashes.append(first_hash)
    assert len(set(hashes)) == len(layers)
    for name in ("gear_up.wav", "gear_down.wav"):
        params, samples = read_samples(first / name)
        assert params.nchannels == 1 and params.framerate == 44100
        assert 0.34 < params.nframes / params.framerate < 0.36
        assert max(abs(value) for value in samples) < 0.99
    metadata = json.loads((first / "generation_metadata.json").read_text(encoding="utf-8"))
    assert metadata["bank_name"] == "v10_prototype"
    assert len(metadata["files"]) == 7

