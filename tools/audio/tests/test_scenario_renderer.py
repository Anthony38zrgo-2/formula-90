"""Scenario renderer tests — deterministic WAV + CSV/JSON per scenario, sample-derived bank."""

from __future__ import annotations

import hashlib
import struct
import wave
from pathlib import Path

from tools.audio.render_audio_scenario import render_scenario
from tools.audio.scenarios import SCENARIO_FACTORIES, make_idle_to_redline_with_shifts

BANK = Path("game/sounds/banks/v10_vehicle")


def test_all_scenarios_render_and_are_deterministic(tmp_path):
    for name, factory in SCENARIO_FACTORIES.items():
        sc = factory()
        out_wav = tmp_path / f"{name}.wav"
        out_csv = tmp_path / f"{name}.csv"
        out_json = tmp_path / f"{name}.json"
        rep = render_scenario(sc, BANK, out_wav, out_csv, out_json)
        assert out_wav.is_file() and out_wav.stat().st_size > 0
        assert out_csv.is_file()
        assert out_json.is_file()
        out2 = tmp_path / f"{name}2.wav"
        rep2 = render_scenario(factory(), BANK, out2, tmp_path / f"{name}2.csv", tmp_path / f"{name}2.json")
        assert hashlib.sha256(out_wav.read_bytes()).hexdigest() == hashlib.sha256(out2.read_bytes()).hexdigest()
        assert rep["peak"] == rep2["peak"]
        assert rep["peak"] < 1.0
        assert rep["rms"] >= 0
        # The continuous sampled engine voices are retired (the procedural synth
        # is the engine), so the engine-only scenario renders silence while the
        # surface/impact scenarios still produce audible output.
        if name == "idle_to_redline_with_shifts":
            assert rep["peak"] == 0.0
            assert rep["rms"] == 0.0
        else:
            assert rep["peak"] > 0.05
            assert rep["rms"] > 0
        assert "\\" not in rep["wav"] and "\\" not in rep["csv"]


def test_rendered_wavs_are_mono_pcm16_44100_no_clipping(tmp_path):
    sc = make_idle_to_redline_with_shifts()
    out_wav = tmp_path / "probe.wav"
    render_scenario(sc, BANK, out_wav, tmp_path / "probe.csv", tmp_path / "probe.json")
    with wave.open(str(out_wav), "rb") as r:
        assert r.getnchannels() == 1
        assert r.getsampwidth() == 2
        assert r.getframerate() == 44100
        raw = r.readframes(r.getnframes())
    vals = [v[0] / 32767.0 for v in struct.iter_unpack("<h", raw[: len(raw) // 2 * 2])]
    assert max(abs(v) for v in vals) < 1.0
    assert all(abs(v) < 1.0 for v in vals)


def test_scenario_families_use_available_surfaces():
    assert set(SCENARIO_FACTORIES) == {"idle_to_redline_with_shifts", "road_to_sand_with_kerb", "grass_skid", "impact"}
    surfaces = set()
    for factory in SCENARIO_FACTORIES.values():
        for f in factory().frames:
            surfaces.add(f.surface)
    assert surfaces <= {"asphalt", "sand", "grass", "rumble"}
