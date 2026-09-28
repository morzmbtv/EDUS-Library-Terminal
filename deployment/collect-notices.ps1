# Build-time tool only. Collect installed dependency license texts; never shipped as runtime code.
[CmdletBinding()]
param([string]$Frontend,[string]$Backend,[string]$Destination)
$ErrorActionPreference='Stop'
Push-Location $Backend
try {
    $raw=& cargo metadata --offline --locked --filter-platform x86_64-pc-windows-msvc --format-version 1
    if($LASTEXITCODE -ne 0){throw 'Cargo license metadata unavailable'}
    $metadata=$raw|ConvertFrom-Json
} finally {Pop-Location}
$builder=[Text.StringBuilder]::new()
$null=$builder.AppendLine('# Third-party notices').AppendLine().AppendLine('Collected from the locked Rust dependency graph and installed frontend build dependencies. This list may also include build/test tools that are not shipped. Microsoft VC Runtime distribution terms are described separately in NATIVE_RUNTIME.md.').AppendLine()
function Append-Notice([string]$Label,[string]$File) {
    $null=$builder.AppendLine('## '+$Label).AppendLine().AppendLine('```text').AppendLine([IO.File]::ReadAllText($File)).AppendLine('```').AppendLine()
}
foreach($package in $metadata.packages|Sort-Object name,version) {
    if(!$package.source -and $package.name -ne 'libsqlite3-sys'){continue}
    $directory=Split-Path $package.manifest_path
    foreach($file in Get-ChildItem $directory -File|Where-Object Name -Match '^(LICENSE|COPYING|NOTICE)([.-]|$)') {Append-Notice "$($package.name) $($package.version) — $($file.Name)" $file.FullName}
    if($package.name -eq 'openssl-src') {
        foreach($file in Get-ChildItem (Join-Path $directory 'openssl') -File|Where-Object Name -Match '^(LICENSE|NOTICE)([.-]|$)') {Append-Notice "OpenSSL native — $($file.Name)" $file.FullName}
    }
}
foreach($directory in Get-ChildItem (Join-Path $Frontend 'node_modules') -Directory) {
    $packages=if($directory.Name.StartsWith('@')){@(Get-ChildItem $directory.FullName -Directory)}else{@($directory)}
    foreach($package in $packages) {
        foreach($file in Get-ChildItem $package.FullName -File|Where-Object Name -Match '^(LICENSE|COPYING|NOTICE)([.-]|$)') {Append-Notice "npm $($package.Name) — $($file.Name)" $file.FullName}
    }
}
$sqlcipher=[IO.File]::ReadAllText((Join-Path $Backend 'vendor\libsqlite3-sys-0.38.2\sqlcipher\sqlite3.c'))
$notice=[regex]::Match($sqlcipher,'(?s)/\*\s*\*\* SQLCipher\s*\*\* http://zetetic.net.*?\*/')
if(!$notice.Success){throw 'SQLCipher redistribution notice missing'}
$null=$builder.AppendLine('## SQLCipher 4.18.0').AppendLine().AppendLine('```text').AppendLine($notice.Value).AppendLine('```')
[IO.File]::WriteAllText($Destination,$builder.ToString(),[Text.UTF8Encoding]::new($false))
