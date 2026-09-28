#requires -Version 5.1
[CmdletBinding()]
param([switch]$RemoveData)
$ErrorActionPreference = 'Stop'
try {
    Import-Module (Join-Path $PSScriptRoot 'Edus.Deployment.psm1') -Force
    Invoke-EdusRemove -PackageRoot (Split-Path -Parent $PSScriptRoot) -RemoveData:$RemoveData
    exit 0
} catch { Write-Error -Message ("EDUS remove failed. " + $_.Exception.Message) -ErrorAction Continue; exit 1 }
