<#
.SYNOPSIS
    Run clean GEVP baseline scene in Godot 4.

.DESCRIPTION
    Resolves Godot executable, validates GDExtension DLL and baseline scene,
    and runs scenes/tracks/test_field/gevp_baseline.tscn.

.PARAMETER GodotPath
    Explicit path to a Godot executable. If omitted, auto-resolves via
    .tools/godot/, GODOT_BIN env var, PATH, or system install.

.EXAMPLE
    .\scripts\run_gevp_baseline.ps1
    .\scripts\run_gevp_baseline.ps1 -GodotPath "C:\Godot\Godot.exe"
#>
[CmdletBinding()]
param(
    [string]$GodotPath
)

$ErrorActionPreference = "Stop"

$ScriptDir = $PSScriptRoot
if (-not $ScriptDir) {
    $ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
}

# Resolve RepoRoot and GameDir cleanly
if (Test-Path (Join-Path $ScriptDir "project.godot")) {
    $GameDir = $ScriptDir
    $RepoRoot = Split-Path -Parent $GameDir
}
elseif (Test-Path (Join-Path (Split-Path -Parent $ScriptDir) "game\project.godot")) {
    $RepoRoot = Split-Path -Parent $ScriptDir
    $GameDir = Join-Path $RepoRoot "game"
}
elseif (Test-Path (Join-Path (Split-Path -Parent $ScriptDir) "project.godot")) {
    $GameDir = Split-Path -Parent $ScriptDir
    $RepoRoot = Split-Path -Parent $GameDir
}
else {
    throw "No se pudo localizar game/project.godot desde: $ScriptDir"
}

$SceneRel = "scenes/tracks/test_field/gevp_baseline.tscn"
$SceneFile = Join-Path $GameDir $SceneRel
$ProjectFile = Join-Path $GameDir "project.godot"
$DllFile = Join-Path $GameDir "addons\formula90s\bin\libformula90s.windows.template_debug.x86_64.dll"

function Resolve-GodotExecutable {
    param([string]$ExplicitPath)

    if ($ExplicitPath) {
        if (Test-Path $ExplicitPath -PathType Leaf) {
            return (Resolve-Path $ExplicitPath).Path
        }
        throw "GodotPath no existe: $ExplicitPath"
    }

    # 1. Environment variable
    if ($env:GODOT_BIN -and (Test-Path $env:GODOT_BIN -PathType Leaf)) {
        return (Resolve-Path $env:GODOT_BIN).Path
    }

    # 2. PATH commands
    foreach ($commandName in @("godot", "godot4")) {
        $cmd = Get-Command $commandName -ErrorAction SilentlyContinue
        if ($cmd) {
            return $cmd.Source
        }
    }

    # 3. Known project & system candidate paths (fast lookup without deep recursive scan)
    $candidates = @(
        (Join-Path $RepoRoot '.tools\godot\Godot_v4.7.1-stable_win64.exe'),
        (Join-Path $RepoRoot '.tools\godot\Godot.exe'),
        (Join-Path $env:LOCALAPPDATA 'Programs\Godot\Godot.exe'),
        (Join-Path $env:ProgramFiles 'Godot\Godot.exe'),
        'C:\Godot\Godot.exe',
        'C:\Tools\Godot.exe'
    )

    foreach ($item in $candidates) {
        if ($item -and (Test-Path $item -PathType Leaf)) {
            return (Resolve-Path $item).Path
        }
    }

    # 4. Fallback search in .tools/godot directory only
    $toolsDir = Join-Path $RepoRoot ".tools\godot"
    if (Test-Path $toolsDir) {
        $found = Get-ChildItem -Path $toolsDir -Filter "Godot*.exe" -File -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -notmatch "_console\.exe$" } |
            Sort-Object LastWriteTime -Descending |
            Select-Object -First 1
        if ($found) {
            return $found.FullName
        }
    }

    throw @"
Godot no fue encontrado.

Opciones:
  1. Ejecuta bootstrap_windows.ps1 para instalar Godot en .tools\godot.
  2. Agrega Godot al PATH o define la variable GODOT_BIN.
  3. Ejecuta este script indicando la ruta explicita:
     .\scripts\run_gevp_baseline.ps1 -GodotPath "C:\ruta\Godot.exe"
"@
}

Write-Host ""
Write-Host "Formula-90 / GEVP Clean Baseline" -ForegroundColor Cyan
Write-Host "--------------------------------" -ForegroundColor DarkGray

if (-not (Test-Path $ProjectFile -PathType Leaf)) {
    throw "No se encontro project.godot en: $ProjectFile"
}

if (-not (Test-Path $SceneFile -PathType Leaf)) {
    throw "No se encontro la escena baseline en: $SceneFile"
}

if (-not (Test-Path $DllFile -PathType Leaf)) {
    Write-Warning "GDExtension DLL no encontrada en: $DllFile. Ejecute build_windows.ps1 si requiere nodos C++."
}

$GodotExe = Resolve-GodotExecutable -ExplicitPath $GodotPath

Write-Host "Repo:   $RepoRoot"
Write-Host "Game:   $GameDir"
Write-Host "Scene:  $SceneRel"
Write-Host "Godot:  $GodotExe"

try {
    $version = & $GodotExe --version 2>$null
    if ($LASTEXITCODE -eq 0 -and $version) {
        Write-Host "Version: $version"
    }
}
catch {
    Write-Warning "No se pudo consultar la version de Godot."
}

Write-Host ""
Write-Host "Iniciando baseline..." -ForegroundColor Green
Write-Host ""

& $GodotExe --path $GameDir $SceneRel

$exitCode = $LASTEXITCODE
if ($null -eq $exitCode) {
    $exitCode = 0
}

if ($exitCode -ne 0) {
    Write-Host ""
    Write-Host "Godot termino con codigo: $exitCode" -ForegroundColor Red
    exit $exitCode
}

exit 0
