#requires -Version 5.1
[CmdletBinding()]
param()
Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'
$source = Split-Path -Parent $PSScriptRoot
$testRoot = 'E:\Codex\temp\edus-portable-script-tests-' + [Guid]::NewGuid().ToString('N')
$null = New-Item -ItemType Directory -Path $testRoot
$savedTemp=$env:TEMP;$savedTmp=$env:TMP;$env:TEMP=$testRoot;$env:TMP=$testRoot
$results = New-Object Collections.Generic.List[object]
function Check([string]$Name,[scriptblock]$Code) {
    try { & $Code; $results.Add([pscustomobject]@{name=$Name;result='PASS'}) }
    catch { $results.Add([pscustomobject]@{name=$Name;result='FAIL';detail=$_.Exception.Message}); throw }
}
function Expect-Reject([scriptblock]$Code) { $caught=$false;try { & $Code | Out-Null } catch { $caught=$true };if(-not $caught){throw 'Expected rejection'} }
function Assert([bool]$Value,[string]$Message) {if(-not $Value){throw $Message}}
function Build-Fixture([string]$Root) {
    $files = @('backend/EDUSLibraryService.exe','backend/EDUSTerminalConfigurator.exe','backend/EDUSInstallerHelper.exe','frontend/index.html','frontend/manifest.json','deployment.json','scripts/Edus.Deployment.psm1','scripts/Setup-EDUS.ps1','scripts/Verify-EDUS.ps1','scripts/Repair-EDUS.ps1','scripts/Remove-EDUS.ps1')
    $entries=@()
    foreach($relative in $files){$path=Join-Path $Root $relative;$null=New-Item -ItemType Directory -Path (Split-Path -Parent $path) -Force;[IO.File]::WriteAllText($path,('fixture '+$relative));$item=Get-Item -LiteralPath $path;$entries += @{path=$relative;size=$item.Length;sha256=(Get-FileHash -LiteralPath $path).Hash.ToLowerInvariant()}}
    $manifest=@{schema_version=1;product='EDUS-Library-Portable';package_version='2.0.0-rc.1';frontend_version='2.0.0-rc.1';backend_version='2.0.0-rc.1';local_api_version='1';files=$entries}
    [IO.File]::WriteAllText((Join-Path $Root 'manifest.json'),($manifest|ConvertTo-Json -Depth 8))
}
try {
    Check 'PowerShell parser (all shipping scripts)' {
        foreach($file in Get-ChildItem -LiteralPath (Join-Path $source 'scripts') -File){$tokens=$null;$errors=$null;$null=[Management.Automation.Language.Parser]::ParseFile($file.FullName,[ref]$tokens,[ref]$errors);Assert ($errors.Count -eq 0) ($errors|Out-String)}
    }
    $module = Import-Module (Join-Path $source 'scripts\Edus.Deployment.psm1') -Force -PassThru
    $package=Join-Path $testRoot 'package';Build-Fixture $package
    Check 'Manifest valid real files' { $null=Test-EdusPackage $package }
    Check 'Tampered payload rejected' { $file=Join-Path $package 'frontend\index.html';$original=[IO.File]::ReadAllText($file);[IO.File]::WriteAllText($file,'tampered');Expect-Reject {Test-EdusPackage $package};[IO.File]::WriteAllText($file,$original) }
    Check 'Unknown payload rejected' { $file=Join-Path $package 'secrets.dpapi';[IO.File]::WriteAllText($file,'test');Expect-Reject {Test-EdusPackage $package};Remove-Item -LiteralPath $file }
    Check 'Traversal and alternate stream rejected' { foreach($relative in @('../secrets','backend/../../secrets','backend/x:secret','C:/Windows/test','backend\bad')){Expect-Reject {Get-EdusChild $package $relative}} }
    Check 'SCM native argument escaping preserves ImagePath quotes' {
        $value='"C:\Program Files\EDUS Library\backend\EDUSLibraryService.exe"'
        $quoted=ConvertTo-EdusNativeArgument $value
        Assert ($quoted -ceq '"\"C:\Program Files\EDUS Library\backend\EDUSLibraryService.exe\""') 'ImagePath quotes would be stripped'
        Assert ((ConvertTo-EdusNativeArgument 'C:\trailing\') -ceq '"C:\trailing\\"') 'Trailing backslash not escaped'
    }
    Check 'In-memory ACL grants service RX on binaries, FullControl only on data' {
        $sid='S-1-5-80-123-456-789-123-456'
        $installAcl=New-EdusDirectoryAcl $sid -PublicRead
        Assert-EdusInstallAclRules $installAcl $sid
        $dataAcl=New-EdusDirectoryAcl $sid
        $serviceRule=@($dataAcl.GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier])|Where-Object {$_.IdentityReference.Value -eq $sid})
        Assert ($serviceRule[0].FileSystemRights -eq [Security.AccessControl.FileSystemRights]::FullControl) 'Data service rights missing'
        $null=$installAcl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule((New-Object Security.Principal.SecurityIdentifier($sid)),'FullControl','ContainerInherit,ObjectInherit','None','Allow')))
        Expect-Reject {Assert-EdusInstallAclRules $installAcl $sid}
    }
    Check 'Junction rejected before recursive traversal' {
        $target=Join-Path $testRoot 'junction-target';$null=New-Item -ItemType Directory -Path $target
        $junction=Join-Path $package 'unsafe-junction';$null=New-Item -ItemType Junction -Path $junction -Target $target
        Expect-Reject {Test-EdusPackage $package}
        # Remove the link itself; never recurse through its destination.
        [IO.Directory]::Delete($junction)
    }
    $install=Join-Path $testRoot 'Install With Spaces';$data=Join-Path $testRoot 'ProgramData';$null=New-Item -ItemType Directory -Path $data
    [IO.File]::WriteAllText((Join-Path $data 'library.db'),'database preserved');[IO.File]::WriteAllText((Join-Path $data 'secrets.dpapi'),'key preserved')
    $dataHash=(Get-FileHash -LiteralPath (Join-Path $data 'library.db')).Hash;$keyHash=(Get-FileHash -LiteralPath (Join-Path $data 'secrets.dpapi')).Hash
    Check 'Atomic activation failure restores previous directory' {
        $null=New-Item -ItemType Directory -Path $install;[IO.File]::WriteAllText((Join-Path $install 'old.marker'),'old')
        $stage=New-EdusStaging $package $install;Expect-Reject {Switch-EdusStaging $stage $install {throw 'injected activate failure'}}
        Assert (Test-Path -LiteralPath (Join-Path $install 'old.marker')) 'Prior installation lost'
        # Delete only this checked test fixture, not application data.
        Assert ($install.StartsWith($testRoot+'\')) 'Unsafe fixture root';Remove-Item -LiteralPath $install -Recurse -Force
    }
    $savedProgramFiles=$env:ProgramFiles;$savedX86=${env:ProgramFiles(x86)}
    $env:ProgramFiles=Join-Path $testRoot 'FakeProgramFiles';${env:ProgramFiles(x86)}=$env:ProgramFiles
    $edge=Join-Path $env:ProgramFiles 'Microsoft\Edge\Application\msedge.exe';$null=New-Item -ItemType Directory -Path (Split-Path -Parent $edge) -Force;[IO.File]::WriteAllText($edge,'fixture')
    # Mocks replace native side effects only; actual setup/staging/manifest/remove code runs.
    & $module {
        param($Install,$Data)
        $script:FixtureRoots=@{Install=$Install;Data=$Data};$script:Exists=$false;$script:State=1;$script:Recorded=$false;$script:Calls=New-Object Collections.Generic.List[string]
        function script:Assert-EdusAdministrator {}
        function script:Get-EdusRoots {return $script:FixtureRoots}
        function script:Get-EdusInstallRecord {if($script:Recorded){return [pscustomobject]@{Product='EDUS-Library-Portable';InstallRoot=$script:FixtureRoots.Install}}}
        function script:Write-EdusInstallRecord {param($InstallRoot);$script:Recorded=$true;$script:Calls.Add('record')}
        function script:Remove-EdusInstallRecord {$script:Recorded=$false}
        function script:Invoke-EdusHelper {
            param($Helper,$Operation,$Backend,$Log)
            $script:Calls.Add($Operation)
            if($Operation -eq 'stop'){$script:State=1}
            if($Operation -eq 'inspect'){return [pscustomobject]@{service=[pscustomobject]@{exists=$script:Exists;account='NT AUTHORITY\LocalService';state=$script:State}}}
        }
        function script:Invoke-EdusSc {
            param($Arguments,$Allowed)
            $script:Calls.Add('sc-'+$Arguments[0])
            switch($Arguments[0]){'create'{$script:Exists=$true};'start'{$script:State=4};'delete'{$script:Exists=$false;$script:State=1}}
        }
        function script:Get-EdusServiceSid {return 'S-1-5-80-123-456-789-123-456'}
        function script:Set-EdusDataAcl {param($Root,$ServiceSid);$script:Calls.Add('data-acl')}
        function script:Set-EdusDirectoryAcl {param($Path,$ServiceSid,[switch]$PublicRead);$script:Calls.Add('install-acl')}
        function script:Set-EdusShortcut {param($Backend)}
        function script:Remove-EdusShortcut {}
        function script:Test-EdusInstallation {param($PackageRoot,$Log);return [pscustomobject]@{Check='MockNativeVerification';Result='PASS';Detail='Mock only, NOT SCM acceptance'}}
    } $install $data
    Check 'Setup absent service (mock SCM, real file activation)' {Invoke-EdusSetup $package;Assert (Test-Path -LiteralPath (Join-Path $install 'backend\EDUSLibraryService.exe')) 'No service binary';$calls=&$module {$script:Calls.ToArray()};Assert ($calls -contains 'sc-create') 'No create';Assert ([Array]::IndexOf($calls,'preflight') -lt [Array]::IndexOf($calls,'stop')) 'Wrong order'}
    Check 'Repeated setup running service (mock SCM)' {&$module {$script:Calls.Clear()};Invoke-EdusSetup $package;$calls=&$module {$script:Calls.ToArray()};Assert ($calls -contains 'sc-config') 'No update';Assert ($calls -notcontains 'sc-create') 'Recreated service'}
    Check 'Stopped service repair restores missing asset (mock SCM)' {&$module {$script:State=1};Remove-Item -LiteralPath (Join-Path $install 'frontend\index.html');Invoke-EdusSetup $package;Assert (Test-Path -LiteralPath (Join-Path $install 'frontend\index.html')) 'Asset not restored'}
    Check 'Unexpected database under binaries prevents move/delete' {
        $file=Join-Path $install 'unexpected.db';[IO.File]::WriteAllText($file,'must survive')
        Expect-Reject {Invoke-EdusSetup $package};Expect-Reject {Invoke-EdusRemove $package}
        Assert (Test-Path -LiteralPath $file) 'Unexpected DB deleted';Remove-Item -LiteralPath $file
    }
    Check 'Setup preserves data and secrets bytes' {Assert ((Get-FileHash -LiteralPath (Join-Path $data 'library.db')).Hash -eq $dataHash) 'DB changed';Assert ((Get-FileHash -LiteralPath (Join-Path $data 'secrets.dpapi')).Hash -eq $keyHash) 'Secrets changed'}
    Check 'Default remove preserves data (mock SCM)' {Invoke-EdusRemove $package;Assert (-not(Test-Path -LiteralPath $install)) 'Install left';Assert ((Get-FileHash -LiteralPath (Join-Path $data 'library.db')).Hash -eq $dataHash) 'DB removed';Assert ((Get-FileHash -LiteralPath (Join-Path $data 'secrets.dpapi')).Hash -eq $keyHash) 'Secrets removed'}
} finally {
    $env:TEMP=$savedTemp;$env:TMP=$savedTmp
    if(Get-Variable -Name savedProgramFiles -ErrorAction SilentlyContinue){$env:ProgramFiles=$savedProgramFiles;${env:ProgramFiles(x86)}=$savedX86}
    $results | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $testRoot 'results.json') -Encoding UTF8
    $results | Format-Table -AutoSize
    Write-Host "Evidence: $testRoot\results.json"
    Write-Host 'SCM/LocalService/ACL/clean installation NOT EXECUTED; native calls mocked. Test files retained on E.'
}
