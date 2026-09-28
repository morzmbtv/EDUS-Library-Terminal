# Native runtime and distribution

EDUSLibraryService.exe, EDUSTerminalConfigurator.exe and EDUSInstallerHelper.exe are x64 release binaries. The core, SQLCipher 4.18.0, migrations used by the core and native crypto dependencies are linked into the Rust service. Migration SQL files are also included for release traceability. No Rust toolchain is required at runtime.

Official Microsoft Visual Studio VC Redist x64 `vcruntime140.dll` and `vcruntime140_1.dll` are deployed beside the executables. The build validates their Authenticode signatures and records source description, file versions, signer and SHA-256 in `native-runtime-provenance.json`. Windows supplies its normal system/UCRT libraries. No EXE/MSI redistributable runs during setup; Configurator is native Win32 and does not require WebView2.

Microsoft documents [application-local deployment](https://learn.microsoft.com/en-us/cpp/windows/redistributing-visual-cpp-files?view=msvc-170) subject to Visual Studio redistribution license terms. Microsoft recommends central deployment for servicing; with app-local deployment EDUS must deliver security updates for these DLLs in subsequent ZIP releases. Do not substitute DLLs downloaded from unofficial sites or copied from arbitrary system directories.

The EDUS binaries and PowerShell scripts are currently unsigned RC/UAT. Microsoft signatures on runtime DLLs do not sign EDUS. Package hashes detect corruption; obtain the package/hash from a trusted release channel. Production distribution requires the publisher's signing/redistribution review.
