"""Stable structured diagnostics for Vehicle Studio contracts."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import IntEnum
from typing import Any


class Severity(IntEnum):
    INFO = 0
    WARNING = 1
    ERROR = 2
    FATAL = 3


@dataclass(frozen=True)
class Diagnostic:
    code: str
    severity: Severity
    message: str
    path: str | None = None
    object_id: str | None = None
    metadata: dict[str, Any] = field(default_factory=dict)

    def sort_key(self) -> tuple[int, str, str, str, str]:
        return (
            -int(self.severity),
            self.code,
            self.path or "",
            self.object_id or "",
            self.message,
        )

    def as_dict(self) -> dict[str, Any]:
        result: dict[str, Any] = {
            "code": self.code,
            "severity": self.severity.name,
            "message": self.message,
            "metadata": dict(sorted(self.metadata.items())),
        }
        if self.path is not None:
            result["path"] = self.path
        if self.object_id is not None:
            result["object_id"] = self.object_id
        return result


def sorted_diagnostics(items: list[Diagnostic]) -> list[Diagnostic]:
    return sorted(items, key=Diagnostic.sort_key)

