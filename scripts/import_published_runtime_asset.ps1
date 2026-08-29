<## Import a factory-published runtime package without invoking any authoring tool. ##>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateScript({ Test-Path -LiteralPath $_ -PathType Container })]
    [string]$Package,
    [Parameter(Mandatory = $true)]
    [ValidateScript({ Test-Path -LiteralPath $_ -PathType Container })]
    [string]$Destination
)
$ErrorActionPreference = 'Stop'
$packageRoot = (Resolve-Path -LiteralPath $Package).Path
$destinationRoot = (Resolve-Path -LiteralPath $Destination).Path
$manifestPath = Join-Path $packageRoot 'package_manifest.json'
if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) { throw "Package manifest missing: $manifestPath" }
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
foreach ($entry in @($manifest.files)) {
    if ([IO.Path]::IsPathRooted($entry.file) -or $entry.file.Contains('..')) { throw "Unsafe package path: $($entry.file)" }
    $source = Join-Path $packageRoot $entry.file
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) { throw "Package file missing: $($entry.file)" }
    $actual = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $entry.sha256.ToLowerInvariant()) { throw "SHA-256 mismatch: $($entry.file)" }
    Copy-Item -LiteralPath $source -Destination (Join-Path $destinationRoot $entry.file) -Force
}
Write-Host "Imported validated $($manifest.package_type) package: $($manifest.vehicle_id)"
