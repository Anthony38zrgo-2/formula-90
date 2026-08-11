"""Shared contracts: exit codes, config helpers and error types.

Exit codes:
    0  PASS   - command completed; all measured validations passed
    1  FAIL   - deterministic validation failure (a measured value violated a
                configured expectation)
    2  CONFIG - invalid configuration, precondition or input
    3  CORRUPT- corrupt or unsupported asset
"""
import math

EXIT_PASS = 0
EXIT_FAIL = 1
EXIT_CONFIG = 2
EXIT_CORRUPT = 3

SUITE_NAME = 'formula90s-analysis-suite'

STATUS_PASS = 'pass'
STATUS_FAIL = 'fail'
STATUS_ERROR = 'error'
STATUS_NOT_MEASURABLE = 'not_measurable'


class ConfigError(ValueError):
    """Invalid configuration, precondition or input (exit code 2)."""


class CorruptAssetError(ValueError):
    """Corrupt or unsupported asset (exit code 3)."""


def require_str(value, label):
    if not isinstance(value, str) or not value.strip():
        raise ConfigError('%s must be a non-empty string' % label)
    return value.strip()


def require_number(value, label):
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ConfigError('%s must be a number' % label)
    return value


def require_finite_number(value, label):
    value = require_number(value, label)
    if not math.isfinite(value):
        raise ConfigError('%s must be finite' % label)
    return value


def require_non_negative_number(value, label):
    value = require_finite_number(value, label)
    if value < 0:
        raise ConfigError('%s must be non-negative' % label)
    return value


def require_dict(value, label):
    if not isinstance(value, dict):
        raise ConfigError('%s must be an object' % label)
    return value


def require_list(value, label):
    if not isinstance(value, list):
        raise ConfigError('%s must be an array' % label)
    return value


def parse_axis(value, label):
    """Parse an axis token: 'X', '-Y', 'Z' ... -> (axis_index, sign).

    A leading '-' inverts the comparison direction so a forward axis of '-Z'
    behaves naturally for vehicle-style coordinate conventions.
    """
    text = require_str(value, label).lower()
    sign = 1
    if text.startswith('-'):
        sign = -1
        text = text[1:]
    if text not in ('x', 'y', 'z'):
        raise ConfigError('%s must be one of X, Y, Z (optionally signed)' % label)
    return 'xyz'.index(text), sign


def convention_axis(convention, role, label):
    """Resolve 'forward' / 'right' / 'up' from a coordinate convention."""
    convention = require_dict(convention, 'coordinate_convention')
    return parse_axis(convention.get(role), '%s.%s' % (label, role))
