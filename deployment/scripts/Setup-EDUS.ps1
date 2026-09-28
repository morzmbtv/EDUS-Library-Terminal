#requires -Version 5.1
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
try {
    Import-Module (Join-Path $PSScriptRoot 'Edus.Deployment.psm1') -Force
    Invoke-EdusSetup -PackageRoot (Split-Path -Parent $PSScriptRoot)
    exit 0
} catch { Write-Error -Message ("EDUS setup failed. Existing data are preserved. " + $_.Exception.Message) -ErrorAction Continue; exit 1 }
