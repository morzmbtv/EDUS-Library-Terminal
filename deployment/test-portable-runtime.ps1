#requires -Version 7.0
# Explicit development-only probe. Never registers a service or modifies Windows ACLs.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$PackageRoot,
    [string]$DataRoot = 'E:\Codex\temp\edus-split-2.0-browser-test'
)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
if (!$DataRoot.StartsWith('E:\Codex\temp\edus-', [StringComparison]::OrdinalIgnoreCase)) { throw 'Only the explicitly isolated E: fixture is permitted' }
if (!(Test-Path (Join-Path $DataRoot 'workspace.json'))) { throw 'Prepare the explicit synthetic UAT fixture first' }
Import-Module (Join-Path $PSScriptRoot 'scripts\Edus.Deployment.psm1') -Force
$null=Test-EdusPackage $PackageRoot
$exe=Join-Path $PackageRoot 'backend\EDUSLibraryService.exe'
$env:TEMP='E:\Codex\temp';$env:TMP=$env:TEMP
$env:EDUS_SERVICE_DATA_ROOT=$DataRoot
Remove-Item Env:EDUS_SERVICE_WEB_ROOT -ErrorAction SilentlyContinue
$evidence=Join-Path $PSScriptRoot ('.build\runtime-'+[Guid]::NewGuid().ToString('N'))
$null=New-Item -ItemType Directory -Path $evidence
function Start-Console([string]$Suffix) {
    $child=Start-Process -FilePath $exe -ArgumentList '--console' -WorkingDirectory (Split-Path $exe) -WindowStyle Hidden -PassThru -RedirectStandardOutput "$evidence\$Suffix.stdout.log" -RedirectStandardError "$evidence\$Suffix.stderr.log"
    $deadline=[DateTime]::UtcNow.AddSeconds(20)
    do {
        if ($child.HasExited) { throw "Console exited: $($child.ExitCode)" }
        try {
            $health=Invoke-RestMethod 'http://127.0.0.1:43180/health' -TimeoutSec 1
            if ($health.pid -eq $child.Id -and $health.state -eq 'READY' -and $health.mode -eq 'UAT') {return $child}
        } catch { }
        Start-Sleep -Milliseconds 150
    } while ([DateTime]::UtcNow -lt $deadline)
    Stop-Process -Id $child.Id -Force
    throw 'UAT health did not become READY'
}
function Read-State {
    $headers=@{Origin='http://127.0.0.1:43180'}
    $auth=Invoke-RestMethod 'http://127.0.0.1:43180/api/local/v1/session/renew' -Method Post -ContentType 'application/json' -Body '{}' -Headers $headers -SessionVariable session
    $headers['X-EDUS-CSRF']=$auth.csrfToken
    $reader=(Invoke-RestMethod 'http://127.0.0.1:43180/api/local/v1/identify/card' -Method Post -ContentType 'application/json' -Body '{"code":"00001009"}' -Headers $headers -WebSession $session).reader
    $snapshot=Invoke-RestMethod 'http://127.0.0.1:43180/api/local/v1/snapshot' -WebSession $session
    $settings=Invoke-RestMethod 'http://127.0.0.1:43180/api/local/v1/settings/ui' -WebSession $session
    $state=@{reader_id=$reader.id;locale=$settings.locale;loans=$snapshot.loans;reservations=$snapshot.reservations;copies=$snapshot.copies}
    $json=$state|ConvertTo-Json -Depth 25 -Compress
    $hash=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($json)))
    return @{hash=$hash;reader_id=$reader.id;locale=$settings.locale;loans=@($snapshot.loans).Count;reservations=@($snapshot.reservations).Count}
}
# Binding the port is only a read-only exclusivity check; never kill a foreign listener.
$probe=[Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback,43180)
try {$probe.Start()} finally {$probe.Stop()}
$first=$null;$second=$null
try {
    $first=Start-Console 'before'
    $before=Read-State
    $dlls=@($first.Modules|Where-Object ModuleName -Match '^vcruntime'|ForEach-Object {$_.FileName})
    if (!$dlls.Count -or @($dlls|Where-Object {!$_.StartsWith((Join-Path $PackageRoot 'backend')+'\',[StringComparison]::OrdinalIgnoreCase)}).Count) {throw 'VC runtime not loaded app-locally'}
    $secrets=@(Get-ChildItem $DataRoot -Recurse -Filter '*.dpapi'|ForEach-Object {(Get-FileHash $_.FullName).Hash})
    if (!$secrets.Count) {throw 'No service DPAPI file'}
    # Intentional process termination tests SQLCipher/WAL recovery, not SCM recovery.
    Stop-Process -Id $first.Id -Force;$first.WaitForExit();$first=$null
    $second=Start-Console 'after'
    $after=Read-State
    $reopened=@(Get-ChildItem $DataRoot -Recurse -Filter '*.dpapi'|ForEach-Object {(Get-FileHash $_.FullName).Hash})
    if ($before.hash -ne $after.hash -or ($secrets -join ',') -ne ($reopened -join ',')) {throw 'Restart state or DPAPI blob changed'}
    $result=@{status='PASS';scope='Extracted release console process under development user; NOT SCM/LocalService';portable_frontend_config='PASS';app_local_runtime='PASS';crash_reopen='PASS';same_card_and_data='PASS';same_dpapi_blob='PASS';locale=$after.locale;loans=$after.loans;reservations=$after.reservations;time=[DateTime]::UtcNow.ToString('o')}
    $result|ConvertTo-Json|Set-Content "$evidence\result.json" -Encoding utf8
    $result|ConvertTo-Json
    Write-Host "Evidence: $evidence"
} finally {
    foreach($child in @($first,$second)){if($child -and !$child.HasExited){Stop-Process -Id $child.Id -Force;$child.WaitForExit()}}
}
