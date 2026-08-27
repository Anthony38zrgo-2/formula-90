"""Fail-closed defaults for audio preview and legacy promotion CLIs."""

from __future__ import annotations

from pathlib import Path

import pytest

from tools.audio import bank_generator, render_audio_scenario
from tools.audio.promote_f1_94_replacement_sounds import main as legacy_promote_main


def test_audio_write_defaults_live_under_scratch():
    assert bank_generator.DEFAULT_OUTPUT == Path("scratch/audio/v10_vehicle")
    assert render_audio_scenario.DEFAULT_OUTPUT == Path("scratch/audio/scenarios")
    remaster_source = Path("tools/audio/remaster_lib.py").read_text(encoding="utf-8")
    assert 'DEFAULT_BANK = Path("scratch/audio/v10_vehicle")' in remaster_source
    assert "D:/Formula90s" not in remaster_source


@pytest.mark.parametrize(
    ("main", "args"),
    [
        (
            bank_generator.main,
            ["--output", "game/sounds/banks/unsafe-preview"],
        ),
        (
            render_audio_scenario.main,
            ["--out", "game/sounds/scenarios/unsafe-preview"],
        ),
    ],
)
def test_preview_clis_reject_runtime_outputs(main, args):
    with pytest.raises(SystemExit) as error:
        main(args)
    assert error.value.code == 2


def test_legacy_promoter_requires_explicit_source_and_bank():
    with pytest.raises(SystemExit) as error:
        legacy_promote_main([])
    assert error.value.code == 2
