$ErrorActionPreference = 'Stop'
& (Join-Path $PSScriptRoot '../../scripts/check.ps1') --relay
exit $LASTEXITCODE
