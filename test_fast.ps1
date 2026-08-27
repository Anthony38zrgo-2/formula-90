$runner = Join-Path $PSScriptRoot 'scripts\test_fast.ps1'
& $runner @args
exit $LASTEXITCODE
