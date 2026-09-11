[CmdletBinding()]
param(
    [switch]$Force
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$downloads = Join-Path $root '.tools\downloads'
New-Item -ItemType Directory -Force -Path $downloads | Out-Null

function Get-PinnedFile {
    param([string]$Name, [string]$Uri, [string]$Sha256)
    $path = Join-Path $downloads $Name
    if ($Force -or -not (Test-Path $path)) {
        Write-Host "Descargando $Name ..." -ForegroundColor Cyan
        & curl.exe -L --fail --retry 3 -o $path $Uri
        if ($LASTEXITCODE -ne 0) { throw "Descarga fallo: $Uri" }
    }
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash
    if ($hash -ne $Sha256) {
        throw "CHECKSUM MISMATCH en ${Name}: esperado $Sha256, obtenido $hash"
    }
    Write-Host "Checksum OK: $Name" -ForegroundColor Green
    return $path
}

function Expand-PortableMsi {
    param([string]$Msi, [string]$Destination, [string]$ProbeExe)
    if (-not $Force -and (Test-Path $ProbeExe)) {
        Write-Host "Ya extraido: $ProbeExe" -ForegroundColor Green
        return
    }
    if ($Force) { Remove-Item -Recurse -Force $Destination -ErrorAction SilentlyContinue }
    New-Item -ItemType Directory -Force -Path $Destination | Out-Null
    Write-Host "Extrayendo $([System.IO.Path]::GetFileName($Msi)) en $Destination ..." -ForegroundColor Cyan
    $p = Start-Process -FilePath 'msiexec.exe' -ArgumentList @('/a', $Msi, '/qn', "TARGETDIR=$Destination") -Wait -PassThru -NoNewWindow
    if ($p.ExitCode -ne 0) { throw "msiexec /a fallo con codigo $($p.ExitCode) para $Msi" }
    if (-not (Test-Path $ProbeExe)) { throw "Extraccion incompleta: no existe $ProbeExe" }
}

# --- LLVM / clang-tidy (portable, checksum-pinned) ---
$llvmVersion = '23.1.1'
$llvmMsi = Get-PinnedFile -Name "LLVM-$llvmVersion-win64.msi" `
    -Uri "https://github.com/llvm/llvm-project/releases/download/llvmorg-$llvmVersion/LLVM-$llvmVersion-win64.msi" `
    -Sha256 '11AF43BA261A1158090DD6F0DA40D7A34F32DBFB4A76B43240BFFA1EBB140984'
$llvmDir = Join-Path $root '.tools\llvm'
$clangTidy = Join-Path $llvmDir 'LLVM\bin\clang-tidy.exe'
Expand-PortableMsi -Msi $llvmMsi -Destination $llvmDir -ProbeExe $clangTidy

# --- cppcheck (portable, checksum-pinned) ---
$cppcheckVersion = '2.21.0'
$cppcheckMsi = Get-PinnedFile -Name "cppcheck-$cppcheckVersion-x64-Setup.msi" `
    -Uri "https://github.com/cppcheck-opensource/cppcheck/releases/download/$cppcheckVersion/cppcheck-$cppcheckVersion-x64-Setup.msi" `
    -Sha256 'A86EEF1180DC18FDE46B08F4E4B12F2A1EAD8CFC0DD6C294AAC8757748CBBB24'
$cppcheckDir = Join-Path $root '.tools\cppcheck'
$cppcheckExe = Join-Path $cppcheckDir 'PFiles\Cppcheck\cppcheck.exe'
Expand-PortableMsi -Msi $cppcheckMsi -Destination $cppcheckDir -ProbeExe $cppcheckExe

# --- Cargo analysis tools (version-pinned) ---
$rustTools = @(
    @{ Name = 'cargo-audit'; Version = '0.22.2' },
    @{ Name = 'cargo-deny'; Version = '0.20.2' },
    @{ Name = 'cargo-geiger'; Version = '0.13.0' },
    @{ Name = 'cargo-crap'; Version = '0.5.0' },
    @{ Name = 'cargo-llvm-cov'; Version = '0.9.1' },
    @{ Name = 'clippy-sarif'; Version = '0.8.0' },
    @{ Name = 'clang-tidy-sarif'; Version = '0.8.0' }
)
foreach ($tool in $rustTools) {
    $exe = Join-Path $env:USERPROFILE ".cargo\bin\$($tool.Name).exe"
    if (-not $Force -and (Test-Path $exe)) {
        Write-Host "$($tool.Name) ya instalado" -ForegroundColor Green
        continue
    }
    Write-Host "cargo install --locked $($tool.Name)@$($tool.Version) ..." -ForegroundColor Cyan
    & cargo install --locked "$($tool.Name)@$($tool.Version)"
    if ($LASTEXITCODE -ne 0) { throw "cargo install $($tool.Name) fallo ($LASTEXITCODE)" }
}
& rustup component add llvm-tools-preview 2>&1 | Out-Null

# --- semgrep (.venv, version-pinned) ---
$semgrepVersion = '1.177.0'
$venvPython = Join-Path $root '.venv\Scripts\python.exe'
$semgrepExe = Join-Path $root '.venv\Scripts\semgrep.exe'
if ($Force -or -not (Test-Path $semgrepExe)) {
    if (-not (Test-Path $venvPython)) { throw 'Falta .venv. Cree el entorno con scripts/bootstrap_windows.ps1.' }
    & $venvPython -m pip install --disable-pip-version-check "semgrep==$semgrepVersion"
    if ($LASTEXITCODE -ne 0) { throw 'pip install semgrep fallo' }
}

# --- Manifiesto reproducible ---
$manifest = [ordered]@{
    generated_at = (Get-Date -Format o)
    llvm         = $llvmVersion
    clang_tidy   = $clangTidy
    cppcheck     = $cppcheckVersion
    cppcheck_exe = $cppcheckExe
    semgrep      = $semgrepVersion
    semgrep_exe  = $semgrepExe
    rust_tools   = $rustTools
}
$manifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $root '.tools\analysis-tools.json') -Encoding UTF8

Write-Host '--- Versiones instaladas ---' -ForegroundColor Cyan
& $clangTidy --version | Select-Object -First 1
& $cppcheckExe --version
& $semgrepExe --version
foreach ($tool in $rustTools) {
    & (Join-Path $env:USERPROFILE ".cargo\bin\$($tool.Name).exe") --version
}
Write-Host 'Herramientas de analisis provisionadas.' -ForegroundColor Green
