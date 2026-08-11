. "$PSScriptRoot\_common.ps1"

Write-Section "ROUTING"

$configPath = Join-Path $AgentsRoot "config\model-routing.json"
$config = Get-Content $configPath -Raw | ConvertFrom-Json

$deepseekAvail = Get-DeepSeekAvailable
$deepseekState = if ($config.providers.deepseek.enabled -and $deepseekAvail) { "AVAILABLE" } else { "UNAVAILABLE" }
Write-Host ("DeepSeek Flash       {0}   ({1})" -f $deepseekState, $config.providers.deepseek.default_model)

$codex = Get-CodexExe
if ($codex) {
    $version = (& $codex --version 2>$null | Out-String).Trim()
    Write-Host ("Codex CLI            {0}" -f $version)
    Write-Host ("Codex binary         {0}" -f $codex)
} else {
    Write-Host "Codex CLI            NOT FOUND"
}

Write-Section "CODEX PROFILES"

foreach ($profileName in ($config.codex_profiles.PSObject.Properties.Name | Sort-Object)) {
    $spec = $config.codex_profiles.$profileName
    $file = Join-Path $env:USERPROFILE ".codex\$profileName.config.toml"
    $fileState = if (Test-Path $file) { "CONFIGURED" } else { "MISSING" }
    Write-Host ("{0,-14} {1,-10} model={2} effort={3}" -f $profileName, $fileState, $spec.model, $spec.effort)
}

Write-Section "VERIFICATION"

$agentdb = Assert-AgentDb
$verifiedState = @{}
$verifiedCount = @{}
$totalCount = @{}
if (Test-Path $DbPath) {
    $sql = "SELECT requested_model, requested_effort, SUM(CASE WHEN verification_status='verified' THEN 1 ELSE 0 END), COUNT(*) FROM model_calls GROUP BY requested_model, requested_effort ORDER BY requested_model, requested_effort;"
    $rows = & (Assert-Sqlite) -readonly $DbPath $sql
    foreach ($profileName in ($config.codex_profiles.PSObject.Properties.Name)) {
        $spec = $config.codex_profiles.$profileName
        $verifiedState[$profileName] = $false
        $verifiedCount[$profileName] = 0
        $totalCount[$profileName] = 0
    }
    foreach ($line in $rows) {
        $parts = $line -split "\|"
        if ($parts.Count -lt 4) { continue }
        $model = $parts[0].Trim()
        $effort = $parts[1].Trim()
        $v = [int]$parts[2].Trim()
        $t = [int]$parts[3].Trim()
        foreach ($profileName in ($config.codex_profiles.PSObject.Properties.Name)) {
            $spec = $config.codex_profiles.$profileName
            if ($spec.model -eq $model -and $spec.effort -eq $effort) {
                $verifiedCount[$profileName] += $v
                $totalCount[$profileName] += $t
                if ($v -gt 0) { $verifiedState[$profileName] = $true }
            }
        }
    }
}

foreach ($profileName in ($config.codex_profiles.PSObject.Properties.Name | Sort-Object)) {
    $state = if ($verifiedState[$profileName]) { "VERIFIED" } else { "UNVERIFIED" }
    $evidence = if ($totalCount[$profileName] -gt 0) {
        "{0}/{1} verified calls" -f $verifiedCount[$profileName], $totalCount[$profileName]
    } else {
        "no calls recorded"
    }
    Write-Host ("{0,-14} {1,-12} {2}" -f $profileName, $state, $evidence)
}

Write-Section "DEFAULTS"

Write-Host ("Default coding        {0}" -f $config.defaults.cheap_worker)
Write-Host ("Default Codex worker  {0} (effort {1})" -f $config.defaults.codex_worker, $config.luna_effort.implementation)
Write-Host ("Strong escalation     {0} (effort {1})" -f $config.defaults.strong_model, $config.sol_effort.default)
Write-Host ("Max attempts before Sol escalation: {0}" -f $config.limits.max_attempts_before_strong_escalation)
Write-Host ("Cross-subsystem threshold: {0}" -f $config.limits.cross_subsystem_threshold)

Write-Host ""
Write-Host "NOTE: 'UNVERIFIED' means the effective runtime model could not be observed by the installed Codex CLI (no model field in codex exec JSONL events). Requested configuration exists; effective model is not observable. This is honest reporting, not a failure."
