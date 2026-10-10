import hashlib
import shutil
from pathlib import Path

from tools.audio.bank_manifest import BankManifest
from tools.audio.prepare_neutral_first_gear_sample import RUNTIME_FILENAME, prepare


def test_neutral_first_gear_preparation_is_reproducible(tmp_path):
    source = Path("game/sounds/bank_sources/neutral_first_gear_transition_source.wav")
    temporary_source = tmp_path / source
    temporary_source.parent.mkdir(parents=True)
    shutil.copyfile(source, temporary_source)
    bank = tmp_path / "game/sounds/banks/commons"
    bank.mkdir(parents=True)
    BankManifest().write(bank / "bank_manifest.json")
    first = prepare(temporary_source, bank, tmp_path)
    second = prepare(temporary_source, bank, tmp_path)
    assert first == second
    assert first.sha256 == "b268f1fa879d6305d56f4db01900d8f05b4930e280b9ce467fac75a4b1c6970b"
    assert hashlib.sha256((Path("game/sounds/banks/commons") / RUNTIME_FILENAME).read_bytes()).hexdigest() == first.sha256
    assert not first.loop
    assert first.role == "neutral_first_gear_transition"
