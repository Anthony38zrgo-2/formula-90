import unittest
from pathlib import Path
import hashlib
import json
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "blender" / "track_pipeline"))

from asset_registry import AssetRegistry, AssetSpec, load_registry
from compile_svg_track import (
    CANONICAL_ARTIFACT,
    COMPILER_VERSION,
    MANIFEST_ARTIFACT,
    NORMALIZED_ARTIFACT,
    CompileError,
    compile_track,
)
from svg_sanitizer import canonical_xml, sanitize_svg
from svg_normalizer import normalize

FIXTURES = ROOT / "blender" / "track_pipeline" / "tests" / "fixtures"
SOURCE = FIXTURES / "compile_track.svg"
REGISTRY_PATH = FIXTURES / "asset_registry_test.json"

BASE = SOURCE.read_text(encoding="utf-8")


def _sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


class CompileTrackTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="f90_compile_"))
        self.addCleanup(_rmtree, self.tmp)
        self.registry = load_registry(REGISTRY_PATH, repo_root=ROOT)

    def _compile(self, source_text: str, *, registry=None, dirname="out"):
        out = self.tmp / dirname
        manifest = compile_track(
            source_text.encode("utf-8"),
            source_name="track.svg",
            registry=registry or self.registry,
            registry_path=REGISTRY_PATH,
            output_dir=out,
        )
        return out, manifest

    def _reject(self, source_text: str, needle: str):
        with self.assertRaises(CompileError) as ctx:
            self._compile(source_text)
        joined = "; ".join(ctx.exception.diagnostics)
        self.assertIn(needle, joined)
        return ctx.exception

    # -- determinism -------------------------------------------------------

    def test_two_runs_are_byte_identical(self):
        out1, _ = self._compile(BASE, dirname="out1")
        out2, _ = self._compile(BASE, dirname="out2")
        for name in (CANONICAL_ARTIFACT, NORMALIZED_ARTIFACT, MANIFEST_ARTIFACT):
            self.assertEqual((out1 / name).read_bytes(), (out2 / name).read_bytes(), name)

    def test_only_generated_artifacts_are_written(self):
        out, _ = self._compile(BASE)
        self.assertEqual(
            {path.name for path in out.iterdir()},
            {CANONICAL_ARTIFACT, MANIFEST_ARTIFACT, NORMALIZED_ARTIFACT},
        )

    def test_manifest_hashes_match_artifacts(self):
        out, manifest = self._compile(BASE)
        self.assertEqual(manifest["artifacts"]["source"]["sha256"], _sha(BASE.encode("utf-8")))
        self.assertEqual(
            manifest["artifacts"]["canonical"]["sha256"],
            _sha((out / CANONICAL_ARTIFACT).read_bytes()),
        )
        self.assertEqual(
            manifest["artifacts"]["normalized"]["sha256"],
            _sha((out / NORMALIZED_ARTIFACT).read_bytes()),
        )
        self.assertEqual(
            manifest["artifacts"]["registry"]["sha256"],
            _sha(REGISTRY_PATH.read_bytes()),
        )
        self.assertEqual(manifest["compiler"]["name"], "compile_svg_track")
        self.assertEqual(manifest["compiler"]["version"], COMPILER_VERSION)
        self.assertEqual(len(manifest["compiler"]["sha256"]), 64)
        self.assertEqual(manifest["track_id"], "compile_fixture")

    def test_normalized_json_matches_normalizer_contract(self):
        out, _ = self._compile(BASE)
        data = json.loads((out / NORMALIZED_ARTIFACT).read_text(encoding="utf-8"))
        self.assertEqual(data["schema_version"], 1)
        self.assertTrue(data["centerline"]["closed"])
        self.assertGreater(data["centerline"]["point_count"], 10)
        self.assertEqual(data["elevation"][0]["s_m"], 60.0)
        self.assertEqual(data["elevation"][1]["height_m"], -2.0)

    def test_track_config_carries_elevation_and_length(self):
        out, _ = self._compile(BASE)
        data = json.loads((out / NORMALIZED_ARTIFACT).read_text(encoding="utf-8"))
        config = data["track_config"]
        self.assertEqual(len(config["elevation"]), 2)
        self.assertGreater(config["centerline_length_m"], 0)
        self.assertEqual(config["elevation"][0]["s_m"], 60.0)
        self.assertEqual(config["elevation"][1]["height_m"], -2.0)

    def test_elevation_shifts_road_surface_and_preserves_seam(self):
        from terrain_grid import (
            NearestTrackSample,
            road_surface_height,
            terrain_height_from_sample,
        )

        out, _ = self._compile(BASE)
        data = json.loads((out / NORMALIZED_ARTIFACT).read_text(encoding="utf-8"))
        config = data["track_config"]
        length = config["centerline_length_m"]
        half = config["road"]["width_m"] * 0.5

        frac_at = lambda s: (s % length) / length
        for s, expected in ((60.0, 1.5), (180.0, -2.0)):
            fraction = frac_at(s)
            elev = road_surface_height(config, fraction, 0.0) - config["road"]["surface_elevation_m"]
            self.assertAlmostEqual(elev, expected, places=3)

        for fraction in (0.0, 0.25, 0.5, 0.75, 0.9):
            for side in (-1.0, 1.0):
                signed = half * side
                seam_h = terrain_height_from_sample(
                    config,
                    NearestTrackSample(half, fraction, side, signed),
                    visual=False,
                )
                road_h = road_surface_height(config, fraction, signed)
                self.assertLessEqual(abs(seam_h - road_h), 1e-6)

    # -- asset registry resolution ----------------------------------------

    def test_unknown_asset_id_rejected(self):
        broken = BASE.replace('data-asset-id="track_flag"', 'data-asset-id="ghost_01"')
        self._reject(broken, "unknown asset id")

    def test_out_of_budget_maximum_rejected(self):
        extra = (
            '<circle data-role="asset-instance" data-instance-id="tree_b" '
            'data-asset-id="tree_v2_01" cx="30" cy="30" r="0.4" data-scale="1.0"/>'
            '<circle data-role="asset-instance" data-instance-id="tree_c" '
            'data-asset-id="tree_v2_01" cx="40" cy="30" r="0.4" data-scale="1.0"/>'
        )
        broken = BASE.replace('</svg>', extra + "</svg>")
        self._reject(broken, "above budget maximum 2")

    def test_below_budget_minimum_rejected(self):
        tree = AssetSpec(
            id="tree_v2_01",
            kind="vegetation",
            category="trees",
            source="blender/track_pipeline/tests/fixtures/assets/dummy_tree.glb",
            dimensions_m={"width": 10.8, "height": 17.2, "depth": 10.8},
            budget={"min_instances": 2, "max_instances": 2},
        )
        flag = AssetSpec(
            id="track_flag",
            kind="flag",
            category="flag",
            source=None,
            dimensions_m={"width": 0.8, "height": 1.8, "depth": 0.05},
            budget={"min_instances": 0, "max_instances": 0},
        )
        registry = AssetRegistry([tree, flag], repo_root=ROOT)
        with self.assertRaises(CompileError) as ctx:
            self._compile(BASE, registry=registry)
        self.assertIn("below budget minimum 2", "; ".join(ctx.exception.diagnostics))

    def test_asset_kind_comes_from_registry_not_source(self):
        _, manifest = self._compile(BASE)
        normalized = json.loads(
            (self.tmp / "out" / NORMALIZED_ARTIFACT).read_text(encoding="utf-8")
        )
        by_id = {asset["asset_id"]: asset for asset in normalized["assets"]}
        self.assertEqual(by_id["tree_v2_01"]["kind"], "vegetation")

    # -- stable instance ids ----------------------------------------------

    def test_authored_instance_ids_survive_into_canonical_and_normalized(self):
        out, _ = self._compile(BASE)
        canonical = (out / CANONICAL_ARTIFACT).read_text(encoding="utf-8")
        self.assertIn('data-instance-id="tree_a"', canonical)
        self.assertIn('data-instance-id="flag_a"', canonical)
        normalized = json.loads((out / NORMALIZED_ARTIFACT).read_text(encoding="utf-8"))
        ids = [asset["instance_id"] for asset in normalized["assets"]]
        self.assertEqual(ids, ["tree_a", "flag_a"])

    def test_missing_instance_ids_assigned_once_and_persisted(self):
        stripped = BASE.replace('data-instance-id="tree_a"', "").replace('data-instance-id="flag_a"', "")
        out, _ = self._compile(stripped)
        canonical = (out / CANONICAL_ARTIFACT).read_text(encoding="utf-8")
        self.assertIn('data-instance-id="asset_0000"', canonical)
        self.assertIn('data-instance-id="asset_0001"', canonical)
        normalized = json.loads((out / NORMALIZED_ARTIFACT).read_text(encoding="utf-8"))
        ids = [asset["instance_id"] for asset in normalized["assets"]]
        self.assertEqual(ids, ["asset_0000", "asset_0001"])
        first = normalize(sanitize_svg((out / CANONICAL_ARTIFACT).read_bytes()))
        second = normalize(sanitize_svg((out / CANONICAL_ARTIFACT).read_bytes()))
        self.assertEqual(
            [asset["instance_id"] for asset in first["assets"]],
            [asset["instance_id"] for asset in second["assets"]],
        )

    def test_duplicate_instance_id_rejected(self):
        broken = BASE.replace('data-instance-id="flag_a"', 'data-instance-id="tree_a"')
        self._reject(broken, "duplicate data-instance-id")

    def test_invalid_instance_id_rejected(self):
        broken = BASE.replace('data-instance-id="tree_a"', 'data-instance-id="Bad Id!"')
        self._reject(broken, "invalid data-instance-id")

    # -- non-finite numbers -----------------------------------------------

    def test_non_finite_scale_rejected(self):
        broken = BASE.replace('data-scale="1.0"', 'data-scale="nan"', 1)
        self._reject(broken, "finite")

    def test_non_finite_path_number_rejected(self):
        broken = BASE.replace("L 30 12", "L 30 Inf")
        self._reject(broken, "non-finite")

    # -- scale and road width ---------------------------------------------

    def test_zero_scale_rejected(self):
        broken = BASE.replace('data-scale="1.0"', 'data-scale="0.0"', 1)
        self._reject(broken, "scale must be positive")

    def test_negative_scale_rejected(self):
        broken = BASE.replace('data-scale="1.0"', 'data-scale="-2.0"', 1)
        self._reject(broken, "scale must be positive")

    def test_zero_road_width_rejected(self):
        broken = BASE.replace('data-width-m="12"', 'data-width-m="0"')
        self._reject(broken, "road width must be a positive number")

    def test_negative_road_width_rejected(self):
        broken = BASE.replace('data-width-m="12"', 'data-width-m="-4"')
        self._reject(broken, "road width must be a positive number")

    # -- malformed path parameter counts ----------------------------------

    def test_path_odd_parameter_count_rejected(self):
        broken = BASE.replace("L 30 12 L 70 12", "L 30 12 70")
        self._reject(broken, "parameters")

    def test_path_missing_parameters_rejected(self):
        broken = BASE.replace("L 30 12 L 70 12", "L 30 12 L")
        self._reject(broken, "missing")

    def test_path_command_with_excess_parameters_rejected(self):
        broken = BASE.replace("Z", "Z 5")
        self._reject(broken, "must not take parameters")

    # -- centerline topology ----------------------------------------------

    def test_open_centerline_rejected(self):
        broken = BASE.replace("Z", "")
        self._reject(broken, "closed")

    def test_self_intersecting_centerline_rejected(self):
        bowtie = "M 10 10 L 90 70 L 90 10 L 10 70 Z"
        broken = BASE.replace("M 10 40 L 30 12 L 70 12 L 110 40 L 70 68 L 30 68 Z", bowtie)
        self._reject(broken, "self-intersect")

    def test_multiple_subpaths_rejected(self):
        doubled = "M 10 40 L 30 12 L 70 12 L 110 40 L 70 68 L 30 68 Z M 20 20 L 40 20 L 40 40 L 20 40 Z"
        broken = BASE.replace("M 10 40 L 30 12 L 70 12 L 110 40 L 70 68 L 30 68 Z", doubled)
        self._reject(broken, "single closed subpath")

    # -- control ranges ----------------------------------------------------

    def test_banking_out_of_range_rejected(self):
        broken = BASE.replace('data-degrees="3.5"', 'data-degrees="60"')
        self._reject(broken, "outside allowed range")

    def test_banking_negative_s_rejected(self):
        broken = BASE.replace('data-s-m="30"', 'data-s-m="-30"')
        self._reject(broken, "data-s-m must be >= 0")

    def test_elevation_out_of_range_rejected(self):
        broken = BASE.replace('data-height-m="1.5"', 'data-height-m="100"')
        self._reject(broken, "outside allowed range")

    def test_elevation_negative_s_rejected(self):
        broken = BASE.replace('data-s-m="60" data-height-m="1.5"', 'data-s-m="-60" data-height-m="1.5"')
        self._reject(broken, "data-s-m must be >= 0")

    # -- CLI behavior ------------------------------------------------------

    def test_cli_requires_explicit_arguments(self):
        import subprocess

        python = ROOT / "blender" / "track_pipeline" / ".venv" / "Scripts" / "python.exe"
        script = ROOT / "blender" / "track_pipeline" / "compile_svg_track.py"
        result = subprocess.run(
            [str(python), str(script)],
            capture_output=True,
            text=True,
            timeout=60,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("--source", result.stderr + result.stdout)

    def test_cli_compile_success_and_rejection_exit_codes(self):
        import subprocess

        python = ROOT / "blender" / "track_pipeline" / ".venv" / "Scripts" / "python.exe"
        script = ROOT / "blender" / "track_pipeline" / "compile_svg_track.py"
        out_dir = self.tmp / "cli_out"

        ok = subprocess.run(
            [
                str(python), str(script),
                "--source", str(SOURCE),
                "--registry", str(REGISTRY_PATH),
                "--output-dir", str(out_dir),
            ],
            capture_output=True,
            text=True,
            timeout=120,
        )
        self.assertEqual(ok.returncode, 0, ok.stderr)
        self.assertTrue((out_dir / CANONICAL_ARTIFACT).exists())

        broken_source = self.tmp / "broken.svg"
        broken_source.write_text(BASE.replace("Z", ""), encoding="utf-8")
        fail = subprocess.run(
            [
                str(python), str(script),
                "--source", str(broken_source),
                "--registry", str(REGISTRY_PATH),
                "--output-dir", str(self.tmp / "cli_bad"),
            ],
            capture_output=True,
            text=True,
            timeout=120,
        )
        self.assertEqual(fail.returncode, 2)


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
