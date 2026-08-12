"""Bootstrap the initial La Chutana authoring revision.

Runs the La Chutana importer to produce a canonical source SVG (with vegetation
regions when the semantic image is available), then creates the first immutable
revision under ``tracks/la_chutana/revisions/<sha>/`` and a scoped Git commit
containing only that track's authoring files.

The editor can then be started with ``--track la_chutana --tracks-dir tracks
--git-repo-root .`` so it always serves the tip revision.
"""

from __future__ import annotations

import argparse
from pathlib import Path
import sys
import tempfile

from authoring.authoring_store import TrackSession
from import_la_chutana import COMPILED_SUBDIR, render_source_svg
from svg_profile import sha256_file

_REPO_ROOT = Path(__file__).resolve().parents[2]
_SVG_SOURCE = "la_chutana.source.svg"


def _load_json(path: Path) -> dict:
    import json

    return json.loads(path.read_text(encoding="utf-8"))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Bootstrap the initial La Chutana authoring revision.")
    parser.add_argument("--compiled", default="blender/generated/la_chutana/semantic_layout/compiled_layout.json")
    parser.add_argument("--centerline", default="blender/generated/la_chutana/centerline.json")
    parser.add_argument("--catalog", default="blender/track_pipeline/layouts/la_chutana/object_catalog.json")
    parser.add_argument("--config", default="blender/track_pipeline/layouts/la_chutana/layout_config.json")
    parser.add_argument("--registry", default="blender/track_pipeline/configs/asset_registry.json")
    parser.add_argument("--tracks-dir", default="tracks")
    parser.add_argument("--git-repo-root", default=None)
    parser.add_argument("--force", action="store_true", help="Create a new revision even if one exists.")
    args = parser.parse_args(argv)

    def resolve(path: str) -> Path:
        candidate = Path(path)
        return candidate if candidate.is_absolute() else (_REPO_ROOT / candidate)

    compiled = _load_json(resolve(args.compiled))
    centerline = _load_json(resolve(args.centerline))
    catalog = _load_json(resolve(args.catalog))
    config = _load_json(resolve(args.config))

    source_text = render_source_svg(compiled, centerline, catalog, config)
    tracks_dir = resolve(args.tracks_dir)
    git_root = resolve(args.git_repo_root) if args.git_repo_root else _REPO_ROOT

    import_metadata = {
        "compiled": sha256_file(resolve(args.compiled)),
        "centerline": sha256_file(resolve(args.centerline)),
        "catalog": sha256_file(resolve(args.catalog)),
        "config": sha256_file(resolve(args.config)),
        "registry": sha256_file(resolve(args.registry)),
        "importer": "import_la_chutana.py",
    }

    with tempfile.TemporaryDirectory(prefix="f90_bootstrap_") as tmp:
        tmp_path = Path(tmp)
        source_path = tmp_path / _SVG_SOURCE
        source_path.write_text(source_text, encoding="utf-8")
        workspace = tmp_path / "workspace" / "track.source.svg"

        session = TrackSession(
            source_path,
            workspace,
            repo_root=_REPO_ROOT,
            tracks_dir=tracks_dir,
            git_repo_root=git_root,
        )

        if session.revision_count(track_id="la_chutana") > 0 and not args.force:
            print(f"track la_chutana already has revisions; use --force to add another.")
            print(f"tip: {session.state()['current_revision_sha256']}")
            return 0

        state = session.save(source_text)
        print(f"created revision: {state['current_revision_sha256']}")
        print(f"revision dir: {tracks_dir / 'la_chutana' / 'revisions' / state['current_revision_sha256']}")
        print(f"asset instances: {state.get('revision_count')} revisions")
        print(f"import provenance: {import_metadata}")
        return 0


if __name__ == "__main__":
    raise SystemExit(main())
