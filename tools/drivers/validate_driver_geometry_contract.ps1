[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$ProjectDirectory,
    [string]$DriverDirectory
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($DriverDirectory)) {
    $DriverDirectory = Join-Path $ProjectDirectory 'game/assets/models/drivers'
}
$driverManifestPath = Join-Path $DriverDirectory 'driver_manifest.json'
$driverModelPath = Join-Path $DriverDirectory 'driver.glb'
$driverPreparedSourcePath = Join-Path $DriverDirectory 'source/prepared_driver.blend'
$driverOriginalSourcePath = Join-Path $ProjectDirectory 'game/assets/models/drivers/source/source/driver.blend'
$driverManifest = Get-Content -LiteralPath $driverManifestPath -Raw | ConvertFrom-Json
$expectedOriginalDigest = '28eded787a300e10bae2f955431f49b9a7a62be9dcc81dbee4e1f011dcaab12b'
if ($driverManifest.character -ne 'low_polygon_race_car_driver' -or $driverManifest.geometry_contract_version -ne 1) {
    throw 'Driver rechazado: falta el contrato de geometria original con escala uniforme.'
}
$driverGeometry = $driverManifest.uniform_source_geometry
$driverVerification = $driverManifest.original_geometry_verification
if ($null -eq $driverGeometry -or $driverGeometry.individual_part_scaling -ne $false -or $driverGeometry.geometry_sculpting -ne $false -or $driverGeometry.skeleton_adapted_to_original_anatomy -ne $true -or $driverGeometry.uniform_scale -le 0.0) {
    throw 'Driver rechazado: anatomia deformada o escala uniforme no demostrada.'
}
if ($null -eq $driverVerification -or $driverVerification.verified -ne $true -or $driverVerification.maximum_error_meters -gt 0.000001 -or $driverVerification.vertex_count -ne 1171 -or $driverVerification.triangle_count -ne 1445 -or $driverManifest.triangle_count -ne 1445) {
    throw 'Driver rechazado: la geometria no supera la comparacion contra el original.'
}
foreach ($fileExpectation in @(
    @{ Path = $driverOriginalSourcePath; Digest = $expectedOriginalDigest },
    @{ Path = $driverModelPath; Digest = $driverManifest.model_sha256 },
    @{ Path = $driverPreparedSourcePath; Digest = $driverManifest.prepared_source_sha256 }
)) {
    $actualDigest = (Get-FileHash -LiteralPath $fileExpectation.Path -Algorithm SHA256).Hash
    if (-not $actualDigest.Equals([string]$fileExpectation.Digest, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Driver rechazado: hash inesperado en $($fileExpectation.Path)"
    }
}
if ($driverManifest.source_sha256 -ne $expectedOriginalDigest) {
    throw 'Driver rechazado: el modelo no procede de la fuente original requerida.'
}
$driverBytes = [System.IO.File]::ReadAllBytes($driverModelPath)
if ($driverBytes.Length -lt 20 -or [System.BitConverter]::ToUInt32($driverBytes, 0) -ne 0x46546C67 -or [System.BitConverter]::ToUInt32($driverBytes, 16) -ne 0x4E4F534A) {
    throw 'Driver rechazado: archivo GLB invalido.'
}
$driverDocument = [System.Text.Encoding]::UTF8.GetString($driverBytes, 20, [System.BitConverter]::ToInt32($driverBytes, 12)) | ConvertFrom-Json
$driverContractNodes = @($driverDocument.nodes | Where-Object { $_.name -eq 'DriverOriginalUniformGeometryContract' })
if ($driverContractNodes.Count -ne 1 -or $driverContractNodes[0].extras.contract_version -ne 1 -or $driverContractNodes[0].extras.source_sha256 -ne $expectedOriginalDigest -or $driverContractNodes[0].extras.neutral_geometry_sha256 -ne $driverVerification.neutral_geometry_sha256 -or [math]::Abs($driverContractNodes[0].extras.uniform_scale - $driverGeometry.uniform_scale) -gt 0.00000001) {
    throw 'Driver rechazado: el GLB no contiene el contrato de geometria verificado.'
}
Write-Host 'Driver original con escala uniforme validado.' -ForegroundColor Green
