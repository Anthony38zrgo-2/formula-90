"""Bank manifest — single responsibility: manifest model & deterministic serialization."""

from __future__ import annotations

import hashlib
import json
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any


@dataclass
class FileEntry:
    file: str
    role: str
    loop: bool
    duration_s: float
    loop_start_s: float | None = None
    loop_end_s: float | None = None
    loudness_dbfs: float = 0.0
    peak: float = 0.0
    dc_offset: float = 0.0
    synthesis: dict[str, Any] = field(default_factory=dict)
    provenance: str = "derived from original samples (assets-lowpoly-python/sounds)"
    sha256: str = ""


@dataclass
class BankManifest:
    schema_version: int = 1
    bank_name: str = "v10_vehicle"
    sample_rate: int = 44100
    channels: int = 1
    pcm_bits: int = 16
    seed: int = 0
    generator: str = "formula90s sample-based bank builder 1.0"
    files: list[FileEntry] = field(default_factory=list)

    def to_dict(self) -> dict[str, Any]:
        d = asdict(self)
        d["files"] = sorted(d["files"], key=lambda e: e["file"])
        return d

    def to_json_bytes(self) -> bytes:
        return (json.dumps(self.to_dict(), indent=2, sort_keys=True, ensure_ascii=False) + "\n").encode("utf-8")

    def write(self, path: Path) -> None:
        import time

        p = Path(path)
        p.parent.mkdir(parents=True, exist_ok=True)
        data = self.to_json_bytes()
        for attempt in range(5):
            try:
                with open(str(p), "wb") as f:
                    f.write(data)
                return
            except OSError:
                if attempt == 4:
                    raise
                time.sleep(0.05)

    @staticmethod
    def load(path: Path) -> BankManifest:
        raw = json.loads(path.read_text(encoding="utf-8"))
        files = [FileEntry(**f) for f in raw.get("files", [])]
        return BankManifest(
            schema_version=raw.get("schema_version", 1),
            bank_name=raw.get("bank_name", ""),
            sample_rate=raw.get("sample_rate", 44100),
            channels=raw.get("channels", 1),
            pcm_bits=raw.get("pcm_bits", 16),
            seed=raw.get("seed", 0),
            generator=raw.get("generator", ""),
            files=files,
        )

    def file_sha256_map(self) -> dict[str, str]:
        return {e.file: e.sha256 for e in self.files}


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()
