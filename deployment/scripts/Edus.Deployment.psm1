Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$script:ServiceName = 'EDUSLibraryService'

function Assert-EdusPath {
    param([Parameter(Mandatory=$true)][string]$Path)
    $full = [IO.Path]::GetFullPath($Path)
    if ($full -notmatch '^[A-Za-z]:\\' -or $full.Contains('..') -or $full.Contains('::')) { throw 'PATH_INVALID' }
    $part = $full
    while ($part) {
        if (Test-Path -LiteralPath $part) {
            $item = Get-Item -LiteralPath $part -Force
            if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw "REPARSE_POINT_REJECTED: $part" }
        }
        $parent = [IO.Path]::GetDirectoryName($part)
        if ($parent -eq $part) { break }; $part = $parent
    }
    return $full.TrimEnd('\')
}

function Get-EdusChild {
    param([string]$Root,[string]$Relative)
    if ([string]::IsNullOrWhiteSpace($Relative) -or $Relative -match '(^/|\\|:|(^|/)\.\.?(/|$))') { throw "MANIFEST_PATH_INVALID: $Relative" }
    $base = Assert-EdusPath $Root
    $child = Assert-EdusPath (Join-Path $base $Relative.Replace('/','\'))
    if (-not $child.StartsWith($base + '\',[StringComparison]::OrdinalIgnoreCase)) { throw 'PATH_ESCAPES_ROOT' }
    return $child
}

function Test-EdusPackage {
    param([Parameter(Mandatory=$true)][Alias('PackageRoot')][string]$Root)
    $base = Assert-EdusPath $Root
    $manifestPath = Join-Path $base 'manifest.json'
    if ((Get-Item -LiteralPath $manifestPath).Length -gt 4MB) { throw 'MANIFEST_TOO_LARGE' }
    $manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($manifest.schema_version -ne 1 -or $manifest.product -ne 'EDUS-Library-Portable' -or $manifest.package_version -ne '2.0.0-rc.1' -or $manifest.local_api_version -ne '1') { throw 'PACKAGE_INCOMPATIBLE' }
    $seen = New-Object 'Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase)
    foreach ($file in $manifest.files) {
        if (-not $seen.Add($file.path) -or $file.path -eq 'manifest.json' -or $file.sha256 -notmatch '^[a-fA-F0-9]{64}$') { throw 'MANIFEST_ENTRY_INVALID' }
        $path = Get-EdusChild $base $file.path
        $item = Get-Item -LiteralPath $path -Force
        if ($item.PSIsContainer -or $item.Length -ne $file.size -or (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ne $file.sha256) { throw "PACKAGE_HASH_MISMATCH: $($file.path)" }
    }
    foreach ($required in @('backend/EDUSLibraryService.exe','backend/EDUSTerminalConfigurator.exe','backend/EDUSInstallerHelper.exe','frontend/index.html','frontend/manifest.json','deployment.json','scripts/Edus.Deployment.psm1','scripts/Setup-EDUS.ps1','scripts/Verify-EDUS.ps1','scripts/Repair-EDUS.ps1','scripts/Remove-EDUS.ps1')) {
        if (-not $seen.Contains($required)) { throw "PACKAGE_FILE_MISSING: $required" }
    }
    foreach ($item in Get-EdusTree $base) {
        $null = Assert-EdusPath $item.FullName
        if (-not $item.PSIsContainer) {
            $relative = $item.FullName.Substring($base.Length + 1).Replace('\','/')
            if ($relative -ne 'manifest.json' -and -not $seen.Contains($relative)) { throw "PACKAGE_UNLISTED_FILE: $relative" }
        }
    }
    return $manifest
}

function Get-EdusTree {
    param([string]$Root)
    $pending = New-Object 'Collections.Generic.Stack[string]'
    $pending.Push((Assert-EdusPath $Root)); $count = 0
    while ($pending.Count -gt 0) {
        foreach ($item in Get-ChildItem -LiteralPath $pending.Pop() -Force) {
            $null = Assert-EdusPath $item.FullName
            $count++; if ($count -gt 1000000) { throw 'DIRECTORY_ENTRY_LIMIT' }
            Write-Output $item
            if ($item.PSIsContainer) { $pending.Push($item.FullName) }
        }
    }
}

function Assert-EdusBinaryRoot {
    param([string]$Root)
    if (-not (Test-Path -LiteralPath $Root)) { return }
    foreach ($item in Get-EdusTree $Root) {
        if (-not $item.PSIsContainer -and ($item.Name -match '(?i)(\.(db|sqlite|sqlite3|dpapi)$|-(wal|shm)$)')) {
            throw "DATA_FOUND_IN_BINARY_DIRECTORY: $($item.FullName). Preserve the directory and request controlled migration."
        }
    }
}

function Assert-EdusAdministrator {
    if (-not [Environment]::Is64BitOperatingSystem -or -not [Environment]::Is64BitProcess) { throw 'Use 64-bit Windows PowerShell on x64 Windows.' }
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    try {
        $principal = New-Object Security.Principal.WindowsPrincipal($identity)
        if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { throw 'Run Windows PowerShell as Administrator.' }
    } finally { $identity.Dispose() }
}

function Get-EdusRoots {
    $programFiles = [Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFiles)
    $programData = [Environment]::GetFolderPath([Environment+SpecialFolder]::CommonApplicationData)
    return @{ Install = (Assert-EdusPath (Join-Path $programFiles 'EDUS Library')); Data = (Assert-EdusPath (Join-Path $programData 'EDUS Library')) }
}

function Invoke-EdusHelper {
    param([string]$Helper,[ValidateSet('preflight','stop','health','inspect')][string]$Operation,[string]$Backend,[string]$Log)
    $output = @(& $Helper $Operation $Backend $Log 2>&1)
    $code = $LASTEXITCODE
    if ($code -ne 0) { throw "HELPER_FAILED operation=$Operation exit=$code details=$($output -join ' ') log=$Log" }
    if ($Operation -eq 'inspect') { return ($output -join "`n" | ConvertFrom-Json) }
}

function Invoke-EdusSc {
    param([string[]]$Arguments,[int[]]$Allowed=@(0))
    # Windows PowerShell 5.1 native binding can strip the literal ImagePath quotes.
    # Explicit CRT argument quoting preserves the SCM value including its quotes.
    $info = New-Object Diagnostics.ProcessStartInfo
    $info.FileName = "$env:SystemRoot\System32\sc.exe"
    $info.Arguments = ($Arguments | ForEach-Object { ConvertTo-EdusNativeArgument $_ }) -join ' '
    $info.UseShellExecute = $false; $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true; $info.RedirectStandardError = $true
    $process = New-Object Diagnostics.Process
    $process.StartInfo = $info
    try {
        if (-not $process.Start()) { throw 'SCM_PROCESS_NOT_STARTED' }
        $stdout = $process.StandardOutput.ReadToEndAsync(); $stderr = $process.StandardError.ReadToEndAsync()
        if (-not $process.WaitForExit(120000)) { $process.Kill(); throw 'SCM_PROCESS_TIMEOUT' }
        $code = $process.ExitCode
        $output = $stdout.Result + $stderr.Result
        if ($Allowed -notcontains $code) { throw "SCM_FAILED operation=$($Arguments[0]) process_exit_code=$code details=$output" }
    } finally { $process.Dispose() }
}

function ConvertTo-EdusNativeArgument {
    param([AllowEmptyString()][string]$Value)
    # Quote each argument; double runs of backslashes before quotes and the closing quote.
    return '"' + ([regex]::Replace([regex]::Replace($Value,'(\\*)"','$1$1\"'),'(\\+)$','$1$1')) + '"'
}

function New-EdusStaging {
    param([string]$PackageRoot,[string]$InstallRoot)
    $null = Test-EdusPackage $PackageRoot
    $root = Assert-EdusPath $InstallRoot
    $stage = $root + '.stage-' + [Guid]::NewGuid().ToString('N')
    $null = Assert-EdusPath $stage
    $null = New-Item -ItemType Directory -Path $stage
    foreach ($entry in Get-ChildItem -LiteralPath $PackageRoot -Force) { Copy-Item -LiteralPath $entry.FullName -Destination $stage -Recurse }
    $null = Test-EdusPackage $stage
    return $stage
}

function Switch-EdusStaging {
    param([string]$Stage,[string]$InstallRoot,[scriptblock]$BeforeActivate)
    $root = Assert-EdusPath $InstallRoot
    $stagePath = Assert-EdusPath $Stage
    if (-not $stagePath.StartsWith($root + '.stage-',[StringComparison]::OrdinalIgnoreCase)) { throw 'STAGING_PATH_INVALID' }
    $null = Test-EdusPackage $stagePath
    $previous = $root + '.previous-' + [Guid]::NewGuid().ToString('N')
    $moved = $false
    try {
        if (Test-Path -LiteralPath $root) { Move-Item -LiteralPath $root -Destination $previous; $moved = $true }
        if ($BeforeActivate) { & $BeforeActivate }
        Move-Item -LiteralPath $stagePath -Destination $root
    } catch {
        if ($moved -and -not (Test-Path -LiteralPath $root)) { Move-Item -LiteralPath $previous -Destination $root }
        throw
    }
    # Preserve previous binaries for deliberate recovery. Never roll database migrations back.
    return $previous
}

function Set-EdusDirectoryAcl {
    param([string]$Path,[string]$ServiceSid,[switch]$PublicRead)
    $null = Assert-EdusPath $Path
    $acl = New-EdusDirectoryAcl $ServiceSid -PublicRead:$PublicRead
    Set-Acl -LiteralPath $Path -AclObject $acl
}

function New-EdusDirectoryAcl {
    param([string]$ServiceSid,[switch]$PublicRead)
    $acl = New-Object Security.AccessControl.DirectorySecurity
    $acl.SetAccessRuleProtection($true,$false)
    $acl.SetOwner((New-Object Security.Principal.SecurityIdentifier('S-1-5-32-544')))
    $principals = @('S-1-5-18','S-1-5-32-544')
    foreach ($sid in $principals) {
        $rule = New-Object Security.AccessControl.FileSystemAccessRule((New-Object Security.Principal.SecurityIdentifier($sid)), 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
        $null = $acl.AddAccessRule($rule)
    }
    $serviceRights = if ($PublicRead) { 'ReadAndExecute' } else { 'FullControl' }
    $null = $acl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule((New-Object Security.Principal.SecurityIdentifier($ServiceSid)), $serviceRights, 'ContainerInherit,ObjectInherit', 'None', 'Allow')))
    if ($PublicRead) {
        $null = $acl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule((New-Object Security.Principal.SecurityIdentifier('S-1-5-32-545')), 'ReadAndExecute', 'ContainerInherit,ObjectInherit', 'None', 'Allow')))
    }
    return $acl
}

function Get-EdusServiceSid {
    return (New-Object Security.Principal.NTAccount('NT SERVICE','EDUSLibraryService')).Translate([Security.Principal.SecurityIdentifier]).Value
}

function Set-EdusDataAcl {
    param([string]$Root,[string]$ServiceSid)
    # Reject reparse points before any recursive ACL mutation. Existing data are preserved.
    $entries = @(Get-EdusTree $Root)
    Set-EdusDirectoryAcl -Path $Root -ServiceSid $ServiceSid
    foreach ($item in $entries) {
        if ($item.PSIsContainer) { Set-EdusDirectoryAcl -Path $item.FullName -ServiceSid $ServiceSid }
        else {
            $acl = New-Object Security.AccessControl.FileSecurity
            $acl.SetAccessRuleProtection($true,$false)
            $acl.SetOwner((New-Object Security.Principal.SecurityIdentifier('S-1-5-32-544')))
            foreach ($sid in @('S-1-5-18','S-1-5-32-544',$ServiceSid)) {
                $null = $acl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule((New-Object Security.Principal.SecurityIdentifier($sid)), 'FullControl', 'Allow')))
            }
            Set-Acl -LiteralPath $item.FullName -AclObject $acl
        }
    }
}

function Get-EdusInstallRecord { return (Get-ItemProperty -LiteralPath 'HKLM:\Software\EDUS\Library\Portable' -ErrorAction SilentlyContinue) }
function Write-EdusInstallRecord {
    param([string]$InstallRoot)
    $key = 'HKLM:\Software\EDUS\Library\Portable'
    $null = New-Item -Path $key -Force
    $null = New-ItemProperty -Path $key -Name Product -Value 'EDUS-Library-Portable' -PropertyType String -Force
    $null = New-ItemProperty -Path $key -Name InstallRoot -Value $InstallRoot -PropertyType String -Force
    $null = New-ItemProperty -Path $key -Name LayoutVersion -Value 1 -PropertyType DWord -Force
}
function Remove-EdusInstallRecord { Remove-Item -LiteralPath 'HKLM:\Software\EDUS\Library\Portable' -Recurse }
function Set-EdusShortcut {
    param([string]$Backend)
    $shell = New-Object -ComObject WScript.Shell
    try {
        $shortcut = $shell.CreateShortcut((Join-Path ([Environment]::GetFolderPath('CommonPrograms')) 'EDUS Terminal Configurator.lnk'))
        $shortcut.TargetPath = Join-Path $Backend 'EDUSTerminalConfigurator.exe'
        $shortcut.WorkingDirectory = $Backend; $shortcut.Save()
    } finally { $null = [Runtime.InteropServices.Marshal]::FinalReleaseComObject($shell) }
}
function Remove-EdusShortcut {
    $shortcut = Join-Path ([Environment]::GetFolderPath('CommonPrograms')) 'EDUS Terminal Configurator.lnk'
    if (Test-Path -LiteralPath $shortcut) { Remove-Item -LiteralPath $shortcut }
}

function Invoke-EdusSetup {
    param([string]$PackageRoot)
    Assert-EdusAdministrator
    $manifest = Test-EdusPackage $PackageRoot
    $roots = Get-EdusRoots
    if ($PackageRoot.TrimEnd('\') -ieq $roots.Install) { throw 'Run Setup/Repair from the extracted ZIP, not from the active installation.' }
    $edgePaths = @((Join-Path ${env:ProgramFiles(x86)} 'Microsoft\Edge\Application\msedge.exe'),(Join-Path $env:ProgramFiles 'Microsoft\Edge\Application\msedge.exe'))
    if (-not @($edgePaths | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf }).Count) { throw 'MICROSOFT_EDGE_MISSING: Install Microsoft Edge before Setup.' }
    $size = ($manifest.files | Measure-Object -Property size -Sum).Sum
    foreach ($path in @($roots.Install,$roots.Data)) {
        $drive = New-Object IO.DriveInfo([IO.Path]::GetPathRoot($path))
        if ($drive.AvailableFreeSpace -lt ($size * 3 + 512MB)) { throw "INSUFFICIENT_DISK_SPACE: $($drive.Name)" }
    }
    $run = [Guid]::NewGuid().ToString('N')
    $log = Join-Path ([IO.Path]::GetTempPath()) "EDUS-Setup-$run.jsonl"
    $helper = Join-Path $PackageRoot 'backend\EDUSInstallerHelper.exe'
    $backend = Join-Path $roots.Install 'backend'
    Invoke-EdusHelper $helper preflight $backend $log
    $inspection = Invoke-EdusHelper $helper inspect $backend $log
    if ($inspection.service.exists -and $inspection.service.account -notin @('NT AUTHORITY\LocalService','NT AUTHORITY\LOCAL SERVICE')) { throw 'SERVICE_IDENTITY_MISMATCH: DPAPI identity must not change.' }
    if ((Test-Path -LiteralPath $roots.Install) -and -not $inspection.service.exists) {
        $record = Get-EdusInstallRecord
        if (-not $record -or $record.Product -ne 'EDUS-Library-Portable' -or $record.InstallRoot -ine $roots.Install) { throw 'UNOWNED_INSTALL_DIRECTORY: manual diagnosis required; directory preserved.' }
    }
    Assert-EdusBinaryRoot $roots.Install
    $stage = New-EdusStaging $PackageRoot $roots.Install
    Invoke-EdusHelper $helper stop $backend $log
    $previous = Switch-EdusStaging $stage $roots.Install
    Write-Host "Previous binaries retained: $previous"
    Write-EdusInstallRecord $roots.Install
    $command = '"' + (Join-Path $backend 'EDUSLibraryService.exe') + '"'
    if ($inspection.service.exists) { Invoke-EdusSc @('config',$script:ServiceName,'binPath=',$command,'start=','auto') }
    else { Invoke-EdusSc @('create',$script:ServiceName,'binPath=',$command,'start=','auto','obj=','NT AUTHORITY\LocalService','DisplayName=','EDUS Library Local Service') }
    Invoke-EdusSc @('description',$script:ServiceName,'Локальная защищённая служба библиотечного терминала EDUS.')
    Invoke-EdusSc @('sidtype',$script:ServiceName,'unrestricted')
    Invoke-EdusSc @('failure',$script:ServiceName,'reset=','86400','actions=','restart/10000/restart/30000/restart/60000')
    $serviceSid = Get-EdusServiceSid
    foreach ($relative in @('','production','production\logs','production\backups','uat','uat\logs','uat\backups','logs','backups')) {
        $path = if ($relative) { Join-Path $roots.Data $relative } else { $roots.Data }
        $null = Assert-EdusPath $path
        $null = New-Item -ItemType Directory -Path $path -Force
    }
    Set-EdusDataAcl $roots.Data $serviceSid
    Set-EdusDirectoryAcl $roots.Install $serviceSid -PublicRead
    Set-EdusShortcut $backend
    Invoke-EdusSc @('start',$script:ServiceName) @(0,1056)
    Invoke-EdusHelper $helper health $backend $log
    $results = @(Test-EdusInstallation -PackageRoot $roots.Install -Log $log)
    $results | Format-Table -AutoSize | Out-Host
    if (@($results | Where-Object { $_.Result -eq 'FAIL' }).Count) { throw "VERIFY_FAILED. Data preserved. Log: $log" }
    Write-Host "EDUS установлен. http://127.0.0.1:43180. Журнал: $log"
    Write-Host 'Windows Settings → Assigned Access → Microsoft Edge → Digital / Interactive signage → http://127.0.0.1:43180'
}

function Test-EdusAcl {
    param([string]$Root,[string]$ServiceSid)
    $allowed = @('S-1-5-18','S-1-5-32-544',$ServiceSid)
    foreach ($item in @((Get-Item -LiteralPath $Root -Force)) + @(Get-EdusTree $Root)) {
        $null = Assert-EdusPath $item.FullName
        $acl = Get-Acl -LiteralPath $item.FullName
        foreach ($rule in $acl.GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier])) {
            if ($rule.AccessControlType -eq 'Allow' -and $allowed -notcontains $rule.IdentityReference.Value) { throw "ACL_UNEXPECTED_PRINCIPAL: $($item.FullName)" }
        }
        foreach ($sid in $allowed) {
            if (-not @($acl.GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier]) | Where-Object { $_.IdentityReference.Value -eq $sid -and $_.AccessControlType -eq 'Allow' -and ($_.FileSystemRights -band [Security.AccessControl.FileSystemRights]::FullControl) -eq [Security.AccessControl.FileSystemRights]::FullControl }).Count) { throw "ACL_REQUIRED_ACCESS_MISSING: $($item.FullName)" }
        }
    }
}

function Test-EdusInstallAcl {
    param([string]$Root,[string]$ServiceSid)
    foreach ($item in @((Get-Item -LiteralPath $Root -Force)) + @(Get-EdusTree $Root)) {
        $null = Assert-EdusPath $item.FullName
        Assert-EdusInstallAclRules (Get-Acl -LiteralPath $item.FullName) $ServiceSid
    }
}

function Assert-EdusInstallAclRules {
    param([Security.AccessControl.FileSystemSecurity]$Acl,[string]$ServiceSid)
    $write = [Security.AccessControl.FileSystemRights]::Write -bor [Security.AccessControl.FileSystemRights]::Delete -bor [Security.AccessControl.FileSystemRights]::ChangePermissions -bor [Security.AccessControl.FileSystemRights]::TakeOwnership -bor [Security.AccessControl.FileSystemRights]::DeleteSubdirectoriesAndFiles
    $rules = @($Acl.GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier]))
    foreach ($rule in $rules) {
        if ($rule.AccessControlType -eq 'Allow' -and $rule.IdentityReference.Value -notin @('S-1-5-18','S-1-5-32-544') -and ($rule.FileSystemRights -band $write) -ne 0) { throw 'INSTALL_ACL_UNPRIVILEGED_WRITE' }
    }
    foreach ($sid in @($ServiceSid,'S-1-5-32-545')) {
        if (-not @($rules | Where-Object { $_.AccessControlType -eq 'Allow' -and $_.IdentityReference.Value -eq $sid -and ($_.FileSystemRights -band [Security.AccessControl.FileSystemRights]::ReadAndExecute) -eq [Security.AccessControl.FileSystemRights]::ReadAndExecute }).Count) { throw 'INSTALL_ACL_READ_EXECUTE_MISSING' }
    }
}

function Test-EdusInstallation {
    param([string]$PackageRoot,[string]$Log)
    $roots = Get-EdusRoots
    $helper = Join-Path $PackageRoot 'backend\EDUSInstallerHelper.exe'
    $context = @{ Inspection=$null; Manifest=$null }
    $checks = [ordered]@{
        FilesAndHashes = { $context.Manifest = Test-EdusPackage $roots.Install }
        ServiceRegistration = {
            $context.Inspection = Invoke-EdusHelper $helper inspect (Join-Path $roots.Install 'backend') $Log
            $svc = $context.Inspection.service
            if (-not $svc.exists -or $svc.executable_path -ine (Join-Path $roots.Install 'backend\EDUSLibraryService.exe')) { throw 'SERVICE_PATH_MISMATCH' }
        }
        ServiceIdentity = { if ($context.Inspection.service.account -notin @('NT AUTHORITY\LocalService','NT AUTHORITY\LOCAL SERVICE')) { throw 'LOCAL_SERVICE_REQUIRED' } }
        StartupAndSid = { if ($context.Inspection.service.startup_type -ne 2 -or $context.Inspection.service.sid_type -ne 1) { throw 'AUTOMATIC_AND_SERVICE_SID_REQUIRED' } }
        Recovery = {
            $recovery = $context.Inspection.service.recovery
            if ($recovery.reset_seconds -ne 86400 -or $recovery.actions.Count -ne 3) { throw 'RECOVERY_INVALID' }
            $delays = @(10000,30000,60000)
            for ($i=0;$i -lt 3;$i++) { if ($recovery.actions[$i].type -ne 1 -or $recovery.actions[$i].delay_ms -ne $delays[$i]) { throw 'RECOVERY_INVALID' } }
        }
        ProgramDataAcl = { Test-EdusAcl $roots.Data (Get-EdusServiceSid) }
        InstallationAcl = { Test-EdusInstallAcl $roots.Install (Get-EdusServiceSid) }
        LoopbackListener = {
            $i = $context.Inspection
            if ($i.service.state -ne 4 -or $i.listeners.Count -ne 1 -or $i.listeners[0].address -ne '127.0.0.1' -or $i.listeners[0].port -ne 43180 -or $i.listeners[0].pid -ne $i.service.pid) { throw 'LISTENER_OR_SCM_MISMATCH' }
        }
        Health = { Invoke-EdusHelper $helper health (Join-Path $roots.Install 'backend') $Log }
        FrontendCompatibility = {
            $front = Get-Content -LiteralPath (Join-Path $roots.Install 'frontend\manifest.json') -Raw -Encoding UTF8 | ConvertFrom-Json
            $version = Invoke-RestMethod -Uri 'http://127.0.0.1:43180/api/local/v1/version' -TimeoutSec 10
            if ($version.local_api_version -ne $front.required_local_api_version -or $front.frontend_version -ne $context.Manifest.frontend_version -or $version.backend_version -ne $context.Manifest.backend_version -or $front.frontend_version -ne $version.minimum_frontend_version -or $front.frontend_version -ne $version.maximum_frontend_version) { throw 'VERSION_INCOMPATIBLE' }
        }
    }
    foreach ($name in $checks.Keys) {
        try { & $checks[$name]; [pscustomobject]@{ Check=$name; Result='PASS'; Detail='' } }
        catch { [pscustomobject]@{ Check=$name; Result='FAIL'; Detail=$_.Exception.Message } }
    }
}

function Invoke-EdusRemove {
    param([string]$PackageRoot,[switch]$RemoveData)
    Assert-EdusAdministrator
    $null = Test-EdusPackage $PackageRoot
    $roots = Get-EdusRoots
    if ($PackageRoot.TrimEnd('\') -ieq $roots.Install) { throw 'Run Remove from the extracted ZIP so its running helper can be preserved.' }
    $helper = Join-Path $PackageRoot 'backend\EDUSInstallerHelper.exe'
    $log = Join-Path ([IO.Path]::GetTempPath()) ('EDUS-Remove-' + [Guid]::NewGuid().ToString('N') + '.jsonl')
    Invoke-EdusHelper $helper preflight (Join-Path $roots.Install 'backend') $log
    $record = Get-EdusInstallRecord
    if (-not $record -or $record.Product -ne 'EDUS-Library-Portable' -or $record.InstallRoot -ine $roots.Install) { throw 'PORTABLE_OWNERSHIP_REQUIRED' }
    Assert-EdusBinaryRoot $roots.Install
    Invoke-EdusHelper $helper stop (Join-Path $roots.Install 'backend') $log
    $inspection = Invoke-EdusHelper $helper inspect (Join-Path $roots.Install 'backend') $log
    if ($inspection.service.exists -and $inspection.service.state -ne 1) { throw 'SERVICE_NOT_STOPPED' }
    Invoke-EdusSc @('delete',$script:ServiceName) @(0,1060)
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    do {
        $inspection = Invoke-EdusHelper $helper inspect (Join-Path $roots.Install 'backend') $log
        if (-not $inspection.service.exists) { break }; Start-Sleep -Milliseconds 250
    } while ([DateTime]::UtcNow -lt $deadline)
    if ($inspection.service.exists) { throw 'SERVICE_DELETION_PENDING: Close service management tools and retry. Files preserved.' }
    # Exact product roots, checked immediately before recursive deletion.
    $null = Assert-EdusPath $roots.Install
    $null = @(Get-EdusTree $roots.Install)
    Remove-Item -LiteralPath $roots.Install -Recurse -Force
    Remove-EdusInstallRecord
    Remove-EdusShortcut
    if ($RemoveData) {
        Write-Warning "All EDUS production/UAT databases, keys, outbox and backups at $($roots.Data) will be permanently deleted."
        if ((Read-Host 'Type DELETE EDUS DATA to confirm') -cne 'DELETE EDUS DATA') { Write-Host 'Data deletion cancelled; ProgramData preserved.'; return }
        $null = Assert-EdusPath $roots.Data
        $null = @(Get-EdusTree $roots.Data)
        Remove-Item -LiteralPath $roots.Data -Recurse -Force
    } else { Write-Host "ProgramData preserved: $($roots.Data)" }
}

Export-ModuleMember -Function *-Edus*
