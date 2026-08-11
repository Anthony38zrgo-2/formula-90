. "$PSScriptRoot\_common.ps1"

Write-Section "Rutas"
Write-Host "Repo      : $RepoRoot"
Write-Host ".agents   : $AgentsRoot"
Write-Host "DB        : $DbPath"
Write-Host "Runtime   : $RuntimeRoot"

Write-Section "Rust"
$rustc = Get-Command rustc -ErrorAction SilentlyContinue
$cargo = Get-Command cargo -ErrorAction SilentlyContinue
if ($rustc) { & $rustc.Source --version } else { Write-Warning "rustc no encontrado" }
if ($cargo) { & $cargo.Source --version } else { Write-Warning "cargo no encontrado" }

Write-Section "SQLite"
$sqlite = Get-SqliteExe
if ($sqlite) {
    $localPath = [System.IO.Path]::GetFullPath((Join-Path $AgentsRoot "tools\sqlite\sqlite3.exe"))
    if ([string]::Equals($sqlite, $localPath, [System.StringComparison]::OrdinalIgnoreCase)) {
        Write-Host "Source : PROJECT LOCAL"
    } else {
        Write-Host "Source : SYSTEM PATH"
    }
    Write-Host "Path   : $sqlite"
    & $sqlite --version
} else {
    Write-Warning "sqlite3 no encontrado"
}

Write-Section "AgentDB"
$r = Resolve-AgentDb
if ($r) {
    Write-Host ("Path      : {0}" -f $r.path)
    Write-Host ("Source    : {0}" -f $r.source)
    Write-Host ("Built now : {0}" -f $(if ($r.built_now) { "YES" } else { "NO" }))
} else {
    Write-Warning "agentdb no disponible"
}
Write-Host ("AGENTS_ROOT: {0}" -f $env:AGENTS_ROOT)
Write-Host ("AGENT_DB   : {0}" -f $env:AGENT_DB)

Write-Section "DB"
if (Test-Path -LiteralPath $DbPath) {
    $info = Get-Item $DbPath
    Write-Host ("Existe     : SI")
    Write-Host ("Tamano     : {0:N0} bytes" -f $info.Length)
    Write-Host ("Modificada : {0}" -f $info.LastWriteTime)
} else {
    Write-Warning "agents.db todavia no existe"
}
