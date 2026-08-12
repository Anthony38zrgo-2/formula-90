# PSX Visuals GD4 upstream pin

- Repository: https://github.com/scolastico/psx_visuals_gd4
- Commit: `9668c203f25f559ac36b6d0e468f4ed446ca4eee`
- Retrieved: 2026-08-11
- License: MIT (upstream `LICENSE` copied as `LICENSE.md`).

The addon is retained as an optional pinned dependency but is disabled in both
the editor and runtime. Formula-90 uses a native scene profile instead: low
internal resolution, nearest texture filtering, depth fog, simple ambient plus
directional lighting, and a selective shadow budget. This avoids the upstream
Auto-Apply behavior that replaces imported `StandardMaterial3D` materials and
retains only albedo/emission.
