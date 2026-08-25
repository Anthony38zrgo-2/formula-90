import unittest
from pathlib import Path
import hashlib
import json
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "blender" / "track_pipeline"))

from asset_registry import (
    ALLOWED_KINDS,
    AssetRegistry,
    AssetSpec,
    parse_registry,
    validate_registry,
    load_registry,
)


def _sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


class AssetRegistryTests(unittest.TestCase):
    def setUp(self):
        self.repo = ROOT
        self.tmp = ROOT / "blender" / "track_pipeline" / "tests" / "_tmp_asset_registry"
        self.tmp.mkdir(parents=True, exist_ok=True)
        self.source = self.tmp / "asset.glb"
        self.source.write_bytes(b"asset-bytes")
        self.preview = self.tmp / "preview.png"
        self.preview.write_bytes(b"png-bytes")

    def tearDown(self):
        for child in self.tmp.glob("*"):
            child.unlink()
        self.tmp.rmdir()

    def _spec(self, **overrides):
        base = dict(
            id="tree_v2_01",
            kind="vegetation",
            category="trees",
            source="blender/track_pipeline/tests/_tmp_asset_registry/asset.glb",
            dimensions_m={"width": 10.8, "height": 17.2, "depth": 10.8},
            source_sha256=_sha(self.source.read_bytes()),
            collision_class="none",
            budget={"min_instances": 0, "max_instances": 130},
        )
        base.update(overrides)
        return AssetSpec(**base)

    def test_valid_spec_has_no_errors(self):
        registry = AssetRegistry([self._spec()], repo_root=self.repo)
        self.assertEqual(validate_registry(registry), [])

    def test_duplicate_id_rejected(self):
        registry = AssetRegistry([self._spec(), self._spec()], repo_root=self.repo)
        errors = validate_registry(registry)
        self.assertTrue(any("duplicate" in e for e in errors))

    def test_missing_source_rejected(self):
        registry = AssetRegistry(
            [self._spec(source="blender/track_pipeline/tests/_tmp_asset_registry/missing.glb")],
            repo_root=self.repo,
        )
        errors = validate_registry(registry)
        self.assertTrue(any("missing source" in e for e in errors))

    def test_source_sha_mismatch_rejected(self):
        registry = AssetRegistry(
            [self._spec(source_sha256="0" * 64)],
            repo_root=self.repo,
        )
        errors = validate_registry(registry)
        self.assertTrue(any("sha256 mismatch" in e for e in errors))

    def test_unknown_kind_rejected(self):
        registry = AssetRegistry([self._spec(kind="rocket")], repo_root=self.repo)
        errors = validate_registry(registry)
        self.assertTrue(any("unknown kind" in e for e in errors))

    def test_unknown_collision_class_rejected(self):
        registry = AssetRegistry([self._spec(collision_class="hot")], repo_root=self.repo)
        errors = validate_registry(registry)
        self.assertTrue(any("collision_class" in e for e in errors))

    def test_nonpositive_dimension_rejected(self):
        registry = AssetRegistry(
            [self._spec(dimensions_m={"width": 0.0, "height": 1.0, "depth": 1.0})],
            repo_root=self.repo,
        )
        errors = validate_registry(registry)
        self.assertTrue(any("must be positive" in e for e in errors))

    def test_budget_min_exceeds_max_rejected(self):
        registry = AssetRegistry(
            [self._spec(budget={"min_instances": 5, "max_instances": 3})],
            repo_root=self.repo,
        )
        errors = validate_registry(registry)
        self.assertTrue(any("exceeds max" in e for e in errors))

    def test_procedural_kind_allows_missing_source(self):
        spec = AssetSpec(
            id="track_flag",
            kind="flag",
            category="flag",
            source=None,
            dimensions_m={"width": 1.0, "height": 1.8, "depth": 0.1},
        )
        registry = AssetRegistry([spec], repo_root=self.repo)
        self.assertEqual(validate_registry(registry), [])

    def test_nonprocedural_missing_source_rejected(self):
        spec = AssetSpec(
            id="card_x",
            kind="card",
            category="spectator",
            source=None,
            dimensions_m={"width": 1.0, "height": 1.0, "depth": 0.1},
        )
        registry = AssetRegistry([spec], repo_root=self.repo)
        errors = validate_registry(registry)
        self.assertTrue(any("missing source" in e for e in errors))

    def test_committed_registry_is_valid(self):
        registry = load_registry(
            ROOT / "blender" / "track_pipeline" / "configs" / "asset_registry.json",
            repo_root=ROOT,
        )
        self.assertEqual(validate_registry(registry), [])
        self.assertEqual(len(registry), 21)
        self.assertEqual(len(registry.by_kind("vegetation")), 12)
        self.assertNotIn("tree_v2_01", registry)
        self.assertIn("tree_3d_06", registry)
        self.assertIn("bush_3d_06", registry)


if __name__ == "__main__":
    unittest.main()
