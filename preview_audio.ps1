Push-Location $PSScriptRoot
try {
    & python -m tools.audio.preview_ab --repo-root $PSScriptRoot @args
    $result = $LASTEXITCODE
} finally {
    Pop-Location
}
exit $result
