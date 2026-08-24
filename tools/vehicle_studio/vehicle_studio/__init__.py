"""Formula-90 Vehicle Studio semantic domain."""

from .domain import VehicleDocument, VehicleDocumentError
from .diagnostics import Diagnostic, Severity

__all__ = [
    "Diagnostic",
    "Severity",
    "VehicleDocument",
    "VehicleDocumentError",
]

