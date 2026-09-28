#requires -Version 5.1
[CmdletBinding()]
param()
# Same validated staging/lifecycle restores missing binaries without touching DB/key contents.
& (Join-Path $PSScriptRoot 'Setup-EDUS.ps1')
exit $LASTEXITCODE
