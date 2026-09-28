# Native EDUS Terminal Configurator

This Windows desktop application uses Rust + Win32 controls. It contains no
Tauri, WebView2, JavaScript, Vue, SQLCipher or database access. The installer puts
it in the backend package. `build.rs` compiles an embedded
`requireAdministrator` manifest with Windows SDK `rc.exe`; Windows handles UAC.
Startup also verifies elevation and membership of the well-known built-in
Administrators SID. It never requests or sends a Windows password.

## Operator workflow

Select an action in the top dropdown, fill the labelled fields, and press
**Выполнить**. Requests run on a worker thread; the window stays responsive.
The result panel shows the service response or explicit failure. After a timeout,
inspect the service state before repeating a mutation: no automatic retry occurs.

1. Check **Служба и версия** and **Текущая рабочая область**.
2. Open isolated UAT, or enroll Production with Cloud URL, enrollment code and
   device name. School/device identifiers are returned by the service.
3. Create a fictional reader and a test book. Readers have unique external IDs;
   the service enforces type/status/class and duplicate rules.
4. Load **Список тестовых читателей и книг**, then choose card/scanner binding.
   Select the existing reader/copy, focus capture, scan and press **Выполнить**.
   Card input is masked and cleared after dispatch. Leading zeros remain intact;
   normalization/HMAC belong to the service.
5. CSV accepts the documented six-column UTF-8 content. A successful preview
   enables a separate apply button, which requires confirmation and submits the
   exact previewed content. Editing the field does not silently change that content.
6. Create/list encrypted backups through the service. Restore accepts only a
   filename returned by the backup list and requires two confirmations. The
   service permits UAT restore only and creates a recovery copy.
7. UAT reset requires two confirmations. Workspace changes clear cached lists.

Production restore and old Tauri migration remain explicit unavailable states;
this application never copies or opens a database file. Hardware status exposes
the service boundary; it is not a claim of physical camera/card-reader testing.

## IPC contract and tests

`edus-library-admin-ipc` owns the protocol types, pipe name, version, allowlist and
256 KiB frame limit. This client checks response request IDs, bounds both frames,
uses a 35-second timeout for ordinary commands and a bounded 180-second timeout
only for `CreateBackup`/`RestoreBackup`, and clears serialized request buffers.
The extended deadline accommodates two encrypted copies, each bounded by the
core at 60 seconds. Neither deadline automatically retries a mutation. The service
independently authenticates the caller. No HTTP admin endpoint is used.

Before writing even the length prefix, the client obtains the connected pipe's
server PID through `GetNamedPipeServerProcessId` and compares it with two typed
SCM `QueryServiceStatusEx` snapshots of the fixed `EDUSLibraryService` service.
Both snapshots must be RUNNING with the same nonzero PID. SCM handles are closed
on every return path. A same-name pipe owned by a different process, stopped or
starting service, missing registration, and a console-only prototype are rejected
before any enrollment/card payload is sent. This is intentional: production
Configurator acceptance requires the actual installed Windows service.

`cargo test -p edus-library-configurator` tests the transport library without
elevation. The production GUI binary is not used as the unit test runner because
its manifest deliberately requires UAC; this does not disable production checks.
Protocol tests cover forbidden commands, exact wire shape, response correlation,
invalid/missing result, bounded request size, and rejection of mismatched/zero/PENDING
pipe-server identities.

Native elevated UI and actual service/pipe acceptance must be executed in an
authorized Windows test environment. Passing IPC library tests is not a claim
that this physical/elevated workflow was tested.

The native window is 1080 × 750 pixels, with 32-pixel single-line fields and
40-pixel action buttons, and uses the Windows GUI font. Its requested bounds fit
a 1280 × 800 desktop at 100% scaling. Actual font readability, administrator
desktop DPI scaling and screen rendering have **not** been visually verified;
the current environment is not an authorized elevated GUI/installation test
machine. No screenshot or visual PASS is claimed for this native window.
