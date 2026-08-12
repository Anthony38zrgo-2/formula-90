# Asset Pipeline Environment

This directory owns the isolated Python environment for deterministic 3D asset
work. It is not the project Python and is not installed globally.

Create or recreate it on Windows with Python 3.12 (Open3D provides the most
reliable Windows wheel there):

```powershell
python -m venv tools/asset_pipeline/.venv
tools\asset_pipeline\.venv\Scripts\python.exe -m pip install --upgrade pip setuptools wheel
tools\asset_pipeline\.venv\Scripts\python.exe -m pip install -r tools/asset_pipeline/requirements.txt
```

The pinned environment has been verified with `pip check` and imports for
NumPy, SciPy, PyMeshLab, Trimesh, Open3D, Pillow, OpenCV, NetworkX, Shapely,
Rtree, and Pyglet. If an optional package cannot be installed on a platform,
keep the core pipeline usable and only disable the feature that needs it:

| Package | Degraded feature |
| --- | --- |
| Open3D | point-cloud registration and ICP |
| OpenCV | advanced texture masks and image morphology |
| Rtree | accelerated Trimesh proximity queries |
| Pyglet | optional Trimesh visualization |

The reusable commands live in
`.agents/skills/3d-asset-generation/scripts/`. Keep asset stages separate as
`input/`, `working/`, `output/`, and `reports/`; these are pipeline conventions,
not locations for the virtual environment itself.
