"""Deterministic report serialization and envelope contract tests."""
import json
import sys
import tempfile
import unittest
from pathlib import Path

SUITE_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SUITE_DIR))

from analysis_suite import reporting  # noqa: E402
from analysis_suite import __version__  # noqa: E402


class SerializationTests(unittest.TestCase):
    def test_serialization_is_deterministic(self):
        report = {
            'zebra': 1,
            'alpha': {'nested': [3, 1, 2], 'text': 'á'},
            'beta': None,
        }
        first = reporting.serialize(report)
        second = reporting.serialize(dict(reversed(list(report.items()))))
        self.assertEqual(first, second)
        self.assertIsInstance(first, str)

    def test_serialization_uses_sorted_keys(self):
        report = {'b': 1, 'a': 2}
        text = reporting.serialize(report)
        self.assertLess(text.index('"a"'), text.index('"b"'))

    def test_serialization_rejects_non_finite_values(self):
        with self.assertRaises(ValueError):
            reporting.serialize({'bad': float('nan')})

    def test_finding_order_is_stable(self):
        findings = [
            reporting.make_finding('error', 'C', 'c'),
            reporting.make_finding('info', 'B', 'b'),
            reporting.make_finding('warning', 'A', 'a'),
            reporting.make_finding('error', 'A', 'a'),
        ]
        envelope = reporting.build_envelope(
            'test', [], findings, None, 'pass', 0, 'done'
        )
        keys = [(f['code'], f['level'], f['message']) for f in envelope['findings']]
        self.assertEqual(keys, sorted(keys))
        self.assertEqual(
            envelope['findings'][0]['code'], 'A',
            'identical codes must fall back to level ordering',
        )

    def test_envelope_has_no_timestamps_or_random_ids(self):
        envelope = reporting.build_envelope(
            'test', [], [], None, 'pass', 0, 'done'
        )
        text = reporting.serialize(envelope)
        for forbidden in ('timestamp', 'datetime', 'uuid', 'random'):
            self.assertNotIn(forbidden, text)
        self.assertEqual(envelope['suite']['version'], __version__)

    def test_write_report_creates_identical_content(self):
        envelope = reporting.build_envelope(
            'test', [], [], None, 'pass', 0, 'done'
        )
        with tempfile.TemporaryDirectory() as temp_dir:
            target = Path(temp_dir) / 'nested' / 'report.json'
            reporting.write_report(envelope, target)
            written = target.read_text(encoding='utf-8').strip()
        self.assertEqual(written, reporting.serialize(envelope))
        self.assertEqual(json.loads(written), envelope)


if __name__ == '__main__':
    unittest.main()
