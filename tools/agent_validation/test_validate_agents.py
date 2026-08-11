from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from tools.agent_validation.validate_agents import MANDATORY_HANDOFF_HEADERS, validate


class AgentValidationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        (self.root / ".agents/skills/sample").mkdir(parents=True)
        (self.root / ".agents/agents").mkdir(parents=True)
        (self.root / ".agents/registry").mkdir(parents=True)
        (self.root / "docs/ai").mkdir(parents=True)
        (self.root / "AGENTS.md").write_text("# Root\n", encoding="utf-8")
        (self.root / ".agents/AGENTS.md").write_text("# Protocol\n", encoding="utf-8")
        (self.root / ".agents/skills/sample/SKILL.md").write_text(
            "---\nname: sample\ndescription: Test skill.\n---\n\n# Sample\n",
            encoding="utf-8",
        )
        registry = {
            "schema_version": 1,
            "skills": [
                {
                    "id": "sample",
                    "manifest_path": ".agents/skills/sample/SKILL.md",
                }
            ],
        }
        (self.root / ".agents/registry/skills.json").write_text(
            json.dumps(registry), encoding="utf-8"
        )
        (self.root / ".agents/agents/test.json").write_text(
            json.dumps({"id": "test", "skills": ["sample"]}), encoding="utf-8"
        )
        headers = "\n\n".join(f"# {header}\n[value]" for header in MANDATORY_HANDOFF_HEADERS)
        (self.root / "docs/ai/handoff_template.md").write_text(headers, encoding="utf-8")

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_valid_fixture_passes(self) -> None:
        self.assertEqual(validate(self.root), [])

    def test_missing_registered_skill_fails(self) -> None:
        (self.root / ".agents/skills/sample/SKILL.md").unlink()
        self.assertTrue(any("manifest missing" in error for error in validate(self.root)))

    def test_broken_relative_link_fails(self) -> None:
        (self.root / "AGENTS.md").write_text("[missing](docs/missing.md)\n", encoding="utf-8")
        self.assertTrue(any("broken relative link" in error for error in validate(self.root)))

    def test_nonexistent_inline_tool_path_fails(self) -> None:
        (self.root / "AGENTS.md").write_text("Use `tools/missing.py`.\n", encoding="utf-8")
        self.assertTrue(any("nonexistent repository path" in error for error in validate(self.root)))

    def test_obsolete_attempt_rule_fails(self) -> None:
        path = self.root / ".agents/skills/sample/SKILL.md"
        path.write_text(read(path) + "\nIf it fails repeatedly, try again.\n", encoding="utf-8")
        self.assertTrue(any("obsolete/conflicting" in error for error in validate(self.root)))

    def test_missing_handoff_header_fails(self) -> None:
        path = self.root / "docs/ai/handoff_template.md"
        path.write_text(read(path).replace("# FAILURE SIGNATURE", "# REMOVED"), encoding="utf-8")
        self.assertTrue(any("FAILURE SIGNATURE" in error for error in validate(self.root)))

    def test_circular_skill_routing_fails(self) -> None:
        path = self.root / ".agents/skills/sample/SKILL.md"
        path.write_text(read(path) + "\nSee ../sample/SKILL.md.\n", encoding="utf-8")
        self.assertTrue(any("circular skill routing" in error for error in validate(self.root)))


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


if __name__ == "__main__":
    unittest.main()
