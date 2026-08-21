# [DEPRECATED] LOW-POLY VEGETATION PACK
NOTE: This 85-asset LOD pack is DEPRECATED and inactive.
The authoritative and canonical track vegetation is located at:
  D:\Formula90s\assets-lowpoly-python\nature\

LOW-POLY VEGETATION COMPLETE PACK

Generated from the supplied autumn forest GLB plus procedural low-poly additions.
Total GLB assets: 85

Folders:
- trees/original: preserved original separated trees
- trees/conifer: cypress/spruce/cedar-like variants, LOD0 + LOD3
- trees/deciduous: oak/poplar/elm/eucalyptus/acacia/ornamental variants
- trees/tropical: palms and banana plants
- trees/dead: standing dead trees, stump and fallen log
- shrubs: wild/dense shrubs, dry scrub and hedges
- grass: green/dry clumps and very cheap cards
- groundcover: weeds, ferns, reeds and small plants

Godot notes:
- Files are GLB and Y-up.
- Main modified trees retain the original embedded materials/textures.
- Procedural additions use simple PBR colors and very low geometry.
- Use LOD0 near the camera and LOD3 farther away where paired variants exist.
- Grass/groundcover are intended for MultiMeshInstance3D / instancing.
- Consider disabling collision on grass/shrubs and using simplified collision only on tree trunks near driveable areas.

Naming convention:
veg_tree_<type>_<variant>_lod0.glb / lod3.glb
veg_grass_<type>_<variant>.glb
veg_shrub_<type>_<variant>.glb
