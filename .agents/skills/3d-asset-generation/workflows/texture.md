# Texture

Classify as `TEXTURE` or `UV`. Keep mesh regions and image operations separate.
Use vertex colors for flat low-poly palettes. Use UV textures for livery,
logos, numbers, and localized patterns.

For a deterministic flat texture or test mask:

```powershell
tools\asset_pipeline\.venv\Scripts\python.exe .agents\skills\3d-asset-generation\scripts\texture_tools.py working\textured\base.png --size 256 256 --color 3B5D3B --seed 90 --grid 16
```

Use Pillow for this simple case. Use OpenCV for morphology, segmentation,
advanced masks, edge detection, or image transforms that Pillow does not
provide directly. Record the seed and parameters.
