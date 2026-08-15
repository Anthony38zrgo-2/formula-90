"""Repository-local pytest bootstrap.

pyproject.toml sets `--basetemp=.tmp/pytest`; pytest requires the basetemp
parent to exist before tmp_path fixtures run. Creating it here (conftest loads
during collection, before fixture setup) makes `pytest tools/audio/tests`
self-contained on a fresh checkout.
"""

from pathlib import Path

Path(".tmp/pytest").mkdir(parents=True, exist_ok=True)
