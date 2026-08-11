#!/usr/bin/env python3
"""Primary invocation for the Formula90s analysis suite.

Runs with Blender 5.2's bundled Python:
  & "C:\\Program Files\\Blender Foundation\\Blender 5.2\\5.2\\python\\bin\\python.exe" ^
      blender\\analysis_suite\\scripts\\run_analysis.py <subcommand> ^
      --config <repo-relative-json> --report <repo-relative-json>
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from analysis_suite.cli import main  # noqa: E402

if __name__ == '__main__':
    sys.exit(main())
