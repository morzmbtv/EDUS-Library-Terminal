# Backend release 2.0.0-rc.1

Build/test this independent Rust workspace with Cargo. Delivery assembly belongs to `E:\Codex\projects\edus-library-deployment\build-edus-portable.ps1` and produces one `EDUS-Library-Portable-2.0.0-rc1.zip`, not NSIS/MSI.

The package includes service, native Configurator, fixed-purpose deployment helper, migrations and official app-local VC runtime DLLs. Core/SQLCipher are statically linked. No Node/Rust toolchain or WebView2 is needed on the target. Microsoft Edge must already be installed. App-local Microsoft DLLs are serviced by future EDUS package updates; they are not automatically updated as a centrally installed VC redistributable.

Current scripts do not invoke the retired backend installer. The old source/package remains checkpointed. Repeated portable Setup/Repair preserves ProgramData and stable LocalService DPAPI context. Actual SCM/ACL clean/repeated setup acceptance requires an authorized test Windows machine.
