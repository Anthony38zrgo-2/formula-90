. "$PSScriptRoot\_common.ps1"

$tests = @(
    @{ Name="Godot RigidBody3D"; Args=@("knowledge","godot","RigidBody3D") },
    @{ Name="Godot RayCast";     Args=@("knowledge","godot","raycast curb") },
    @{ Name="Blender glTF";      Args=@("knowledge","blender","gltf modifiers") },
    @{ Name="C++ vector";        Args=@("knowledge","cpp","vector reserve") },
    @{ Name="Python subprocess"; Args=@("knowledge","python","subprocess") },
    @{ Name="Known problem";     Args=@("problem","node_paths") },
    @{ Name="Agent";             Args=@("agent","developer-godot") }
)

$failed = $false

Write-Section "agent governance"
try {
    $python = Get-PythonExecutable
    if (-not $python) { throw 'Python no disponible para validate_agents.py.' }
    & $python (Join-Path $RepoRoot 'tools\agent_validation\validate_agents.py')
    if ($LASTEXITCODE -ne 0) { $failed = $true }
} catch {
    Write-Error $_
    $failed = $true
}

Write-Section "validate"
try {
    Invoke-AgentDb -Arguments @("validate")
} catch {
    Write-Error $_
    $failed = $true
}

foreach ($test in $tests) {
    Write-Section $test.Name
    try {
        $agentdb = Assert-AgentDb
        Push-Location $RepoRoot
        try {
            $output = (& $agentdb @($test.Args) 2>&1 | Out-String).Trim()
            $code = $LASTEXITCODE
        } finally {
            Pop-Location
        }

        if ($code -ne 0 -or [string]::IsNullOrWhiteSpace($output)) {
            Write-Error "FAIL: $($test.Name)"
            $failed = $true
        } else {
            Write-Host $output
            Write-Host "PASS: $($test.Name)"
        }
    } catch {
        Write-Error $_
        $failed = $true
    }
}

if ($failed) {
    Write-Error "SMOKE TEST: FAIL"
    exit 1
}

Write-Host ""
Write-Host "SMOKE TEST: PASS"
exit 0
