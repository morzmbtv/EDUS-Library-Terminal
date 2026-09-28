#requires -Version 7.0
[CmdletBinding()]
param(
    [string]$Frontend = 'E:\Codex\projects\edus-library-frontend',
    [string]$Backend = 'E:\Codex\projects\edus-library-backend',
    [string]$ArtifactDirectory = 'E:\Codex\artifacts',
    [string]$CargoTarget = 'E:\Codex\cache\edus-library-edge-fix2\target',
    [string]$VcRuntime = 'E:\Codex\tools\VSBuildTools\VC\Redist\MSVC\14.51.36231\x64\Microsoft.VC145.CRT'
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$version = '2.0.0-rc.1'
$run = (Get-Date).ToUniversalTime().ToString('yyyyMMddTHHmmssfffZ')
$work = Join-Path $PSScriptRoot ".build\$run"
$root = Join-Path $work 'stage\EDUS-Library'
New-Item -ItemType Directory -Force -Path $root, $ArtifactDirectory | Out-Null
$env:TEMP = 'E:\Codex\temp'
$env:TMP = $env:TEMP
$env:CARGO_HOME = 'E:\Codex\cache\edus-library-tauri\cargo'
$env:RUSTUP_HOME = 'E:\Codex\cache\edus-library-tauri\rustup'
$env:CARGO_TARGET_DIR = $CargoTarget
$env:PATH = "$env:CARGO_HOME\bin;$env:PATH"
$utf8 = [Text.UTF8Encoding]::new($false)
function Write-Json($Value, [string]$Path) {
    [IO.File]::WriteAllText($Path, ($Value | ConvertTo-Json -Depth 30) + "`n", $utf8)
}
function Run-Check([string]$Name, [string]$Directory, [scriptblock]$Action) {
    Push-Location $Directory
    try {
        Write-Host "CHECK $Name"
        & $Action 2>&1 | Out-File -FilePath (Join-Path $work "$Name.log") -Encoding utf8
        if ($LASTEXITCODE -ne 0) { Get-Content (Join-Path $work "$Name.log") -Tail 25 | Write-Host; throw "$Name failed: $LASTEXITCODE" }
        Write-Host "PASS $Name (full log: $work\$Name.log)"
    } finally { Pop-Location }
}
function Revision([string]$Directory) {
    $head = & git -c "safe.directory=$($Directory.Replace('\','/'))" -C $Directory rev-parse HEAD
    if ($LASTEXITCODE -ne 0) { throw "Git revision unavailable: $Directory" }
    $status = & git -c "safe.directory=$($Directory.Replace('\','/'))" -C $Directory status --porcelain
    if ($LASTEXITCODE -ne 0) { throw "Git status unavailable: $Directory" }
    return @{ commit = "$head".Trim(); dirty = [bool]$status }
}
if ((Get-Content -Raw (Join-Path $Frontend 'package.json') | ConvertFrom-Json).version -ne $version) { throw 'Frontend version mismatch' }
foreach ($cargoFile in Get-ChildItem (Join-Path $Backend 'crates') -Filter Cargo.toml -Recurse) {
    if ((Get-Content -Raw $cargoFile.FullName) -notmatch ('(?m)^version\s*=\s*"' + [regex]::Escape($version) + '"')) { throw "Crate version mismatch: $cargoFile" }
}
Run-Check 'frontend-ci' $Frontend { npm.cmd ci --no-fund --no-audit }
foreach ($check in @('typecheck','test','lint','format:check','build')) {
    $scriptName = $check
    Run-Check ('frontend-' + $check.Replace(':','-')) $Frontend { npm.cmd run $scriptName }
}
$devCmd = 'E:\Codex\tools\VSBuildTools\Common7\Tools\VsDevCmd.bat'
foreach ($cargoCommand in @('fmt --all -- --check','clippy --workspace --all-targets --locked -- -D warnings','check --workspace --locked','test --workspace --locked','build --release --workspace --locked')) {
    $cargoShellCommand = 'call "' + $devCmd + '" -arch=x64 -host_arch=x64 >nul && cargo ' + $cargoCommand
    $name = ($cargoCommand -split ' ')[0]
    Run-Check "rust-$name" $Backend { cmd.exe /d /s /c $cargoShellCommand }
}
Run-Check 'deployment-tests' $PSScriptRoot { powershell.exe -NoProfile -ExecutionPolicy Bypass -File tests\Test-Deployment.ps1 }
foreach ($directory in @('backend','frontend','scripts','docs')) { New-Item -ItemType Directory -Path (Join-Path $root $directory) | Out-Null }
$executables = @('EDUSLibraryService.exe','EDUSTerminalConfigurator.exe','EDUSInstallerHelper.exe')
foreach ($exe in $executables) {
    $source = Join-Path $CargoTarget "release\$exe"
    if (!(Test-Path -LiteralPath $source -PathType Leaf)) { throw "Missing freshly built binary: $source" }
    Copy-Item -LiteralPath $source -Destination (Join-Path $root "backend\$exe")
}
$runtimeProvenance = foreach ($dll in @('vcruntime140.dll','vcruntime140_1.dll')) {
    $source = Join-Path $VcRuntime $dll
    $signature = Get-AuthenticodeSignature -LiteralPath $source
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'Microsoft Corporation') { throw "Unverified Microsoft runtime: $dll ($($signature.Status))" }
    Copy-Item -LiteralPath $source -Destination (Join-Path $root "backend\$dll")
    @{ file = $dll; version = (Get-Item $source).VersionInfo.FileVersion; sha256 = (Get-FileHash $source -Algorithm SHA256).Hash.ToLowerInvariant(); signer = $signature.SignerCertificate.Subject; source = 'Official Visual Studio VC Redist x64 directory'; signature = "$($signature.Status)" }
}
Copy-Item -LiteralPath (Join-Path $Backend 'migrations') -Destination (Join-Path $root 'backend\migrations') -Recurse
Copy-Item -Path (Join-Path $Frontend 'dist\*') -Destination (Join-Path $root 'frontend') -Recurse
Move-Item -LiteralPath (Join-Path $root 'frontend\frontend-manifest.json') -Destination (Join-Path $root 'frontend\manifest.json')
Copy-Item -Path (Join-Path $PSScriptRoot 'scripts\*.ps*1') -Destination (Join-Path $root 'scripts')
Copy-Item -Path (Join-Path $PSScriptRoot 'docs\*.md') -Destination (Join-Path $root 'docs')
Copy-Item -LiteralPath (Join-Path $Backend 'openapi\local-service-v1.openapi.json') -Destination (Join-Path $root 'docs')
Copy-Item -LiteralPath (Join-Path $Backend 'docs\VENDOR_SQLCIPHER.md') -Destination (Join-Path $root 'docs')
& (Join-Path $PSScriptRoot 'collect-notices.ps1') -Frontend $Frontend -Backend $Backend -Destination (Join-Path $root 'docs\THIRD_PARTY_NOTICES.md')
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'deployment.json') -Destination $root
Write-Json $runtimeProvenance (Join-Path $root 'docs\native-runtime-provenance.json')
$frontendManifest = Get-Content -Raw (Join-Path $root 'frontend\manifest.json') | ConvertFrom-Json
if ($frontendManifest.frontend_version -ne $version -or $frontendManifest.required_local_api_version -ne '1') { throw 'Frontend/API version mismatch' }
foreach ($file in $frontendManifest.files) {
    if ($file.path -match '(^/|\\|(^|/)\.\.(/|$)|:)') { throw 'Unsafe frontend manifest path' }
    if ((Get-FileHash -LiteralPath (Join-Path $root "frontend\$($file.path)") -Algorithm SHA256).Hash.ToLowerInvariant() -ne $file.sha256) { throw "Frontend hash mismatch: $($file.path)" }
}
$files = @(Get-ChildItem -LiteralPath $root -Recurse -File | ForEach-Object {
    if ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Reparse point in payload' }
    $relative = [IO.Path]::GetRelativePath($root, $_.FullName).Replace('\','/')
    if ($relative -match '(?i)(node_modules|(^|/)target/|\.db($|-)|\.dpapi$|\.map$|\.rs$|\.vue$|\.msi$|\.zip$)') { throw "Forbidden payload: $relative" }
    if ($_.Extension -eq '.exe' -and ($relative -notmatch '^backend/' -or $_.Name -notin $executables)) { throw "Unexpected executable: $relative" }
    @{ path = $relative; size = $_.Length; sha256 = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
} | Sort-Object { $_.path })
$manifest = [ordered]@{
    schema_version = 1; product = 'EDUS-Library-Portable'; package_version = $version
    frontend_version = $version; backend_version = $version; local_api_version = '1'
    build_time = (Get-Date).ToUniversalTime().ToString('o')
    revisions = @{ frontend = Revision $Frontend; backend = Revision $Backend; deployment = Revision $PSScriptRoot }
    signing = 'UNSIGNED RC/UAT; hashes detect corruption, not publisher authentication'
    files = $files
}
Write-Json $manifest (Join-Path $root 'manifest.json')
Import-Module (Join-Path $PSScriptRoot 'scripts\Edus.Deployment.psm1') -Force
# Runtime validator is also used before any privileged target installation.
Test-EdusPackage -Root $root | Out-Null
$zip = Join-Path $ArtifactDirectory 'EDUS-Library-Portable-2.0.0-rc1.zip'
$candidate = Join-Path $work 'EDUS-Library-Portable-2.0.0-rc1.zip'
[IO.Compression.ZipFile]::CreateFromDirectory((Split-Path $root -Parent), $candidate, [IO.Compression.CompressionLevel]::Optimal, $false)
$extracted = Join-Path $work 'extracted'
[IO.Compression.ZipFile]::ExtractToDirectory($candidate, $extracted)
Test-EdusPackage -Root (Join-Path $extracted 'EDUS-Library') | Out-Null
$archive = [IO.Compression.ZipFile]::OpenRead($candidate)
try {
    if (@($archive.Entries | Where-Object { !$_.FullName.StartsWith('EDUS-Library/') -or $_.FullName -match '(^|/)\.\.(/|$)' }).Count -ne 0) { throw 'Invalid ZIP root/path' }
    if ($archive.Entries.Count -ne $files.Count + 1) { throw 'ZIP entry count mismatch' }
} finally { $archive.Dispose() }
Copy-Item -LiteralPath $candidate -Destination $zip -Force
$artifact = Get-Item $zip
$result = [ordered]@{ path = $zip; version = $version; size = $artifact.Length; LastWriteTime = $artifact.LastWriteTime.ToString('o'); sha256 = (Get-FileHash $zip -Algorithm SHA256).Hash; files = $files.Count + 1; signing = 'UNSIGNED RC/UAT'; build_evidence = $work }
Write-Json $result (Join-Path $work 'artifact.json')
$result | ConvertTo-Json
