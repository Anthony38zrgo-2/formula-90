[CmdletBinding()]
param()
# HUD-1205 repository grep gate: fail the build when any removed physical
# state leaks back into runtime telemetry/HUD surfaces.
#
# Removed state (BRAKE-1000, AERO-800/900/1400) must never be exposed again:
#   - caliper_c / hub_c            brake/tire snapshot keys
#   - diffuser_expansion(deg)      aero telemetry
#   - flex_factor                  wing-flex aero telemetry
#   - aero_yaw_decay_exponent      config-level yaw decay (AERO-1400)
#
# Deliberately NOT flagged: per-element aero "yaw_decay_exponent" JSON keys
# (live physical state) and v2 parse-only "caliper_cooling_weight" /
# "hub_cooling_weight" config keys (purged with CAL-1300 schema v3).
# game\tests is scanned separately? No: tests assert ABSENCE and therefore
# legitimately contain these strings - exclude them from the raw gate.

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

$patterns = @(
    '"caliper_c"',
    "'caliper_c'",
    '"hub_c"',
    "'hub_c'",
    'diffuser_expansion',
    'flex_factor',
    'aero_yaw_decay_exponent'
)
$roots = @(
    (Join-Path $root 'game\scripts'),
    (Join-Path $root 'game\features'),
    (Join-Path $root 'game\crates'),
    (Join-Path $root 'game\data\vehicles'),
    (Join-Path $root 'native\src'),
    (Join-Path $root 'native\include')
)

$violations = @()
foreach ($item in $roots) {
    Get-ChildItem -LiteralPath $item -Recurse -File |
        Where-Object {
            $_.FullName -notmatch '\\target|\\\.godot|\\.git\\|\.obj(\.int07-old)?$' -and
            $_.Extension -notmatch '^\.(obj|o|lib|a|exe|dll|pdb|so|dylib|wasm|bin|wav|json\.import|uid)$'
        } |
        ForEach-Object {
            $file = $_
            foreach ($pattern in $patterns) {
                if (Select-String -LiteralPath $file.FullName -SimpleMatch -Pattern $pattern -Quiet) {
                    $violations += "$($file.FullName.Substring($root.Length + 1)) -> $pattern"
                }
            }
        }
}

if ($violations.Count -gt 0) {
    Write-Host "Gate FAILED: removed telemetry/HUD state detected:" -ForegroundColor Red
    $violations | ForEach-Object { Write-Host "  $_" -ForegroundColor Red }
    exit 1
}
Write-Host 'Gate passed: no removed telemetry/HUD state in runtime surfaces.' -ForegroundColor Green
