[CmdletBinding()]
param(
    [string]$Workspace = "D:\Formula90s",
    [string]$DumpRoot = "D:\Formula90s_Dump_Legacy_2026-08-14"
)

$ErrorActionPreference = "Stop"

$workspacePath = [IO.Path]::GetFullPath($Workspace).TrimEnd('\')
$dumpPath = [IO.Path]::GetFullPath($DumpRoot).TrimEnd('\')
$protectedAssetPath = Join-Path $workspacePath "assets-lowpoly-python"
$protectedBackendPath = Join-Path $workspacePath "blender\track_pipeline\blender_backend"

if (-not (Test-Path -LiteralPath $workspacePath -PathType Container)) {
    throw "Workspace not found: $workspacePath"
}
if ($dumpPath -eq $workspacePath -or $dumpPath.StartsWith($workspacePath + '\', [StringComparison]::OrdinalIgnoreCase)) {
    throw "Dump must be outside the workspace: $dumpPath"
}

$targets = @(
    @{ Category = "vehicles_legacy"; Relative = "blender\models" },
    @{ Category = "vehicles_legacy"; Relative = "blender\backup" },
    @{ Category = "vehicles_legacy"; Relative = "blender\current" },
    @{ Category = "vehicles_legacy"; Relative = "blender\cgtrader_optimized_F1_Concept_2026.fbx" },
    @{ Category = "vehicles_legacy"; Relative = "blender\f12030-v10.blend" },
    @{ Category = "vehicles_legacy"; Relative = "blender\f12030-v10.mtl" },
    @{ Category = "vehicles_legacy"; Relative = "blender\f12030-v10.obj" },
    @{ Category = "vehicles_legacy"; Relative = "blender\f12030-v10_fixed.obj" },
    @{ Category = "vehicles_legacy"; Relative = "blender\F1.obj" },
    @{ Category = "vehicles_legacy"; Relative = "blender\F1-97_front_wing_symmetric.blend" },
    @{ Category = "vehicles_legacy"; Relative = "blender\F1-97_front_wing_symmetric.blend1" },
    @{ Category = "vehicles_legacy"; Relative = "blender\F1-97_front_wing_symmetric.mtl" },
    @{ Category = "vehicles_legacy"; Relative = "blender\F1-97_front_wing_symmetric.obj" },
    @{ Category = "vehicles_legacy"; Relative = "blender\F1-99.blend" },
    @{ Category = "vehicles_legacy"; Relative = "blender\Lotus_88_OBJ.obj" },
    @{ Category = "track_history"; Relative = "blender\generated\la_chutana\backups" },
    @{ Category = "track_history"; Relative = "blender\generated\la_chutana\raw_vegetation\backups" },
    @{ Category = "generated_diagnostics"; Relative = "blender\generated\godot_import_probe" },
    @{ Category = "generated_diagnostics"; Relative = "blender\generated\godot_import_probe_failed" },
    @{ Category = "generated_diagnostics"; Relative = "blender\generated\collision_group_probe" },
    @{ Category = "generated_diagnostics"; Relative = "blender\generated\godot_probe_nonveg" },
    @{ Category = "generated_diagnostics"; Relative = "blender\generated\godot_probe_veg" },
    @{ Category = "generated_diagnostics"; Relative = "blender\generated\analysis_suite" },
    @{ Category = "generated_diagnostics"; Relative = "blender\generated\diagnostics" },
    @{ Category = "generated_diagnostics"; Relative = "blender\generated\vehicles" },
    @{ Category = "generated_diagnostics"; Relative = "blender\generated\agentdb_collision_diagnostic.db" },
    @{ Category = "generated_diagnostics"; Relative = "blender\generated\export_collision_probe_groups.py" },
    @{ Category = "generated_diagnostics"; Relative = "blender\generated\collision_runtime_probe.gd" },
    @{ Category = "generated_diagnostics"; Relative = "blender\obj_validator\logs" }
)

$cacheDirectories = Get-ChildItem -LiteralPath (Join-Path $workspacePath "blender") -Directory -Recurse -Force |
    Where-Object {
        $_.Name -eq "__pycache__" -and
        -not $_.FullName.StartsWith((Join-Path $workspacePath "blender\track_pipeline\.venv") + '\', [StringComparison]::OrdinalIgnoreCase) -and
        -not $_.FullName.StartsWith($protectedBackendPath + '\', [StringComparison]::OrdinalIgnoreCase)
    }
foreach ($cacheDirectory in $cacheDirectories) {
    $relative = $cacheDirectory.FullName.Substring($workspacePath.Length + 1)
    $targets += @{ Category = "reproducible_caches"; Relative = $relative }
}

$targets = $targets | Sort-Object { $_.Relative.Length } -Descending
$entries = [Collections.Generic.List[object]]::new()
$existingTargets = [Collections.Generic.List[object]]::new()
$seenSourceFiles = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)

foreach ($target in $targets) {
    $source = [IO.Path]::GetFullPath((Join-Path $workspacePath $target.Relative))
    if (-not $source.StartsWith($workspacePath + '\', [StringComparison]::OrdinalIgnoreCase)) {
        throw "Unsafe source path: $source"
    }
    if ($source.StartsWith($protectedAssetPath + '\', [StringComparison]::OrdinalIgnoreCase) -or
        $source.StartsWith($protectedBackendPath, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Protected path selected: $source"
    }
    if (-not (Test-Path -LiteralPath $source)) {
        continue
    }

    $destination = [IO.Path]::GetFullPath((Join-Path (Join-Path $dumpPath $target.Category) $target.Relative))
    if (-not $destination.StartsWith($dumpPath + '\', [StringComparison]::OrdinalIgnoreCase)) {
        throw "Unsafe destination path: $destination"
    }
    if (Test-Path -LiteralPath $destination) {
        throw "Destination already exists: $destination"
    }

    $files = if (Test-Path -LiteralPath $source -PathType Container) {
        @(Get-ChildItem -LiteralPath $source -Recurse -File -Force)
    } else {
        @(Get-Item -LiteralPath $source -Force)
    }
    foreach ($file in $files) {
        if (-not $seenSourceFiles.Add($file.FullName)) {
            continue
        }
        $relativeWithinTarget = if (Test-Path -LiteralPath $source -PathType Container) {
            $file.FullName.Substring($source.Length).TrimStart('\')
        } else {
            $file.Name
        }
        $destinationFile = if (Test-Path -LiteralPath $source -PathType Container) {
            Join-Path $destination $relativeWithinTarget
        } else {
            $destination
        }
        $entries.Add([ordered]@{
            source = $file.FullName
            destination = $destinationFile
            bytes = $file.Length
            sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $file.FullName).Hash.ToLowerInvariant()
        })
    }
    $existingTargets.Add([ordered]@{
        source = $source
        destination = $destination
        category = $target.Category
    })
}

New-Item -ItemType Directory -Force -Path $dumpPath | Out-Null
$manifest = [ordered]@{
    schema = "formula90s/local-legacy-dump/v1"
    created_at = (Get-Date).ToString("o")
    workspace = $workspacePath
    dump_root = $dumpPath
    protected = @($protectedAssetPath, $protectedBackendPath)
    files = $entries
}
$manifestPath = Join-Path $dumpPath "manifest_sha256.json"
$manifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $manifestPath -Encoding UTF8

foreach ($target in $existingTargets) {
    $parent = Split-Path -Parent $target.destination
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
    Move-Item -LiteralPath $target.source -Destination $target.destination
}

$verified = 0
foreach ($entry in $entries) {
    if (-not (Test-Path -LiteralPath $entry.destination -PathType Leaf)) {
        throw "Archived file missing: $($entry.destination)"
    }
    $actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $entry.destination).Hash.ToLowerInvariant()
    if ($actualHash -ne $entry.sha256) {
        throw "Archived hash mismatch: $($entry.destination)"
    }
    $verified++
}

[pscustomobject]@{
    dump = $dumpPath
    files = $entries.Count
    bytes = ($entries | Measure-Object -Property bytes -Sum).Sum
    verified = $verified
    manifest = $manifestPath
} | ConvertTo-Json
