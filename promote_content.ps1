$runner = Join-Path $PSScriptRoot 'tools\common\content_flow.py'
& python $runner promote --repo $PSScriptRoot @args
exit $LASTEXITCODE
