<#
.SYNOPSIS
    Ejecuta directamente el baseline limpio de GEVP en Formula-90.

.DESCRIPTION
    - Localiza la raíz del repositorio y game/project.godot.
    - Localiza Godot portable/global.
    - Prefiere Godot *_console.exe para mostrar errores en PowerShell.
    - Ejecuta explícitamente:
        --path <game> --scene res://scenes/tracks/test_field/gevp_baseline.tscn
    - Guarda un log en game/logs/gevp_baseline.log.

.EXAMPLE
    .\scripts\run_gevp_baseline.ps1

.EXAMPLE
    .\scripts\run_gevp_baseline.ps1 -GodotPath "D:\Godot\Godot_v4.7.1-stable_win64.exe"

.EXAMPLE
    .\scripts\run_gevp_baseline.ps1 -ClearCache
#>

[CmdletBinding()]
param(
    [string]$GodotPath = "",
    [switch]$ClearCache
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$ExpectedBranch = "refactor/gevp-clean-baseline"
$SceneRes = "res://scenes/tracks/test_field/gevp_baseline.tscn"

function Find-ProjectRoot {
    param([string]$StartDir)

    $cursor = (Resolve-Path $StartDir).Path

    for ($i = 0; $i -lt 6; $i++) {
        # Case A: this directory is game/
        if (Test-Path (Join-Path $cursor "project.godot") -PathType Leaf) {
            return @{
                RepoRoot = Split-Path -Parent $cursor
                GameDir  = $cursor
            }
        }

        # Case B: this directory is repo root
        $gameProject = Join-Path $cursor "game\project.godot"
        if (Test-Path $gameProject -PathType Leaf) {
            return @{
                RepoRoot = $cursor
                GameDir  = Join-Path $cursor "game"
            }
        }

        $parent = Split-Path -Parent $cursor
        if (-not $parent -or $parent -eq $cursor) {
            break
        }
        $cursor = $parent
    }

    throw "No se encontro game\project.godot partiendo desde: $StartDir"
}

function Find-Godot {
    param(
        [string]$ExplicitPath,
        [string]$RepoRoot
    )

    $regularExe = $null

    if ($ExplicitPath) {
        if (-not (Test-Path $ExplicitPath -PathType Leaf)) {
            throw "GodotPath no existe: $ExplicitPath"
        }
        $regularExe = (Resolve-Path $ExplicitPath).Path
    }
    elseif ($env:GODOT_BIN -and (Test-Path $env:GODOT_BIN -PathType Leaf)) {
        $regularExe = (Resolve-Path $env:GODOT_BIN).Path
    }
    else {
        foreach ($cmdName in @("godot", "godot4")) {
            $cmd = Get-Command $cmdName -ErrorAction SilentlyContinue
            if ($cmd) {
                $regularExe = $cmd.Source
                break
            }
        }
    }

    if (-not $regularExe) {
        $candidateDirs = @(
            (Join-Path $RepoRoot ".tools\godot"),
            (Join-Path $RepoRoot "tools\godot"),
            (Join-Path $RepoRoot "godot"),
            (Join-Path $env:LOCALAPPDATA "Programs\Godot"),
            (Join-Path $env:USERPROFILE "Downloads"),
            "C:\Godot",
            "C:\Tools\Godot"
        ) | Where-Object { $_ -and (Test-Path $_) }

        foreach ($dir in $candidateDirs) {
            $found = Get-ChildItem `
                -Path $dir `
                -Filter "Godot*.exe" `
                -File `
                -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -notmatch "_console\.exe$" } |
                Sort-Object LastWriteTime -Descending |
                Select-Object -First 1

            if ($found) {
                $regularExe = $found.FullName
                break
            }
        }
    }

    if (-not $regularExe) {
        throw @"
No se encontro Godot.

Prueba una de estas opciones:

  `$env:GODOT_BIN = "C:\ruta\Godot.exe"
  .\scripts\run_gevp_baseline.ps1

o:

  .\scripts\run_gevp_baseline.ps1 -GodotPath "C:\ruta\Godot.exe"
"@
    }

    # Prefer the console build when it exists. It exposes Godot errors
    # directly in the current PowerShell terminal.
    if ($regularExe -notmatch "_console\.exe$") {
        $consoleExe = [System.IO.Path]::Combine(
            [System.IO.Path]::GetDirectoryName($regularExe),
            ([System.IO.Path]::GetFileNameWithoutExtension($regularExe) + "_console.exe")
        )

        if (Test-Path $consoleExe -PathType Leaf) {
            return $consoleExe
        }
    }

    return $regularExe
}

# -------------------------------------------------------------------
# Resolve paths
# -------------------------------------------------------------------

$ScriptDir = if ($PSScriptRoot) {
    $PSScriptRoot
} else {
    Split-Path -Parent $MyInvocation.MyCommand.Path
}

$project = Find-ProjectRoot -StartDir $ScriptDir
$RepoRoot = $project.RepoRoot
$GameDir = $project.GameDir

$ProjectFile = Join-Path $GameDir "project.godot"
$SceneFile = Join-Path $GameDir "scenes\tracks\test_field\gevp_baseline.tscn"
$LogDir = Join-Path $GameDir "logs"
$LogFile = Join-Path $LogDir "gevp_baseline.log"

if (-not (Test-Path $ProjectFile -PathType Leaf)) {
    throw "project.godot no existe: $ProjectFile"
}

if (-not (Test-Path $SceneFile -PathType Leaf)) {
    throw "La escena baseline no existe: $SceneFile"
}

if (-not (Test-Path $LogDir)) {
    New-Item -ItemType Directory -Path $LogDir | Out-Null
}

# -------------------------------------------------------------------
# Branch check
# -------------------------------------------------------------------

$currentBranch = $null
try {
    $currentBranch = (& git -C $RepoRoot branch --show-current 2>$null).Trim()
} catch {
    $currentBranch = $null
}

if ($currentBranch -and $currentBranch -ne $ExpectedBranch) {
    Write-Warning "Estas en '$currentBranch'. El baseline fue creado para '$ExpectedBranch'."
}

# -------------------------------------------------------------------
# Optional cache clear
# -------------------------------------------------------------------

if ($ClearCache) {
    $GodotCache = Join-Path $GameDir ".godot"
    if (Test-Path $GodotCache) {
        Write-Host "Eliminando cache: $GodotCache" -ForegroundColor Yellow
        Remove-Item $GodotCache -Recurse -Force
    }
}

# -------------------------------------------------------------------
# Godot
# -------------------------------------------------------------------

$GodotExe = Find-Godot -ExplicitPath $GodotPath -RepoRoot $RepoRoot

Write-Host ""
Write-Host "Formula-90 / GEVP Clean Baseline" -ForegroundColor Cyan
Write-Host "================================" -ForegroundColor DarkGray
Write-Host "Branch : $currentBranch"
Write-Host "Repo   : $RepoRoot"
Write-Host "Game   : $GameDir"
Write-Host "Scene  : $SceneRes"
Write-Host "Godot  : $GodotExe"
Write-Host "Log    : $LogFile"

try {
    $version = (& $GodotExe --version 2>&1 | Select-Object -First 1)
    if ($version) {
        Write-Host "Version: $version"
    }
} catch {
    Write-Warning "No se pudo obtener la version de Godot: $($_.Exception.Message)"
}

Write-Host ""
Write-Host "Ejecutando escena GEVP baseline..." -ForegroundColor Green
Write-Host "Ctrl+C para cancelar desde esta consola." -ForegroundColor DarkGray
Write-Host ""

# Important:
# --scene makes the target explicit.
# res:// avoids dependence on PowerShell's current working directory.
# Push-Location additionally guarantees project-relative filesystem behavior.

Push-Location $GameDir
try {
    & $GodotExe `
        --path $GameDir `
        --scene $SceneRes `
        --log-file $LogFile

    $exitCode = $LASTEXITCODE
}
finally {
    Pop-Location
}

if ($null -eq $exitCode) {
    $exitCode = 0
}

if ($exitCode -ne 0) {
    Write-Host ""
    Write-Host "Godot termino con codigo $exitCode." -ForegroundColor Red
    Write-Host "Ultimas lineas del log:" -ForegroundColor Yellow

    if (Test-Path $LogFile) {
        Get-Content $LogFile -Tail 80
    }

    exit $exitCode
}

Write-Host ""
Write-Host "Godot termino normalmente." -ForegroundColor Green
exit 0
