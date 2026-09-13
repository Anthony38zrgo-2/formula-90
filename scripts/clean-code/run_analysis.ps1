[CmdletBinding()]
param(
    [ValidateSet('clippy', 'audit', 'deny', 'geiger', 'crap', 'clang-tidy', 'cppcheck', 'semgrep')]
    [string[]]$Tools = @('clippy', 'audit', 'deny', 'geiger', 'crap', 'clang-tidy', 'cppcheck', 'semgrep'),
    [string]$RunId = (Get-Date -Format 'yyyyMMdd-HHmmss'),
    [string]$CompareTo,
    [switch]$NoNormalize
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $root
$env:PYTHONUTF8 = '1'
$env:PYTHONIOENCODING = 'utf-8'

$runDir = Join-Path $root "reports\static-analysis\$RunId"
$rawDir = Join-Path $runDir 'raw'
$sarifDir = Join-Path $runDir 'sarif'
New-Item -ItemType Directory -Force -Path $rawDir, $sarifDir | Out-Null

$cargo = (Get-Command cargo -ErrorAction Stop).Source
$python = Join-Path $root '.venv\Scripts\python.exe'
$semgrep = Join-Path $root '.venv\Scripts\semgrep.exe'
$clangTidy = Join-Path $root '.tools\llvm\LLVM\bin\clang-tidy.exe'
$cppcheck = Join-Path $root '.tools\cppcheck\PFiles\Cppcheck\cppcheck.exe'
$workspaceManifest = Join-Path $root 'game\crates\Cargo.toml'

$script:toolStatus = [ordered]@{}
$previousStatusPath = Join-Path $runDir 'tool-status.json'
if (Test-Path $previousStatusPath) {
    try {
        $previous = Get-Content -LiteralPath $previousStatusPath -Raw | ConvertFrom-Json
        foreach ($prop in $previous.PSObject.Properties) { $script:toolStatus[$prop.Name] = $prop.Value }
    } catch {
        Write-Warning "No se pudo leer tool-status.json previo: $_"
    }
}

function Save-Text {
    param([string]$Path, [string]$Content)
    $enc = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, $Content, $enc)
}

function Invoke-NativeCapture {
    param(
        [string]$FilePath,
        [string[]]$Arguments,
        [string]$WorkingDirectory
    )
    $prev = Get-Location
    $stdoutFile = Join-Path $env:TEMP ("f90-out-" + [guid]::NewGuid().ToString('N'))
    $stderrFile = Join-Path $env:TEMP ("f90-err-" + [guid]::NewGuid().ToString('N'))
    if ($WorkingDirectory) { Set-Location $WorkingDirectory }
    try {
        $p = Start-Process -FilePath $FilePath -ArgumentList $Arguments -NoNewWindow -Wait -PassThru `
            -RedirectStandardOutput $stdoutFile -RedirectStandardError $stderrFile
        $result = @{
            ExitCode = $p.ExitCode
            StdOut   = [System.IO.File]::ReadAllText($stdoutFile)
            StdErr   = [System.IO.File]::ReadAllText($stderrFile)
        }
    } finally {
        if ($WorkingDirectory) { Set-Location $prev }
        Remove-Item -LiteralPath $stdoutFile, $stderrFile -Force -ErrorAction SilentlyContinue
    }
    return $result
}

function Invoke-Analyzer {
    param([string]$Name, [scriptblock]$Action)
    Write-Host "==> $Name" -ForegroundColor Cyan
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        & $Action
        $sw.Stop()
        $script:toolStatus[$Name] = [ordered]@{
            status     = 'ok'
            seconds    = [math]::Round($sw.Elapsed.TotalSeconds, 1)
            command    = $script:currentCommand
            sarif      = $script:currentSarif
            raw        = $script:currentRaw
        }
    } catch {
        $sw.Stop()
        $script:toolStatus[$Name] = [ordered]@{
            status  = 'failed'
            seconds = [math]::Round($sw.Elapsed.TotalSeconds, 1)
            command = $script:currentCommand
            error   = "$_"
        }
        Write-Warning "[$Name] $_"
    }
    $script:currentCommand = $null
    $script:currentSarif = $null
    $script:currentRaw = $null
}

function Get-LeadingJson {
    param([string]$Preferred, [string]$Fallback)
    $text = if ($Preferred) { $Preferred.TrimStart([char]0xFEFF).TrimStart() } else { '' }
    if ($text.StartsWith('{')) { return $text }
    $text = if ($Fallback) { $Fallback.TrimStart([char]0xFEFF).TrimStart() } else { '' }
    if ($text.StartsWith('{')) { return $text }
    return $null
}

# ---------------------------------------------------------------- clippy
function Invoke-Clippy {
    $jsonPath = Join-Path $rawDir 'clippy.cargo.json'
    $clippyTargetDir = Join-Path $root '.tools\analysis-target\clippy'
    New-Item -ItemType Directory -Force -Path $clippyTargetDir | Out-Null
    # Clippy reutiliza diagnosticos guardados por builds previos (rustc o
    # coverage) y las unidades "frescas" se replayan de forma parcial, lo que
    # hace variar los lints entre corridas. Para garantizar determinismo:
    # target-dir dedicado + clean -p + CARGO_INCREMENTAL=0 y clippy por
    # paquete (una invocacion --workspace mezcla unidades primarias y de
    # dependencia del mismo crate).
    $metaResult = Invoke-NativeCapture -FilePath $cargo -Arguments @('metadata', '--manifest-path', $workspaceManifest, '--format-version', '1', '--no-deps')
    if ($metaResult.ExitCode -ne 0) { throw "cargo metadata fallo ($($metaResult.ExitCode)): $($metaResult.StdErr)" }
    $meta = $metaResult.StdOut | ConvertFrom-Json
    $packages = @($meta.packages | Sort-Object { $_.name })
    $previousIncremental = $env:CARGO_INCREMENTAL
    $env:CARGO_INCREMENTAL = '0'
    $allOutput = New-Object System.Collections.Generic.List[string]
    $failures = @()
    $usedLocked = $true
    try {
        foreach ($pkg in $packages) {
            $cleanArgs = @('clean', '--manifest-path', $workspaceManifest, '--target-dir', $clippyTargetDir, '-p', $pkg.name)
            Invoke-NativeCapture -FilePath $cargo -Arguments $cleanArgs | Out-Null
            $clippyArgs = @('clippy', '--manifest-path', $pkg.manifest_path, '--all-targets', '--no-deps', '--locked', '--target-dir', $clippyTargetDir, '--message-format=json')
            $r = Invoke-NativeCapture -FilePath $cargo -Arguments $clippyArgs
            if ($r.ExitCode -ne 0 -and $r.StdErr -match 'lock file') {
                $usedLocked = $false
                $clippyArgs = @('clippy', '--manifest-path', $pkg.manifest_path, '--all-targets', '--no-deps', '--target-dir', $clippyTargetDir, '--message-format=json')
                $r = Invoke-NativeCapture -FilePath $cargo -Arguments $clippyArgs
            }
            Save-Text -Path (Join-Path $rawDir "clippy-$($pkg.name).stderr.txt") -Content $r.StdErr
            if ([string]::IsNullOrWhiteSpace($r.StdOut)) {
                $failures += $pkg.name
                continue
            }
            foreach ($line in ($r.StdOut -split "`r?`n")) { if ($line.Trim()) { $allOutput.Add($line) } }
        }
    } finally {
        if ($null -eq $previousIncremental) {
            Remove-Item Env:CARGO_INCREMENTAL -ErrorAction SilentlyContinue
        } else {
            $env:CARGO_INCREMENTAL = $previousIncremental
        }
    }
    if ($allOutput.Count -eq 0) { throw "cargo clippy no produjo salida JSON (paquetes con fallo: $($failures -join ', '))" }
    if ($failures.Count -gt 0) { Write-Warning "clippy fallo en paquetes: $($failures -join ', ')" }
    $script:currentCommand = "cargo clippy <pkg> --all-targets --no-deps --locked --target-dir .tools/analysis-target/clippy --message-format=json (clean -p por paquete; CARGO_INCREMENTAL=0)"
    Save-Text -Path $jsonPath -Content ($allOutput -join "`n")
    $script:currentRaw = $jsonPath
    $sarifPath = Join-Path $sarifDir 'clippy.sarif'
    $c = Invoke-NativeCapture -FilePath 'clippy-sarif.exe' -Arguments @('-i', $jsonPath, '-o', $sarifPath)
    if ($c.ExitCode -ne 0) { throw "clippy-sarif fallo ($($c.ExitCode)): $($c.StdErr)" }
    $script:currentSarif = $sarifPath
    if (-not $usedLocked) { Write-Warning 'cargo clippy se ejecuto sin --locked (lock desactualizado)' }
}

# ---------------------------------------------------------------- cargo-audit
function Invoke-Audit {
    $args = @('audit', '--format', 'sarif', '--quiet')
    $script:currentCommand = "$cargo $($args -join ' ')"
    $r = Invoke-NativeCapture -FilePath $cargo -Arguments $args -WorkingDirectory (Split-Path -Parent $workspaceManifest)
    Save-Text -Path (Join-Path $rawDir 'audit.stderr.txt') -Content $r.StdErr
    $json = Get-LeadingJson -Preferred $r.StdOut -Fallback $r.StdErr
    if (-not $json) { throw "cargo audit no produjo SARIF (exit $($r.ExitCode)): $($r.StdErr)" }
    $sarifPath = Join-Path $sarifDir 'audit.sarif'
    Save-Text -Path $sarifPath -Content $json
    $script:currentSarif = $sarifPath
}

# ---------------------------------------------------------------- cargo-deny
function Invoke-Deny {
    $config = Join-Path $PSScriptRoot 'deny.toml'
    $args = @('deny', '--manifest-path', $workspaceManifest, '--config', $config, '--format', 'sarif', 'check', 'advisories', 'bans', 'licenses', 'sources')
    $script:currentCommand = "$cargo $($args -join ' ')"
    $r = Invoke-NativeCapture -FilePath $cargo -Arguments $args
    Save-Text -Path (Join-Path $rawDir 'deny.stderr.txt') -Content $r.StdErr
    $json = Get-LeadingJson -Preferred $r.StdOut -Fallback $r.StdErr
    if (-not $json) { throw "cargo deny no produjo SARIF (exit $($r.ExitCode)): $($r.StdErr)" }
    $sarifPath = Join-Path $sarifDir 'deny.sarif'
    Save-Text -Path $sarifPath -Content $json
    $script:currentSarif = $sarifPath
}

# ---------------------------------------------------------------- cargo-geiger
function Invoke-Geiger {
    $metaResult = Invoke-NativeCapture -FilePath $cargo -Arguments @('metadata', '--manifest-path', $workspaceManifest, '--format-version', '1', '--no-deps')
    if ($metaResult.ExitCode -ne 0) { throw "cargo metadata fallo ($($metaResult.ExitCode)): $($metaResult.StdErr)" }
    $meta = $metaResult.StdOut | ConvertFrom-Json
    $manifests = @($meta.packages | ForEach-Object { $_.manifest_path } | Sort-Object -Unique)
    if ($manifests.Count -eq 0) { throw 'cargo metadata no devolvio paquetes del workspace' }
    $allPackages = @()
    $withoutMetrics = @()
    $notScanned = @()
    $failures = @()
    foreach ($manifest in $manifests) {
        $r = Invoke-NativeCapture -FilePath $cargo -Arguments @('geiger', '--manifest-path', $manifest, '--output-format', 'Json', '--include-tests')
        if ($r.ExitCode -ne 0 -or [string]::IsNullOrWhiteSpace($r.StdOut)) {
            $failures += "$manifest (exit $($r.ExitCode))"
            continue
        }
        try {
            $doc = $r.StdOut | ConvertFrom-Json
            $allPackages += @($doc.packages)
            $withoutMetrics += @($doc.packages_without_metrics)
            $notScanned += @($doc.used_but_not_scanned_files)
        } catch {
            $failures += "$manifest (JSON invalido)"
        }
    }
    if ($failures.Count -eq $manifests.Count) { throw "cargo geiger fallo en todos los crates: $($failures -join '; ')" }
    if ($failures.Count -gt 0) { Write-Warning "cargo geiger fallo en: $($failures -join '; ')" }
    $merged = [ordered]@{
        packages                  = @($allPackages | Sort-Object { $_.package.id.name })
        packages_without_metrics  = @($withoutMetrics | Sort-Object -Unique)
        used_but_not_scanned_files = @($notScanned | Sort-Object -Unique)
    }
    $jsonPath = Join-Path $rawDir 'geiger.json'
    Save-Text -Path $jsonPath -Content ($merged | ConvertTo-Json -Depth 12)
    $script:currentCommand = "$cargo geiger --manifest-path <crate> --output-format Json --include-tests"
    $script:currentRaw = $jsonPath
}

# ---------------------------------------------------------------- cargo-crap (via llvm-cov)
function Invoke-Crap {
    $lcovTests = Join-Path $rawDir 'lcov-tests.info'
    $lcovLib = Join-Path $rawDir 'lcov-lib.info'
    $lcov = Join-Path $rawDir 'lcov.info'
    # CLEAN-12: `--tests` (lib + bins + integracion) carga dos instanciaciones del
    # mismo modulo en crates ["rlib","cdylib"] (el rlib no-test que enlazan los
    # bins colapsa a 0 los contadores de las funciones `#[no_mangle]`). La pasada
    # `--lib` produce la instanciacion de test con hits reales del ABI. merge_lcov.py
    # combina ambas: cobertura de integracion de la pasada --tests + counts ABI de
    # la pasada --lib + dedup de la doble instanciacion.
    $covTests = Invoke-NativeCapture -FilePath $cargo -Arguments @('llvm-cov', '--manifest-path', $workspaceManifest, '--workspace', '--tests', '--lcov', '--output-path', $lcovTests, '--ignore-run-fail') -WorkingDirectory (Split-Path -Parent $workspaceManifest)
    Save-Text -Path (Join-Path $rawDir 'llvm-cov-tests.stderr.txt') -Content $covTests.StdErr
    if ($covTests.ExitCode -ne 0 -or -not (Test-Path $lcovTests)) { throw "cargo llvm-cov --tests fallo ($($covTests.ExitCode))" }
    $covLib = Invoke-NativeCapture -FilePath $cargo -Arguments @('llvm-cov', '--manifest-path', $workspaceManifest, '--workspace', '--lib', '--lcov', '--output-path', $lcovLib, '--ignore-run-fail') -WorkingDirectory (Split-Path -Parent $workspaceManifest)
    Save-Text -Path (Join-Path $rawDir 'llvm-cov-lib.stderr.txt') -Content $covLib.StdErr
    if ($covLib.ExitCode -ne 0 -or -not (Test-Path $lcovLib)) { throw "cargo llvm-cov --lib fallo ($($covLib.ExitCode))" }
    $merge = & $python (Join-Path $PSScriptRoot 'merge_lcov.py') --lcov $lcovTests --abi $lcovLib --output $lcov
    if ($LASTEXITCODE -ne 0) { throw "merge_lcov.py fallo ($LASTEXITCODE): $merge" }
    $script:currentCommand = "cargo llvm-cov --workspace --tests/--lib + merge_lcov.py; cargo crap --workspace --lcov lcov.info --format sarif"
    $crapArgs = @('crap', '--workspace', '--lcov', $lcov, '--format', 'sarif')
    $r = Invoke-NativeCapture -FilePath $cargo -Arguments $crapArgs -WorkingDirectory (Split-Path -Parent $workspaceManifest)
    Save-Text -Path (Join-Path $rawDir 'crap.stderr.txt') -Content $r.StdErr
    $json = Get-LeadingJson -Preferred $r.StdOut -Fallback $r.StdErr
    if (-not $json) { throw "cargo crap no produjo SARIF (exit $($r.ExitCode)): $($r.StdErr)" }
    $sarifPath = Join-Path $sarifDir 'cargo-crap.sarif'
    Save-Text -Path $sarifPath -Content $json
    $script:currentSarif = $sarifPath
    $script:currentRaw = $lcov
}

# ---------------------------------------------------------------- clang-tidy
function Invoke-ClangTidy {
    $checkSet = '-*,bugprone-*,clang-analyzer-*,performance-*,portability-*,readability-*,-readability-magic-numbers,-readability-identifier-length'
    $headerFilter = '^(?!.*generated).*(native|vehicle-audio-dsp).*$'
    $common = @('--checks', $checkSet, '--header-filter', $headerFilter, '--quiet')
    $script:currentCommand = "$clangTidy --checks=$checkSet <native + vehicle-audio-dsp>"
    $allOutput = New-Object System.Collections.Generic.List[string]
    $totalFiles = 0

    $rootDb = Join-Path $root 'compile_commands.json'
    if (Test-Path $rootDb) {
        $db = Get-Content -LiteralPath $rootDb -Raw | ConvertFrom-Json
        $files = @($db | Where-Object {
                $_.file -match '(^|[\\/])native[\\/]src[\\/].*\.cpp$'
            } | ForEach-Object { $_.file } | Sort-Object -Unique)
        if ($files.Count -eq 0) {
            Write-Warning 'clang-tidy: el compile DB raiz no contiene archivos de native/src'
        } else {
            $totalFiles += $files.Count
            $r = Invoke-NativeCapture -FilePath $clangTidy -Arguments (@('-p', $root) + $common + $files)
            Save-Text -Path (Join-Path $rawDir 'clang-tidy-native.stderr.txt') -Content $r.StdErr
            foreach ($line in ($r.StdOut -split "`r?`n")) { if ($line) { $allOutput.Add($line) } }
        }
    } else {
        Write-Warning 'compile_commands.json raiz ausente; ejecute scripts/build_windows.ps1 -CompileCommands'
    }

    $dspRoot = Join-Path $root 'game\native\vehicle-audio-dsp'
    $dspFiles = @()
    foreach ($sub in @('src', 'tests')) {
        $dir = Join-Path $dspRoot $sub
        if (Test-Path $dir) {
            $dspFiles += @(Get-ChildItem -Path $dir -Filter *.cpp -File | ForEach-Object { $_.FullName })
        }
    }
    if ($dspFiles.Count -gt 0) {
        $totalFiles += $dspFiles.Count
        $extra = @('--', '-std=c++20',
            "-I$(Join-Path $dspRoot 'include')",
            "-I$(Join-Path $dspRoot 'src')",
            "-I$(Join-Path $dspRoot 'generated')",
            "-I$(Join-Path $root '.tools\faust\include')")
        $r = Invoke-NativeCapture -FilePath $clangTidy -Arguments ($common + $dspFiles + $extra)
        Save-Text -Path (Join-Path $rawDir 'clang-tidy-dsp.stderr.txt') -Content $r.StdErr
        foreach ($line in ($r.StdOut -split "`r?`n")) { if ($line) { $allOutput.Add($line) } }
    }

    if ($totalFiles -eq 0) { throw 'clang-tidy no recibio archivos propios' }
    $textPath = Join-Path $rawDir 'clang-tidy.txt'
    Save-Text -Path $textPath -Content ($allOutput -join "`n")
    $script:currentRaw = $textPath
    $sarifPath = Join-Path $sarifDir 'clang-tidy.sarif'
    $c = Invoke-NativeCapture -FilePath 'clang-tidy-sarif.exe' -Arguments @('-i', $textPath, '-o', $sarifPath)
    if ($c.ExitCode -ne 0) { throw "clang-tidy-sarif fallo ($($c.ExitCode)): $($c.StdErr)" }
    $script:currentSarif = $sarifPath
}

# ---------------------------------------------------------------- cppcheck
function Invoke-Cppcheck {
    $out = Join-Path $sarifDir 'cppcheck.sarif'
    $args = @(
        '--output-format=sarif', "--output-file=$out",
        '--enable=warning,style,performance,portability', '--inconclusive',
        '--std=c++20', '--language=c++', '--platform=win64', '--quiet',
        '--suppress=missingIncludeSystem', '--suppress=unknownMacro',
        "-I$(Join-Path $root 'native\include')",
        "-I$(Join-Path $root 'game\native\vehicle-audio-dsp\include')",
        (Join-Path $root 'native\src'),
        (Join-Path $root 'game\native\vehicle-audio-dsp\src'),
        (Join-Path $root 'game\native\vehicle-audio-dsp\tests')
    )
    $script:currentCommand = "cppcheck --output-format=sarif --enable=warning,style,performance,portability --suppress=missingIncludeSystem --suppress=unknownMacro <paths> (sin include paths de godot-cpp: coste desmedido)"
    $r = Invoke-NativeCapture -FilePath $cppcheck -Arguments $args
    Save-Text -Path (Join-Path $rawDir 'cppcheck.stderr.txt') -Content $r.StdErr
    if (-not (Test-Path $out)) { throw "cppcheck no genero SARIF (exit $($r.ExitCode)): $($r.StdErr)" }
    $script:currentSarif = $out
}

# ---------------------------------------------------------------- semgrep
function Invoke-Semgrep {
    $out = Join-Path $sarifDir 'semgrep.sarif'
    $localRules = Join-Path $PSScriptRoot 'semgrep\rules.yml'
    $excludes = @('--exclude', 'third_party', '--exclude', 'game/addons', '--exclude', '.tools', '--exclude', '.venv', '--exclude', 'reports', '--exclude', '.tmp', '--exclude', 'build')
    $targets = @('game/crates', 'native', 'game/native/vehicle-audio-dsp')
    $script:currentCommand = "semgrep scan --sarif --jobs 4 --config p/default --config semgrep/rules.yml <targets>"
    $r = Invoke-NativeCapture -FilePath $semgrep -Arguments (@('scan', '--sarif', '--output', $out, '--metrics=off', '--quiet', '--jobs', '4', '--config', 'p/default', '--config', $localRules) + $excludes + $targets)
    if ($r.ExitCode -ne 0 -or -not (Test-Path $out)) {
        Write-Warning "semgrep con p/default fallo (exit $($r.ExitCode)); reintentando solo reglas locales"
        Save-Text -Path (Join-Path $rawDir 'semgrep-first.stderr.txt') -Content $r.StdErr
        $r = Invoke-NativeCapture -FilePath $semgrep -Arguments (@('scan', '--sarif', '--output', $out, '--metrics=off', '--quiet', '--jobs', '4', '--config', $localRules) + $excludes + $targets)
    }
    Save-Text -Path (Join-Path $rawDir 'semgrep.stderr.txt') -Content $r.StdErr
    if (-not (Test-Path $out)) { throw "semgrep no genero SARIF (exit $($r.ExitCode)): $($r.StdErr)" }
    $script:currentSarif = $out
}

# ---------------------------------------------------------------- versions
function Get-ToolVersions {
    $versions = [ordered]@{}
    $probes = @(
        @{ Name = 'clippy'; File = $cargo; Args = @('clippy', '--version') },
        @{ Name = 'cargo-audit'; File = $cargo; Args = @('audit', '--version') },
        @{ Name = 'cargo-deny'; File = $cargo; Args = @('deny', '--version') },
        @{ Name = 'cargo-geiger'; File = $cargo; Args = @('geiger', '--version') },
        @{ Name = 'cargo-crap'; File = $cargo; Args = @('crap', '--version') },
        @{ Name = 'cargo-llvm-cov'; File = $cargo; Args = @('llvm-cov', '--version') },
        @{ Name = 'clippy-sarif'; File = 'clippy-sarif.exe'; Args = @('--version') },
        @{ Name = 'clang-tidy-sarif'; File = 'clang-tidy-sarif.exe'; Args = @('--version') },
        @{ Name = 'clang-tidy'; File = $clangTidy; Args = @('--version') },
        @{ Name = 'cppcheck'; File = $cppcheck; Args = @('--version') },
        @{ Name = 'semgrep'; File = $semgrep; Args = @('--version') }
    )
    foreach ($probe in $probes) {
        try {
            $r = Invoke-NativeCapture -FilePath $probe.File -Arguments $probe.Args
            $line = (($r.StdOut + "`n" + $r.StdErr) -split "`r?`n" | Where-Object { $_.Trim() } | Select-Object -First 1)
            $versions[$probe.Name] = $line.Trim()
        } catch {
            $versions[$probe.Name] = 'unavailable'
        }
    }
    return $versions
}

# ---------------------------------------------------------------- dispatch
$actions = @{
    'clippy'     = { Invoke-Clippy }
    'audit'      = { Invoke-Audit }
    'deny'       = { Invoke-Deny }
    'geiger'     = { Invoke-Geiger }
    'crap'       = { Invoke-Crap }
    'clang-tidy' = { Invoke-ClangTidy }
    'cppcheck'   = { Invoke-Cppcheck }
    'semgrep'    = { Invoke-Semgrep }
}

$head = (& git rev-parse HEAD).Trim()
$branch = (& git branch --show-current).Trim()
Write-Host "Run $RunId | HEAD $head | branch $branch" -ForegroundColor Green

foreach ($tool in $Tools) {
    Invoke-Analyzer -Name $tool -Action $actions[$tool]
}

$versions = Get-ToolVersions
Save-Text -Path (Join-Path $runDir 'tool-status.json') -Content ($script:toolStatus | ConvertTo-Json -Depth 6)
Save-Text -Path (Join-Path $runDir 'tool-versions.json') -Content ($versions | ConvertTo-Json -Depth 4)

if (-not $NoNormalize) {
    & $python (Join-Path $PSScriptRoot 'normalize_sarif.py') --run-dir $runDir --repository-root $root
    if ($LASTEXITCODE -ne 0) { throw "normalize_sarif.py fallo ($LASTEXITCODE)" }
    if ($CompareTo) {
        & $python (Join-Path $PSScriptRoot 'compare_findings.py') --previous $CompareTo --current (Join-Path $runDir 'static-analysis-summary.json') --output (Join-Path $runDir 'comparison.json')
        if ($LASTEXITCODE -ne 0) { throw "compare_findings.py fallo ($LASTEXITCODE)" }
    }
}

Write-Host "Run dir: $runDir" -ForegroundColor Green
