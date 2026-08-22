<#
================================================================================
 gate_auditivo.ps1 — Sesion del GATE AUDITIVO HUMANO (integracion audio
 Common + V10_V2, backend common_v10_commands).
================================================================================

Que hace, en orden:
  1. Preflight: Godot, cargo, bancos, DLLs.
  2. Construye SOLO si hace falta (Rust release + GDExtension debug/release,
     comparando timestamps contra las fuentes; usa -ForceBuild para forzar).
  3. Regenera los artefactos A/B deterministas:
       - golden trace  -> diagnostics\golden_trace_v10_v2.json
       - harness A/B   -> diagnostics\ab\*.legacy.wav + *.commands.json
  4. Lanza, EN VENTANA (con audio real), las tres escenas de escucha.
     Conduce con el teclado; CIERRA LA VENTANA cuando hayas escuchado cada una
     (o usa -AutoCloseSeconds N para cerrarlas automaticamente).
  5. Copia las capturas y la telemetria grabada al repo (diagnostics\).
  6. Replay offline de TU telemetria real por ambos backends
     (diagnostics\ab\live_telemetry_live.{legacy.wav,commands.json}).
  7. Resumen final con las rutas y el orden de comparacion recomendado.

Uso (desde D:\Formula90s):
  powershell -ExecutionPolicy Bypass -File .\gate_auditivo.ps1
  # opciones utiles:
  powershell -ExecutionPolicy Bypass -File .\gate_auditivo.ps1 -AutoCloseSeconds 45
  powershell -ExecutionPolicy Bypass -File .\gate_auditivo.ps1 -SkipBuild
  powershell -ExecutionPolicy Bypass -File .\gate_auditivo.ps1 -SkipListen   # solo artefactos, sin escenas

Requisitos: Godot 4.7.1 en .tools\godot, cargo en PATH, banco legacy
v10_vehicle presente y banco v10_v2 (ya estan en el repo).

Checklist y decisiones abiertas: docs\audio\GATE_AUDITIVO_CHECKLIST.md
================================================================================
#>
[CmdletBinding()]
param(
    [int]    $AutoCloseSeconds = 0,     # 0 = espera a que cierres cada ventana
    [switch] $SkipBuild,                # no reconstruir nada
    [switch] $ForceBuild,               # reconstruir aunque no haya cambios
    [switch] $SkipGolden,               # no regenerar el golden trace
    [switch] $SkipListen,               # no lanzar las escenas (solo artefactos)
    [string] $Godot   = "D:\Formula90s\.tools\godot\Godot_v4.7.1-stable_win64_console.exe",
    [string] $Project = "D:\Formula90s\game",
    [string] $Repo    = "D:\Formula90s",
    [string] $Crates  = "D:\Formula90s\game\crates"
)

$ErrorActionPreference = "Stop"
$UserData = Join-Path $env:APPDATA "Godot\app_userdata\FORMULA 90s"
$Diag = Join-Path $Repo "diagnostics"
$Ab   = Join-Path $Diag "ab"

function Write-Step([string]$msg) {
    Write-Host ""
    Write-Host ("=" * 72) -ForegroundColor Cyan
    Write-Host ("  " + $msg) -ForegroundColor Cyan
    Write-Host ("=" * 72) -ForegroundColor Cyan
}

function Write-Ok([string]$msg)   { Write-Host ("  [OK]  " + $msg) -ForegroundColor Green }
function Write-Warn([string]$msg) { Write-Host ("  [!]   " + $msg) -ForegroundColor Yellow }
function Write-Fail([string]$msg) { Write-Host ("  [ERR] " + $msg) -ForegroundColor Red }

function Newest-Mtime {
    param([string[]]$Paths)
    $m = [datetime]::MinValue
    foreach ($p in $Paths) {
        $items = @(Get-Item $p -ErrorAction SilentlyContinue)
        foreach ($it in $items) {
            if ($it.LastWriteTime -gt $m) { $m = $it.LastWriteTime }
        }
    }
    return $m
}

function Invoke-Native {
    param([string]$FilePath, [string[]]$ArgList, [string]$WorkDir)
    $outLog = Join-Path $env:TEMP ("native_" + [guid]::NewGuid().ToString("N") + ".log")
    $p = Start-Process -FilePath $FilePath -ArgumentList $ArgList -WorkingDirectory $WorkDir `
        -RedirectStandardOutput $outLog -RedirectStandardError ($outLog + ".err") `
        -NoNewWindow -Wait -PassThru
    if ($p.ExitCode -ne 0) {
        if (Test-Path $outLog) { Get-Content $outLog -Tail 15 | ForEach-Object { Write-Host "    $_" -ForegroundColor DarkGray } }
        if (Test-Path ($outLog + ".err")) { Get-Content ($outLog + ".err") -Tail 15 | ForEach-Object { Write-Host "    $_" -ForegroundColor DarkGray } }
        throw ("comando fallo (exit " + $p.ExitCode + "): " + $FilePath + " " + ($ArgList -join " "))
    }
    if (Test-Path $outLog) { Get-Content $outLog -Tail 4 | ForEach-Object { Write-Host "    $_" -ForegroundColor DarkGray } }
}

function Wait-Scene {
    param([string]$Title, [string]$Scene, [string]$LogFile)
    Write-Step ("Escucha: " + $Title)
    Write-Host "  Lanzando: $Scene" -ForegroundColor Yellow
    Write-Host "  CONDUCE y ESCUCHA; " -NoNewline -ForegroundColor Yellow
    if ($AutoCloseSeconds -gt 0) {
        Write-Host "se cerrara sola en $AutoCloseSeconds s." -ForegroundColor Yellow
    } else {
        Write-Host "CIERRA LA VENTANA cuando hayas escuchado para continuar." -ForegroundColor Yellow
    }
    Write-Host "  (el audio va por la escena; si no oyes nada revisa el dispositivo de salida de Windows)"
    $p = Start-Process -FilePath $Godot -ArgumentList @("--path", $Project, $Scene) `
        -RedirectStandardOutput $LogFile -RedirectStandardError ($LogFile + ".err") -PassThru
    if ($AutoCloseSeconds -gt 0) {
        Start-Sleep -Seconds $AutoCloseSeconds
        if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
    } else {
        $p.WaitForExit()
    }
    Start-Sleep -Milliseconds 500
}

# ------------------------------------------------------------------ 1. preflight
Write-Step "Preflight"
foreach ($need in @($Godot, (Join-Path $Project "project.godot"), (Join-Path $Repo "SConstruct"))) {
    if (-not (Test-Path $need)) {
        Write-Fail "falta: $need"
        exit 1
    }
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Fail "cargo no esta en PATH (necesario para el harness A/B y el replay)."
    exit 1
}
if (-not (Test-Path (Join-Path $Project "sounds\banks\v10_vehicle\bank_manifest.json"))) {
    Write-Warn "banco legacy v10_vehicle ausente: los .legacy.wav del harness se omitiran (tools/audio/bank_generator.py)."
}
if (-not (Test-Path (Join-Path $Project "sounds\banks\v10_v2\vrc_2005_renault_r25.bank"))) {
    Write-Fail "falta el banco fuente v10_v2."
    exit 1
}
New-Item -ItemType Directory -Force -Path $Ab | Out-Null
Write-Ok "entorno verificado"

# ------------------------------------------------------------- 2. builds (si falta)
if (-not $SkipBuild) {
    $rustSrc = Newest-Mtime @(
        "$Crates\vehicle-audio-engine\src\*.rs",
        "$Crates\formula90-core\src\*.rs"
    )
    $rustDll = Join-Path $Project "addons\formula90s\bin\formula90_core.dll"
    $rustStale = $ForceBuild -or (-not (Test-Path $rustDll)) -or ((Get-Item $rustDll).LastWriteTime -lt $rustSrc)

    if ($rustStale) {
        Write-Step "Build Rust (release + deploy DLL)"
        Invoke-Native "cargo" @("build", "-p", "formula90_core", "--release") $Crates
        Copy-Item "$Crates\target\release\formula90_core.dll" $rustDll -Force
        Copy-Item "$Crates\target\release\formula90_core.dll" (Join-Path $Project "addons\formula90s\bin\formula90_core.windows.template_release.x86_64.dll") -Force
        Write-Ok "formula90_core desplegado"
    } else {
        Write-Ok "Rust al dia (sin rebuild)"
    }

    $cppSrc = Newest-Mtime @("$Repo\native\src\core\*.cpp", "$Repo\native\include\formula90s\core\*.h", "$Repo\native\include\formula90s\core\*.hpp")
    $extDll = Join-Path $Project "addons\formula90s\bin\libformula90s.windows.template_release.x86_64.dll"
    $extStale = $ForceBuild -or (-not (Test-Path $extDll)) -or ((Get-Item $extDll).LastWriteTime -lt $cppSrc)
    $scons = Get-Command scons -ErrorAction SilentlyContinue

    if ($extStale -and $scons) {
        Write-Step "Build GDExtension (release + debug — las escenas usan template_debug)"
        Invoke-Native "scons" @("-j8", "target=template_release") $Repo
        Invoke-Native "scons" @("-j8", "target=template_debug") $Repo
        Write-Ok "GDExtension desplegada"
    } elseif ($extStale) {
        Write-Warn "scons no disponible y la GDExtension parece desactualizada; las escenas podrian fallar al cargar el metodo nuevo."
    } else {
        Write-Ok "GDExtension al dia (sin rebuild)"
    }
} else {
    Write-Warn "-SkipBuild: no se verifica/construye nada"
}

# ------------------------------------------------ 3. artefactos A/B deterministas
if (-not $SkipGolden) {
    Write-Step "Golden trace (gates de cobertura / huecos)"
    $env:F90_GOLDEN_TRACE_OUT = Join-Path $Diag "golden_trace_v10_v2.json"
    Invoke-Native "cargo" @("test", "-p", "vehicle_audio_engine", "--test", "golden_trace") $Crates
    Remove-Item Env:F90_GOLDEN_TRACE_OUT
    Write-Ok "golden: $Diag\golden_trace_v10_v2.json"
}

Write-Step "Harness A/B (4 escenarios -> WAVs legacy + traces commands)"
Invoke-Native "cargo" @("test", "-p", "vehicle_audio_engine", "--test", "ab_legacy_commands", "--", "--nocapture") $Crates
Write-Ok "A/B: $Ab"

# ------------------------------------------------------------ 4. escenas (escucha)
if (-not $SkipListen) {
    $capInt = Join-Path $UserData "audio_capture_engine_int.txt"
    $capFull = Join-Path $UserData "audio_capture_full.txt"
    $liveCsv = Join-Path $UserData "telemetry_live.csv"
    Remove-Item $capInt, $capFull, $liveCsv -Force -ErrorAction SilentlyContinue

    Wait-Scene -Title "1/3  SWEEP INTERIOR (solo engine_int; mutes=254)" `
        -Scene "res://scenes/runtime/vehicle_test_session_engine_int.tscn" `
        -LogFile (Join-Path $Diag "gate_engine_int.log")
    if (Test-Path $capInt) { Copy-Item $capInt (Join-Path $Diag "gate_capture_engine_int.txt") -Force; Write-Ok "captura interior copiada" }

    Wait-Scene -Title "2/3  MEZCLA COMPLETA (mutes=0; chase cam a 10 m)" `
        -Scene "res://scenes/runtime/vehicle_test_session_full.tscn" `
        -LogFile (Join-Path $Diag "gate_full.log")
    if (Test-Path $capFull) { Copy-Item $capFull (Join-Path $Diag "gate_capture_full.txt") -Force; Write-Ok "captura completa copiada" }

    Wait-Scene -Title "3/3  GRABADOR DE SESION (graba TU telemetria para el A/B real)" `
        -Scene "res://scenes/runtime/vehicle_test_session_record.tscn" `
        -LogFile (Join-Path $Diag "gate_record.log")
    if (-not (Test-Path $liveCsv)) {
        Write-Warn "no se grabo telemetria (revisa $liveCsv); se omitira el replay real."
    } else {
        Copy-Item $liveCsv (Join-Path $Diag "telemetry_live.csv") -Force
        Write-Ok "telemetria copiada: $Diag\telemetry_live.csv"
    }
} else {
    # aun asi intentar copiar capturas previas si existen
    foreach ($pair in @(
        @{ Src = (Join-Path $UserData "audio_capture_engine_int.txt"); Dst = (Join-Path $Diag "gate_capture_engine_int.txt") },
        @{ Src = (Join-Path $UserData "audio_capture_full.txt");       Dst = (Join-Path $Diag "gate_capture_full.txt") },
        @{ Src = (Join-Path $UserData "telemetry_live.csv");           Dst = (Join-Path $Diag "telemetry_live.csv") }
    )) {
        if (Test-Path $pair.Src) { Copy-Item $pair.Src $pair.Dst -Force; Write-Ok ("captura previa: " + $pair.Dst) }
    }
}

# ----------------------------------------------- 5. replay offline de la sesion real
if (Test-Path (Join-Path $Diag "telemetry_live.csv")) {
    Write-Step "Replay A/B de la sesion real grabada (ambos backends, misma telemetria)"
    Invoke-Native "cargo" @("run", "-p", "vehicle_audio_engine", "--example", "live_replay", "--", (Join-Path $Diag "telemetry_live.csv")) $Crates
    Write-Ok "A/B real: $Ab\live_telemetry_live.{legacy.wav,commands.json}"
} else {
    Write-Warn "sin telemetria grabada; el replay real se omite (pasa a la escena 3/3 sin -SkipListen)."
}

# ------------------------------------------------------------- 6. resumen final
Write-Step "RESUMEN DEL GATE — artefactos para comparar"
$artifacts = @(
    , (Join-Path $Ab  "idle_to_redline_with_shifts.legacy.wav")
    , (Join-Path $Ab  "idle_to_redline_with_shifts.commands.json")
    , (Join-Path $Ab  "road_to_sand_with_kerb.legacy.wav")
    , (Join-Path $Ab  "road_to_sand_with_kerb.commands.json")
    , (Join-Path $Ab  "grass_skid.legacy.wav")
    , (Join-Path $Ab  "grass_skid.commands.json")
    , (Join-Path $Ab  "impact.legacy.wav")
    , (Join-Path $Ab  "impact.commands.json")
    , (Join-Path $Ab  "live_telemetry_live.legacy.wav")
    , (Join-Path $Ab  "live_telemetry_live.commands.json")
    , (Join-Path $Diag "gate_capture_engine_int.txt")
    , (Join-Path $Diag "gate_capture_full.txt")
)
foreach ($a in $artifacts) {
    if (Test-Path $a) {
        $size = if ((Get-Item $a).Length -gt 1MB) { "{0:N1} MB" -f ((Get-Item $a).Length / 1MB) } else { "{0:N0} KB" -f ((Get-Item $a).Length / 1KB) }
        Write-Host ("    " + $a.Replace($Repo + "\", "") + "  (" + $size + ")") -ForegroundColor Green
    } else {
        Write-Host ("    " + $a.Replace($Repo + "\", "") + "  (pendiente)") -ForegroundColor DarkGray
    }
}

Write-Host ""
Write-Host "  ORDEN DE ESCUCHA RECOMENDADO:" -ForegroundColor Yellow
Write-Host "    1. En las escenas ya conduciste el sweep interior y la mezcla completa."
Write-Host "    2. Escucha los .legacy.wav de diagnostics\ab\ (backend LEGACY de referencia)"
Write-Host "       y repite las mismas maniobras con el backend commands (escena full)."
Write-Host "    3. compara tus impresiones con el checklist:"
Write-Host "       docs\audio\GATE_AUDITIVO_CHECKLIST.md"
Write-Host "    Decisiones abiertas del gate: pitch/auto_pitch_ref, trims wind/wheel,"
Write-Host "    loops por capa (loop_regions.json), corte de 3 m interior/exterior."
Write-Host ""
Write-Host "  Si algo suena mal: dime capa + maniobra + rpm y adjunta la captura"
Write-Host "  (gate_capture_*.txt) o el momento del WAV A/B."
Write-Host "  El fallback legacy sigue intacto y el backend commands sigue OPT-IN."
Write-Host ""