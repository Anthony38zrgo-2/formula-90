from __future__ import annotations

import json
import os
import tempfile
import time
from pathlib import Path


_RETRY_ERRNOS = {13, 22}
_RETRY_WINERRORS = {5, 32, 87}


def write_json_atomic(
    path: str | Path,
    payload: object,
    *,
    indent: int = 2,
    retries: int = 5,
    retry_delay_s: float = 0.08,
) -> None:
    """Write JSON via same-directory temp file + os.replace, with short Windows retries."""
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    text = json.dumps(payload, indent=indent) + "\n"

    last_error: OSError | None = None
    for attempt in range(max(1, int(retries))):
        temp_path: Path | None = None
        try:
            fd, raw_temp = tempfile.mkstemp(
                prefix=f".{target.name}.",
                suffix=".tmp",
                dir=str(target.parent),
                text=True,
            )
            temp_path = Path(raw_temp)
            with os.fdopen(fd, "w", encoding="utf-8", newline="\n") as handle:
                handle.write(text)
                handle.flush()
                os.fsync(handle.fileno())
            os.replace(temp_path, target)
            return
        except OSError as exc:
            last_error = exc
            if temp_path is not None:
                try:
                    temp_path.unlink(missing_ok=True)
                except OSError:
                    pass
            retryable = exc.errno in _RETRY_ERRNOS or getattr(exc, "winerror", None) in _RETRY_WINERRORS
            if retryable and attempt + 1 < max(1, int(retries)):
                time.sleep(max(0.0, float(retry_delay_s)) * (attempt + 1))
                continue
            break

    assert last_error is not None
    raise RuntimeError(
        "Atomic JSON write failed: "
        f"path={str(target)!r} exists={target.exists()} parent={str(target.parent)!r} "
        f"errno={last_error.errno} winerror={getattr(last_error, 'winerror', None)}"
    ) from last_error
