param(
    [int]$Seconds = 5
)

. "$PSScriptRoot\_common.ps1"

if ($Seconds -lt 1) { throw "Seconds debe ser >= 1." }

Write-Host "Actualizando conteos cada $Seconds segundos. Ctrl+C para salir."

while ($true) {
    Clear-Host
    Write-Host ("DB: {0}" -f $DbPath)
    Write-Host ("Hora: {0}" -f (Get-Date))
    Write-Host ""
    try {
        & "$PSScriptRoot\03-db-counts.ps1"
    } catch {
        Write-Error $_
    }
    Start-Sleep -Seconds $Seconds
}
