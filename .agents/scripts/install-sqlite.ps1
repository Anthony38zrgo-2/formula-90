<#
.SYNOPSIS
    Instala las herramientas SQLite CLI de forma local en <repo>\.agents\tools\sqlite\
    sin modificar PATH.

.DESCRIPTION
    Idempotente: si sqlite3.exe local ya funciona, no re-descarga nada.
    Descarga únicamente del sitio oficial sqlite.org.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. "$PSScriptRoot\_common.ps1"

$SqliteDir = Join-Path $AgentsRoot "tools\sqlite"
$SqliteExe = Join-Path $SqliteDir "sqlite3.exe"
$ArchiveName = "sqlite-tools-win-x64-3530400.zip"
$DownloadUrl = "https://www.sqlite.org/2026/$ArchiveName"
$ZipPath = Join-Path $env:TEMP $ArchiveName
$ExtractDir = Join-Path $env:TEMP "formula90-sqlite-tools"
$Binaries = @("sqlite3.exe", "sqldiff.exe", "sqlite3_analyzer.exe", "sqlite3_rsync.exe")

function Test-SqliteWorks {
    param([string]$Path)
    if (-not (Test-Path $Path)) { return $false }
    try {
        & $Path --version 2>$null | Out-Null
        return ($LASTEXITCODE -eq 0)
    } catch {
        return $false
    }
}

Write-Host "=== install-sqlite ==="
Write-Host "Repo root : $RepoRoot"
Write-Host "Destino   : $SqliteDir"

if (Test-SqliteWorks $SqliteExe) {
    Write-Host "SQLite local ya funciona; no se descarga nada."
    $alreadyInstalled = $true
} else {
    $alreadyInstalled = $false
}

if (-not $alreadyInstalled) {
    if (-not (Test-Path $ZipPath)) {
        Write-Host "Descargando $DownloadUrl ..."
        Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipPath -UseBasicParsing -TimeoutSec 120
    } else {
        Write-Host "Reusando ZIP existente: $ZipPath"
    }

    if (Test-Path $ExtractDir) {
        Remove-Item $ExtractDir -Recurse -Force
    }
    New-Item -ItemType Directory -Path $ExtractDir | Out-Null
    Expand-Archive -Path $ZipPath -DestinationPath $ExtractDir -Force

    New-Item -ItemType Directory -Force -Path $SqliteDir | Out-Null
    foreach ($binary in $Binaries) {
        $src = Join-Path $ExtractDir $binary
        if (Test-Path $src) {
            Copy-Item $src (Join-Path $SqliteDir $binary) -Force
            Write-Host "Instalado: $binary"
        }
    }
}

if (-not (Test-SqliteWorks $SqliteExe)) {
    Write-Error "Fallo la instalación: sqlite3.exe no funciona en $SqliteExe"
    exit 1
}

Write-Host ""
Write-Host "Version:"
& $SqliteExe --version
if ($LASTEXITCODE -ne 0) { exit 1 }

if (Test-Path $DbPath) {
    Write-Host ""
    Write-Host "PRAGMA integrity_check:"
    & $SqliteExe -readonly $DbPath "PRAGMA integrity_check;"
    if ($LASTEXITCODE -ne 0) { exit 1 }
} else {
    Write-Host ""
    Write-Host "agents.db no existe; se omite integrity_check."
}

if (Test-Path $ZipPath) {
    Remove-Item $ZipPath -Force
    Write-Host "Limpiado: $ZipPath"
}
if (Test-Path $ExtractDir) {
    Remove-Item $ExtractDir -Recurse -Force
    Write-Host "Limpiado: $ExtractDir"
}

Write-Host ""
Write-Host "INSTALL SQLITE: PASS"
exit 0
