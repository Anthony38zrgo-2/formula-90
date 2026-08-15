[CmdletBinding()]
param(
    [string]$DumpRoot = "D:\Formula90s_Dump_Legacy_2026-08-14"
)

$ErrorActionPreference = "Stop"
$dumpPath = [IO.Path]::GetFullPath($DumpRoot).TrimEnd('\')
$manifestPath = Join-Path $dumpPath "manifest_sha256.json"

if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw "Manifest not found: $manifestPath"
}

$manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json
$verifiedEntries = [Collections.Generic.List[object]]::new()

foreach ($group in ($manifest.files | Group-Object source)) {
    $verifiedEntry = $null
    foreach ($entry in $group.Group) {
        if (-not (Test-Path -LiteralPath $entry.destination -PathType Leaf)) {
            continue
        }
        $actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $entry.destination).Hash.ToLowerInvariant()
        if ($actualHash -eq $entry.sha256) {
            $verifiedEntry = $entry
            break
        }
    }
    if ($null -eq $verifiedEntry) {
        throw "No verified archive copy for source: $($group.Name)"
    }
    $verifiedEntries.Add($verifiedEntry)
}

$repairedManifest = [ordered]@{
    schema = $manifest.schema
    created_at = $manifest.created_at
    verified_at = (Get-Date).ToString("o")
    workspace = $manifest.workspace
    dump_root = $manifest.dump_root
    protected = $manifest.protected
    file_count = $verifiedEntries.Count
    bytes = ($verifiedEntries | Measure-Object -Property bytes -Sum).Sum
    files = $verifiedEntries
}
$repairedManifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $manifestPath -Encoding UTF8

[pscustomobject]@{
    dump = $dumpPath
    files = $verifiedEntries.Count
    bytes = $repairedManifest.bytes
    verified = $verifiedEntries.Count
    manifest = $manifestPath
} | ConvertTo-Json
