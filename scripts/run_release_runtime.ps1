[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Script,
    [string[]]$ScriptArgs = @(),
    [string]$GodotPath
)
# QA-only: run a script against the RELEASE GDExtension build. The editor binary
# selects `windows.editor.*` entries, which ship pointing at template_debug, so
# release can only be exercised by temporarily swapping those two lines. The
# original file is restored (and hash-verified) in `finally` — never leave the
# tree dirty.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$gdext = Join-Path $root 'game\addons\formula90s\formula90s.gdextension'
$bin = Join-Path $root 'game\addons\formula90s\bin'
$releaseDll = Join-Path $bin 'libformula90s.windows.template_release.x86_64.dll'
if (-not (Test-Path -LiteralPath $releaseDll)) {
    throw "Release extension missing: $releaseDll. Run scripts/build_windows.ps1 -Configuration release first."
}
$godot = if ($GodotPath) { $GodotPath } else { Join-Path $root '.tools\godot\Godot_v4.7.1-stable_win64_console.exe' }
if (-not (Test-Path -LiteralPath $godot)) { throw "Godot not found: $godot" }

$original = Get-Content -LiteralPath $gdext -Raw
$originalHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $gdext).Hash
$patched = $original -replace '(windows\.editor\.dev\.x86_64\s*=\s*")[^"]+(")', '$1res://addons/formula90s/bin/libformula90s.windows.template_release.x86_64.dll$2'
$patched = $patched -replace '(windows\.editor\.x86_64\s*=\s*")[^"]+(")', '$1res://addons/formula90s/bin/libformula90s.windows.template_release.x86_64.dll$2'
if ($patched -eq $original) {
    throw "No windows.editor entries were swapped; .gdextension layout changed?"
}
Set-Content -LiteralPath $gdext -Value $patched -NoNewline
Write-Host "release-runtime: editor entries -> template_release" -ForegroundColor Cyan

$exit = 1
try {
    $args = @('--path', (Join-Path $root 'game'), '--script', $Script)
    if ($ScriptArgs.Count -gt 0) { $args += '--'; $args += $ScriptArgs }
    & $godot @args
    $exit = $LASTEXITCODE
}
finally {
    # Godot (or the AV scanner) can hold the mapping for a moment after exit;
    # retry the restore before declaring failure.
    $restored = $false
    for ($attempt = 0; $attempt -lt 25; $attempt++) {
        try {
            [System.IO.File]::WriteAllText($gdext, $original)
            $restored = $true
            break
        }
        catch {
            Start-Sleep -Milliseconds 200
        }
    }
    $restoredHash = if (Test-Path -LiteralPath $gdext) { (Get-FileHash -Algorithm SHA256 -LiteralPath $gdext).Hash } else { '' }
    if (-not $restored -or $restoredHash -ne $originalHash) {
        $bak = Join-Path ([System.IO.Path]::GetTempPath()) 'formula90s.gdextension.original'
        [System.IO.File]::WriteAllText($bak, $original)
        throw "FATAL: formula90s.gdextension was not restored (hash mismatch). Original saved to $bak. Original hash=$originalHash current=$restoredHash"
    }
    Write-Host "release-runtime: .gdextension restored (hash verified)" -ForegroundColor Green
}
exit $exit
