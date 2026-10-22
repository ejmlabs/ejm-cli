$ErrorActionPreference = 'Stop'
& python3 (Join-Path $PSScriptRoot '../../scripts/check.py') --relay
exit $LASTEXITCODE
