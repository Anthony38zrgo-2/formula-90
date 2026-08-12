import unittest
from pathlib import Path
import json
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "blender" / "track_pipeline"))

from asset_registry import load_registry
from compile_svg_track import (
    CANONICAL_ARTIFACT,
    MANIFEST_ARTIFACT,
    NORMALIZED_ARTIFACT,
    CompileError,
    compile_track,
)
from import_la_chutana import (
    COMPILED_SUBDIR,
    SOURCE_FILENAME,
    render_source_svg,
    verify_parity,
)

FIXTURES = ROOT / "blender" / "track_pipeline" / "tests" / "fixtures"


def _load(name: str) -> dict:
    return json.loads((FIXTURES / name).read_text(encoding="utf-8"))


class ImportLaChutanaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.compiled = _load("la_chutana_compiled_mini.json")
        cls.centerline = _load("la_chutana_centerline_mini.json")
        cls.catalog = _load("la_chutana_catalog_mini.json")
        cls.config = _load("la_chutana_config_mini.json")
        cls.registry_path = FIXTURES / "la_chutana_registry_mini.json"
        cls.mini_object_target = {"spectator": 1, "marshal": 1, "flag": 1}
        cls.mini_vegetation_target = {"grass": 2, "bushes": 1, "trees": 1}

    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="f90_import_"))
        self.addCleanup(_rmtree, self.tmp)
        self.registry = load_registry(self.registry_path, repo_root=ROOT)

    def _run(self, out: Path) -> tuple[str, dict]:
        source = render_source_svg(self.compiled, self.centerline, self.catalog, self.config)
        manifest = compile_track(
            source.encode("utf-8"),
            source_name=SOURCE_FILENAME,
            registry=self.registry,
            registry_path=self.registry_path,
            output_dir=out,
        )
        return source, manifest

    def _normalized(self, out: Path) -> dict:
        return json.loads((out / NORMALIZED_ARTIFACT).read_text(encoding="utf-8"))

    # -- source SVG structure ---------------------------------------------

    def test_renders_all_profile_roles(self):
        source = render_source_svg(self.compiled, self.centerline, self.catalog, self.config)
        self.assertTrue(source.lstrip().startswith('<?xml version="1.0"'))
        for role in (
            "data-role=\"centerline\"",
            "data-role=\"road\"",
            "data-role=\"banking\"",
            "data-role=\"elevation\"",
            "data-role=\"terrain-zone\"",
            "data-role=\"barrier\"",
        ):
            self.assertIn(role, source)
        centerline_d = source.split('data-role="centerline" d="', 1)[1].split('"', 1)[0]
        self.assertTrue(centerline_d.endswith("Z"))
        self.assertEqual(source.count('data-role="asset-instance"'), 7)
        self.assertEqual(source.count("data-role=\"barrier\""), 3)

    def test_centerline_is_closed_and_resampled(self):
        out = self.tmp / "out"
        self._run(out)
        data = self._normalized(out)
        self.assertTrue(data["centerline"]["closed"])
        self.assertGreater(data["centerline"]["point_count"], 10)
        self.assertGreater(data["centerline"]["length_m"], 0)

    def test_banking_elevation_and_barrier_present(self):
        out = self.tmp / "out"
        self._run(out)
        data = self._normalized(out)
        self.assertGreaterEqual(len(data["banking"]), 1)
        self.assertTrue(all(entry["degrees"] == 0.0 for entry in data["banking"]))
        self.assertGreaterEqual(len(data["elevation"]), 1)
        self.assertGreaterEqual(len(data["barriers"]), 1)
        self.assertEqual(data["road"]["width_m"], 12.0)

    # -- counts and ids ---------------------------------------------------

    def test_compiles_with_correct_counts_and_unique_ids(self):
        out = self.tmp / "out"
        self._run(out)
        self.assertTrue((out / CANONICAL_ARTIFACT).is_file())
        self.assertTrue((out / MANIFEST_ARTIFACT).is_file())
        data = self._normalized(out)
        assets = data["assets"]
        self.assertEqual(len(assets), 7)
        self.assertEqual(len({a["instance_id"] for a in assets}), 7)
        counts: dict[str, int] = {}
        for asset in assets:
            counts[asset["asset_id"]] = counts.get(asset["asset_id"], 0) + 1
        self.assertEqual(
            counts,
            {
                "spectator_wave": 1,
                "marshal_flag": 1,
                "track_flag": 1,
                "grass_v2_01": 2,
                "bush_v2_01": 1,
                "tree_v2_01": 1,
            },
        )
        self.assertFalse(any(a["collision"] for a in assets))

    def test_collision_validation_ok(self):
        out = self.tmp / "out"
        self._run(out)
        validation = self._normalized(out)["collision_validation"]
        for key in (
            "finite_vertices",
            "nondegenerate_triangles",
            "blender_winding_upward",
            "continuous_collision_grid",
            "safety_floor_valid",
        ):
            self.assertTrue(validation[key], key)

    # -- determinism ------------------------------------------------------

    def test_two_runs_are_byte_identical(self):
        out1 = self.tmp / "a"
        out2 = self.tmp / "b"
        source1, _ = self._run(out1)
        source2, _ = self._run(out2)
        self.assertEqual(source1, source2)
        for name in (CANONICAL_ARTIFACT, NORMALIZED_ARTIFACT, MANIFEST_ARTIFACT):
            self.assertEqual((out1 / name).read_bytes(), (out2 / name).read_bytes(), name)

    # -- parity -----------------------------------------------------------

    def test_parity_targets_pass(self):
        out = self.tmp / "out"
        self._run(out)
        data = self._normalized(out)
        diagnostics = verify_parity(
            self.compiled,
            data,
            object_category_target=self.mini_object_target,
            vegetation_category_target=self.mini_vegetation_target,
        )
        self.assertEqual(diagnostics, [])

    def test_parity_catches_count_drift(self):
        out = self.tmp / "out"
        self._run(out)
        data = self._normalized(out)
        diagnostics = verify_parity(
            self.compiled,
            data,
            object_category_target={"spectator": 2, "marshal": 1, "flag": 1},
            vegetation_category_target=self.mini_vegetation_target,
        )
        self.assertTrue(any("object category counts changed" in d for d in diagnostics))

    # -- rejection --------------------------------------------------------

    def test_unknown_asset_id_rejected(self):
        compiled = json.loads(json.dumps(self.compiled))
        compiled["vegetation"][0]["asset_path"] = (
            "blender/generated/la_chutana/raw_vegetation/assets_v2/glb/ghost_99.glb"
        )
        source = render_source_svg(compiled, self.centerline, self.catalog, self.config)
        with self.assertRaises(CompileError) as ctx:
            compile_track(
                source.encode("utf-8"),
                source_name=SOURCE_FILENAME,
                registry=self.registry,
                registry_path=self.registry_path,
                output_dir=self.tmp / "bad",
            )
        self.assertIn("unknown asset id", "; ".join(ctx.exception.diagnostics))

    def test_catalog_asset_id_mismatch_rejected(self):
        compiled = json.loads(json.dumps(self.compiled))
        compiled["objects"][0]["asset_id"] = "la_chutana"
        with self.assertRaises(ValueError):
            render_source_svg(compiled, self.centerline, self.catalog, self.config)

    # -- CLI --------------------------------------------------------------

    def test_cli_import_mini_fixture(self):
        import subprocess

        python = ROOT / "blender" / "track_pipeline" / ".venv" / "Scripts" / "python.exe"
        script = ROOT / "blender" / "track_pipeline" / "import_la_chutana.py"
        out_dir = self.tmp / "cli"
        result = subprocess.run(
            [
                str(python), str(script),
                "--compiled", str(FIXTURES / "la_chutana_compiled_mini.json"),
                "--centerline", str(FIXTURES / "la_chutana_centerline_mini.json"),
                "--catalog", str(FIXTURES / "la_chutana_catalog_mini.json"),
                "--config", str(FIXTURES / "la_chutana_config_mini.json"),
                "--registry", str(self.registry_path),
                "--output-dir", str(out_dir),
            ],
            capture_output=True,
            text=True,
            timeout=120,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((out_dir / SOURCE_FILENAME).is_file())
        self.assertTrue((out_dir / COMPILED_SUBDIR / CANONICAL_ARTIFACT).is_file())
        self.assertTrue((out_dir / COMPILED_SUBDIR / NORMALIZED_ARTIFACT).is_file())

    def test_cli_check_flags_parity_failure(self):
        import subprocess

        python = ROOT / "blender" / "track_pipeline" / ".venv" / "Scripts" / "python.exe"
        script = ROOT / "blender" / "track_pipeline" / "import_la_chutana.py"
        out_dir = self.tmp / "cli_check"
        result = subprocess.run(
            [
                str(python), str(script),
                "--compiled", str(FIXTURES / "la_chutana_compiled_mini.json"),
                "--centerline", str(FIXTURES / "la_chutana_centerline_mini.json"),
                "--catalog", str(FIXTURES / "la_chutana_catalog_mini.json"),
                "--config", str(FIXTURES / "la_chutana_config_mini.json"),
                "--registry", str(self.registry_path),
                "--output-dir", str(out_dir),
                "--check",
            ],
            capture_output=True,
            text=True,
            timeout=120,
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("object category counts changed", result.stderr + result.stdout)


def _rmtree(path: Path):
    if path.exists():
        for child in path.iterdir():
            if child.is_dir():
                _rmtree(child)
            else:
                child.unlink()
        path.rmdir()


if __name__ == "__main__":
    unittest.main()
