$runner = Join-Path $PSScriptRoot 'tools\common\content_flow.py'
& python $runner preview --repo $PSScriptRoot @args
exit $LASTEXITCODE
