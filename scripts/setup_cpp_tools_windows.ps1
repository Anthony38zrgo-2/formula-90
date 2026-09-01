[CmdletBinding()]
param(
    [string]$Version = '3.30.5',
    [switch]$Faust
)
$ErrorActionPreference = 'Stop'; $root = Split-Path -Parent $PSScriptRoot; Set-Location $root

if ($Faust) {
    # --- Fase 5: provision Faust (offline, pinned, checksum-guarded) ---
    $faustVersion = '2.79.3'
    $expectedSha = 'DB7E144466F3B30DE419C8C402CEBA2C3CCBA8493C39141C5DF484387D2D0424'
    $faustDir = Join-Path $root '.tools\faust'
    $faustExe = Join-Path $faustDir 'bin\faust.exe'
    if (Test-Path $faustExe) {
        $v = & $faustExe --version 2>&1 | Out-String
        if ($v -match "FAUST Version $faustVersion") {
            Write-Host "Faust $faustVersion ya instalado:" -ForegroundColor Green
            & $faustExe --version
            exit 0
        }
    }
    $downloads = Join-Path $root '.tools\downloads'
    New-Item -ItemType Directory -Force -Path $downloads | Out-Null
    $installer = Join-Path $downloads "Faust-$faustVersion-win64.exe"
    $uri = "https://github.com/grame-cncm/faust/releases/download/$faustVersion/Faust-$faustVersion-win64.exe"
    if (-not (Test-Path $installer)) {
        Write-Host "Descargando Faust $faustVersion portable..." -ForegroundColor Cyan
        Invoke-WebRequest -Uri $uri -OutFile $installer
    }
    $hash = (Get-FileHash -Algorithm SHA256 $installer).Hash
    if ($hash -ne $expectedSha) {
        throw "FAUST CHECKSUM MISMATCH: expected $expectedSha, got $hash. Abortando (posible binario corrupto/manipulado)."
    }
    Write-Host "Checksum Faust OK ($hash)" -ForegroundColor Green
    # Silent NSIS install into a temp dir, then copy what the build needs.
    $tmp = Join-Path $root '.tools\faust_install_tmp'
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $tmp | Out-Null
    Write-Host "Instalando Faust en $tmp ..." -ForegroundColor Cyan
    & $installer /S /D=$tmp | Out-Null
    Start-Sleep -Seconds 2
    Remove-Item -Recurse -Force $faustDir -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $faustDir | Out-Null
    foreach ($sub in @('bin', 'include', 'share')) {
        if (Test-Path (Join-Path $tmp $sub)) {
            Copy-Item (Join-Path $tmp $sub) (Join-Path $faustDir $sub) -Recurse -Force
        }
    }
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
    if (-not (Test-Path $faustExe)) {
        throw "Instalacion de Faust fallo: $faustExe no encontrado."
    }
    Write-Host "Faust instalado en $faustDir" -ForegroundColor Green
    & $faustExe --version
    exit 0
}

# --- Default: provision CMake (unchanged) ---
$cmakeDir = Join-Path $root '.tools\cmake'
$cmakeExe = Join-Path $cmakeDir 'bin\cmake.exe'
if (Test-Path $cmakeExe) {
    Write-Host "CMake ya instalado:" -ForegroundColor Green
    & $cmakeExe --version
    exit 0
}
$downloads = Join-Path $root '.tools\downloads'
New-Item -ItemType Directory -Force -Path $downloads | Out-Null
$zip = Join-Path $downloads "cmake-$Version-windows-x86_64.zip"
$uri = "https://github.com/Kitware/CMake/releases/download/v$Version/cmake-$Version-windows-x86_64.zip"
if (-not (Test-Path $zip)) {
    Write-Host "Descargando CMake $Version portable..." -ForegroundColor Cyan
    Invoke-WebRequest -Uri $uri -OutFile $zip
}
Write-Host "Extrayendo..." -ForegroundColor Cyan
Expand-Archive -Path $zip -DestinationPath $downloads -Force
Move-Item -Force (Join-Path $downloads "cmake-$Version-windows-x86_64") $cmakeDir
Write-Host "CMake instalado en $cmakeDir" -ForegroundColor Green
& $cmakeExe --version
