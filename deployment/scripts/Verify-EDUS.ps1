#requires -Version 5.1
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
try {
    Import-Module (Join-Path $PSScriptRoot 'Edus.Deployment.psm1') -Force
    Assert-EdusAdministrator
    $package = Split-Path -Parent $PSScriptRoot
    $null = Test-EdusPackage $package
    $log = Join-Path ([IO.Path]::GetTempPath()) ('EDUS-Verify-' + [Guid]::NewGuid().ToString('N') + '.jsonl')
    $results = @(Test-EdusInstallation -PackageRoot $package -Log $log)
    $results | Format-Table -AutoSize
    Write-Host "Log: $log"
    if (@($results | Where-Object { $_.Result -eq 'FAIL' }).Count) { exit 1 }
    exit 0
} catch { Write-Error -Message $_.Exception.Message -ErrorAction Continue; exit 1 }
