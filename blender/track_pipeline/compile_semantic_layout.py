from __future__ import annotations

import argparse
import json
from pathlib import Path

from semantic_layout_common import compile_layout


def main() -> int:
    parser = argparse.ArgumentParser(description="Compile semantic track zones and indexed object markers deterministically.")
    parser.add_argument("--layout-config", required=True)
    parser.add_argument("--output")
    args = parser.parse_args()
    config_path = Path(args.layout_config).resolve()
    config = json.loads(config_path.read_text(encoding="utf-8"))
    root = config_path.parents[4]
    output = Path(args.output).resolve() if args.output else root / config["compiled_output"]
    result = compile_layout(config_path)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"output": str(output), **result["counts"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
