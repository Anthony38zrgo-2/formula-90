Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptRoot = $PSScriptRoot

# ---------------------------------------------------------------------------
# Repository root discovery: CWD-independent. Prefers git (works in worktrees
# where .git may be a FILE, not a directory); falls back to a bounded
# parent-directory search for a .git entry.
# ---------------------------------------------------------------------------
function Get-RepoRoot {
    $git = Get-Command git -ErrorAction SilentlyContinue
    if ($git) {
        $out = & $git -C $ScriptRoot rev-parse --show-toplevel 2>$null
        if ($LASTEXITCODE -eq 0 -and $out) {
            $line = [string]($out | Select-Object -First 1)
            $line = $line.Trim()
            if ($line) {
                return [System.IO.Path]::GetFullPath($line)
            }
        }
    }

    $dir = [System.IO.Path]::GetFullPath($ScriptRoot)
    for ($i = 0; $i -lt 12; $i++) {
        if (Test-Path -LiteralPath (Join-Path $dir ".git")) {
            return $dir
        }
        $parent = Split-Path -Path $dir -Parent
        if (-not $parent -or $parent -eq $dir) { break }
        $dir = $parent
    }
    return $null
}

$RepoRoot = Get-RepoRoot
if (-not $RepoRoot) {
    throw ("No se pudo localizar la raiz del repositorio desde '{0}'. " +
        "Se intento 'git rev-parse --show-toplevel' y una busqueda ascendente de .git." -f $ScriptRoot)
}

$AgentsRoot = [System.IO.Path]::GetFullPath((Join-Path $RepoRoot ".agents"))
$DbPath = [System.IO.Path]::GetFullPath((Join-Path $AgentsRoot "data\agents.db"))
$RuntimeRoot = [System.IO.Path]::GetFullPath((Join-Path $AgentsRoot "runtime"))

$AgentsLeaf = Split-Path $AgentsRoot -Leaf
if ($AgentsLeaf -ne ".agents") {
    throw ("Layout invalido: se esperaba '<repo>\.agents'. Ruta detectada: '{0}'." -f $AgentsRoot)
}

# Absolute environment for agentdb. Never resolve the DB against CWD.
function Set-AgentDbEnv {
    $env:AGENTS_ROOT = $AgentsRoot
    $env:AGENT_DB = $DbPath
}
Set-AgentDbEnv

$script:AgentDbResult = $null

function Write-Section {
    param([Parameter(Mandatory=$true)][string]$Title)
    Write-Host ""
    Write-Host ("=== {0} ===" -f $Title)
}

function Get-SqliteExe {
    $LocalSqlite = Join-Path $AgentsRoot "tools\sqlite\sqlite3.exe"

    if (Test-Path -LiteralPath $LocalSqlite) {
        return [System.IO.Path]::GetFullPath($LocalSqlite)
    }

    $cmd = Get-Command sqlite3 -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }

    return $null
}

# ---------------------------------------------------------------------------
# Binary validation: finding the file is not enough. Execute it against the
# project database with absolute env. On a fresh database (no schema yet) an
# idempotent `init` retry is performed before declaring the binary healthy.
# ---------------------------------------------------------------------------
function Test-AgentDbBinary {
    param([Parameter(Mandatory=$true)][string]$Path)

    if (-not (Test-Path -LiteralPath $Path)) { return $false }

    Set-AgentDbEnv
    $prevEAP = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    & $Path stats 2>$null | Out-Null
    if ($LASTEXITCODE -ne 0) {
        & $Path init 2>$null | Out-Null
        if ($LASTEXITCODE -eq 0) {
            & $Path stats 2>$null | Out-Null
        }
    }
    $ok = ($LASTEXITCODE -eq 0)
    $ErrorActionPreference = $prevEAP
    return $ok
}

function Invoke-AgentDbBuild {
    $cargo = Get-Command cargo -ErrorAction SilentlyContinue
    if (-not $cargo) { return $false }

    $cargoToml = Join-Path $RuntimeRoot "Cargo.toml"
    if (-not (Test-Path -LiteralPath $cargoToml)) { return $false }

    Push-Location $RuntimeRoot
    try {
        $prevEAP = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        & $cargo.Source build --release
        $ok = ($LASTEXITCODE -eq 0)
        $ErrorActionPreference = $prevEAP
    }
    finally {
        Pop-Location
    }
    return $ok
}

# ---------------------------------------------------------------------------
# Central agentdb resolver. Resolution order:
#   1. AGENTDB_EXE / AGENTDB_BIN (explicit env override, validated)
#   2. project release binary
#   3. project debug binary (temporary fallback only)
#   4. agentdb in PATH
#   5. automatic local build (cargo build --release) when source exists
# Returns a result object { available, path, source, built_now } or $null.
# Sources: explicit_env, project_release, project_debug, system_path, built_release
# ---------------------------------------------------------------------------
function Resolve-AgentDb {
    param([switch]$NoCache)

    foreach ($envName in @("AGENTDB_EXE", "AGENTDB_BIN")) {
        $candidate = [System.Environment]::GetEnvironmentVariable($envName)
        if ($candidate -and (Test-AgentDbBinary $candidate)) {
            return [pscustomobject]@{
                available = $true
                path      = [System.IO.Path]::GetFullPath($candidate)
                source    = "explicit_env"
                built_now = $false
            }
        }
    }

    if (-not $NoCache -and $script:AgentDbResult) {
        return $script:AgentDbResult
    }

    $release = Join-Path $RuntimeRoot "target\release\agentdb.exe"
    if (Test-AgentDbBinary $release) {
        $script:AgentDbResult = [pscustomobject]@{
            available = $true
            path      = $release
            source    = "project_release"
            built_now = $false
        }
        return $script:AgentDbResult
    }

    $debug = Join-Path $RuntimeRoot "target\debug\agentdb.exe"
    if (Test-AgentDbBinary $debug) {
        $script:AgentDbResult = [pscustomobject]@{
            available = $true
            path      = $debug
            source    = "project_debug"
            built_now = $false
        }
        return $script:AgentDbResult
    }

    $cmd = Get-Command agentdb -ErrorAction SilentlyContinue
    if ($cmd -and (Test-AgentDbBinary $cmd.Source)) {
        $script:AgentDbResult = [pscustomobject]@{
            available = $true
            path      = $cmd.Source
            source    = "system_path"
            built_now = $false
        }
        return $script:AgentDbResult
    }

    $cargoToml = Join-Path $RuntimeRoot "Cargo.toml"
    $cargo = Get-Command cargo -ErrorAction SilentlyContinue
    if ((Test-Path -LiteralPath $cargoToml) -and $cargo) {
        Write-Host "agentdb no compilado; compilando release local (cargo build --release)..."
        if (Invoke-AgentDbBuild) {
            $release = Join-Path $RuntimeRoot "target\release\agentdb.exe"
            if (Test-AgentDbBinary $release) {
                $script:AgentDbResult = [pscustomobject]@{
                    available = $true
                    path      = $release
                    source    = "built_release"
                    built_now = $true
                }
                return $script:AgentDbResult
            }
        }
    }

    return $null
}

# ---------------------------------------------------------------------------
# Structured diagnostics that distinguish:
#   binary missing / build unavailable / database unavailable / query miss
# ---------------------------------------------------------------------------
function Get-BootstrapDiagnostics {
    param([switch]$NoCache)

    $r = Resolve-AgentDb -NoCache:$NoCache
    $cargoToml = Join-Path $RuntimeRoot "Cargo.toml"
    $cargo = Get-Command cargo -ErrorAction SilentlyContinue

    return [pscustomobject]@{
        binary_available      = [bool]$r
        binary_source         = $(if ($r) { $r.source } else { "none" })
        binary_path           = $(if ($r) { $r.path } else { $null })
        cargo_available       = [bool]$cargo
        runtime_source_present = (Test-Path -LiteralPath $cargoToml)
        build_available       = [bool]($cargo -and (Test-Path -LiteralPath $cargoToml))
        database_available    = (Test-Path -LiteralPath $DbPath)
        database_path         = $DbPath
    }
}

function Get-AgentDbExe {
    $r = Resolve-AgentDb
    if ($r) { return $r.path }
    return $null
}

function Assert-AgentDb {
    $r = Resolve-AgentDb
    if (-not $r) {
        throw ("agentdb no disponible: sin binario valido y sin build automatico posible. " +
            "Infraestructura .agents NO disponible. Compila .agents\runtime con 'cargo build --release' " +
            "o define AGENTDB_EXE apuntando a un binario valido.")
    }
    return $r.path
}

function Assert-Database {
    if (-not (Test-Path -LiteralPath $DbPath)) {
        throw ("No existe la DB: {0}. Ejecuta 00-preflight.ps1 (bootstrap automatico) " +
            "o 07-rebuild-db.ps1 -ConfirmRebuild." -f $DbPath)
    }
}

# ---------------------------------------------------------------------------
# Fresh-workspace database bootstrap. Existing databases are never touched.
# Missing database: init + seed + validate (reproducible, non-destructive).
# ---------------------------------------------------------------------------
function Ensure-Database {
    if (Test-Path -LiteralPath $DbPath) {
        return [pscustomobject]@{ available = $true; initialized_now = $false }
    }

    $agentdb = Assert-AgentDb
    Invoke-AgentDb -Arguments @("init")
    Invoke-AgentDb -Arguments @("seed")
    Invoke-AgentDb -Arguments @("validate")
    return [pscustomobject]@{ available = $true; initialized_now = $true }
}

# ---------------------------------------------------------------------------
# Full bootstrap result:
#   { ok, repo_root, agents_root,
#     agentdb:  { available, path, source, built_now },
#     database: { path, available } }
# ---------------------------------------------------------------------------
function Resolve-Bootstrap {
    param([switch]$NoCache)

    $agentdb = Resolve-AgentDb -NoCache:$NoCache

    $database = $null
    if ($agentdb) {
        try {
            $database = Ensure-Database
        }
        catch {
            $database = $null
        }
    }

    $agentdbResult = if ($agentdb) {
        [pscustomobject]@{
            available = $true
            path      = $agentdb.path
            source    = $agentdb.source
            built_now = $agentdb.built_now
        }
    }
    else {
        [pscustomobject]@{
            available = $false
            path      = $null
            source    = $null
            built_now = $false
        }
    }

    $databaseResult = if ($database) {
        [pscustomobject]@{ path = $DbPath; available = $true }
    }
    else {
        [pscustomobject]@{ path = $DbPath; available = $false }
    }

    return [pscustomobject]@{
        ok          = ($agentdbResult.available -and $databaseResult.available)
        repo_root   = $RepoRoot
        agents_root = $AgentsRoot
        agentdb     = $agentdbResult
        database    = $databaseResult
    }
}

function Assert-Sqlite {
    $sqlite = Get-SqliteExe
    if (-not $sqlite) {
        throw "sqlite3 no esta disponible. Ejecuta .agents\scripts\install-sqlite.ps1 para instalarlo en .agents\tools\sqlite."
    }
    return $sqlite
}

function Invoke-SqliteReadOnly {
    param(
        [Parameter(Mandatory=$true)][string]$Sql,
        [switch]$Raw
    )
    Assert-Database
    $sqlite = Assert-Sqlite

    if ($Raw) {
        & $sqlite -readonly $DbPath $Sql
    } else {
        & $sqlite -readonly -header -column $DbPath $Sql
    }

    if ($LASTEXITCODE -ne 0) {
        throw "sqlite3 termino con codigo $LASTEXITCODE"
    }
}

function Invoke-AgentDb {
    param(
        [Parameter(Mandatory=$true)][string[]]$Arguments
    )
    $agentdb = Assert-AgentDb
    Set-AgentDbEnv

    Push-Location $RepoRoot
    try {
        & $agentdb @Arguments
        if ($LASTEXITCODE -ne 0) {
            throw ("agentdb termino con codigo {0}: {1}" -f $LASTEXITCODE, ($Arguments -join " "))
        }
    }
    finally {
        Pop-Location
    }
}

function Assert-SafeIdentifier {
    param([Parameter(Mandatory=$true)][string]$Name)
    if ($Name -notmatch '^[A-Za-z_][A-Za-z0-9_]*$') {
        throw "Identificador SQL no permitido: $Name"
    }
}

function ConvertTo-NativeArg {
    param([Parameter(Mandatory=$true)][AllowEmptyString()][string]$Value)
    return $Value.Replace('"', '\"')
}

function ConvertTo-HexJson {
    param([Parameter(Mandatory=$true)][AllowEmptyString()][string]$Json)
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Json)
    $hex = -join ($bytes | ForEach-Object { $_.ToString("x2") })
    return "hex:$hex"
}

function Get-CodexExe {
    $envCodex = $env:CODEX_CLI_PATH
    if ($envCodex -and (Test-Path -LiteralPath $envCodex)) {
        return [System.IO.Path]::GetFullPath($envCodex)
    }

    $candidates = @()
    $binRoot = Join-Path $env:LOCALAPPDATA "OpenAI\Codex\bin"
    if (Test-Path -LiteralPath $binRoot) {
        $candidates += Get-ChildItem $binRoot -Recurse -Filter "codex.exe" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty FullName
    }

    $homeConfig = Join-Path $env:USERPROFILE ".codex\config.toml"
    if (Test-Path -LiteralPath $homeConfig) {
        $raw = Get-Content $homeConfig -Raw -ErrorAction SilentlyContinue
        if ($raw -match "CODEX_CLI_PATH\s*=\s*'([^']+)'") {
            $candidates += $matches[1]
        }
    }

    foreach ($candidate in ($candidates | Where-Object { $_ -and (Test-Path -LiteralPath $_) })) {
        return [System.IO.Path]::GetFullPath($candidate)
    }

    $cmd = Get-Command codex -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }

    return $null
}

function Get-DeepSeekAvailable {
    if ($env:AGENT_DEEPSEEK_AVAILABLE) {
        return [bool]$env:AGENT_DEEPSEEK_AVAILABLE
    }
    if (-not $env:DEEPSEEK_API_KEY) {
        return $false
    }
    try {
        $models = (& opencode models 2>$null | Out-String)
        return $models -match "deepseek/deepseek-v4-flash"
    } catch {
        return $false
    }
}

function Set-DeepSeekAvailableEnv {
    $avail = Get-DeepSeekAvailable
    if ($avail) { $env:AGENT_DEEPSEEK_AVAILABLE = "1" } else { $env:AGENT_DEEPSEEK_AVAILABLE = "0" }
}
