$launcher = Join-Path $PSScriptRoot 'scripts\run_f1_94_no_rebuild.ps1'
& $launcher @args
exit $LASTEXITCODE
